from __future__ import annotations

import hashlib
import json
import socket
import tempfile
import threading
import time
import unittest
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

from batch.cpap_ezshare.mirror import Limits, Mirror, MirrorError, load_manifest, parse_listing

FIXED_TIME = 1_786_017_600


class Fixture:
    def __init__(self) -> None:
        self.listings: dict[str, bytes] = {}
        self.files: dict[str, bytes] = {}
        self.downloads: dict[str, int] = {}
        self.redirect: str | None = None
        self.slow_listing = 0.0
        self.truncate: set[str] = set()

    def listing(self, directory: str, links: list[tuple[str, bool, int | None]]) -> None:
        rows = ["<html><body><pre>"]
        for path, is_dir, size in links:
            if is_dir:
                rows.append(f'<a href="/dir?dir={path}">{path}</a>')
            else:
                rows.append(f'<a href="/download?file={path}" data-size="{size}">{path}</a>')
        rows.append("</pre></body></html>")
        self.listings[directory] = "\n".join(rows).encode()


@contextmanager
def serve(fixture: Fixture):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            split = urlsplit(self.path)
            query = parse_qs(split.query)
            if split.path == "/dir":
                if fixture.slow_listing:
                    time.sleep(fixture.slow_listing)
                key = query.get("dir", [""])[0]
                body = fixture.listings.get(key)
                if body is None:
                    self.send_error(404)
                    return
                self.send_response(200)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
                return
            if split.path == "/download":
                key = query.get("file", [""])[0]
                fixture.downloads[key] = fixture.downloads.get(key, 0) + 1
                if fixture.redirect:
                    self.send_response(302)
                    self.send_header("Location", fixture.redirect)
                    self.end_headers()
                    return
                body = fixture.files.get(key)
                if body is None:
                    self.send_error(404)
                    return
                self.send_response(200)
                declared = len(body) + 5 if key in fixture.truncate else len(body)
                self.send_header("Content-Length", str(declared))
                self.end_headers()
                self.wfile.write(body)
                if key in fixture.truncate:
                    self.close_connection = True
                return
            self.send_error(404)

        def log_message(self, format: str, *args: object) -> None:
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


def standard_fixture() -> Fixture:
    fixture = Fixture()
    fixture.listing("A:", [("A:/DATALOG", True, None)])
    fixture.listing("A:/DATALOG", [("A:/DATALOG/night one.edf", False, 3)])
    fixture.files["A:/DATALOG/night one.edf"] = b"edf"
    return fixture


class MirrorTests(unittest.TestCase):
    def run_mirror(self, fixture: Fixture, root: Path, **limit_changes: object):
        values = {
            "connect_timeout": 1.0,
            "read_timeout": 1.0,
            "max_listing_bytes": 4096,
            "max_file_bytes": 4096,
            "max_files": 10,
            "max_depth": 4,
        }
        values.update(limit_changes)
        with serve(fixture) as origin:
            return Mirror(origin, root, Limits(**values), now=lambda: FIXED_TIME).run()

    def test_nested_discovery_space_encoding_and_manifest(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            effects = self.run_mirror(fixture, root)
            self.assertEqual((effects.discovered, effects.downloaded, effects.bytes), (2, 1, 3))
            key = "A:/DATALOG/night one.edf"
            self.assertEqual(fixture.downloads[key], 1)
            digest = hashlib.sha256(b"edf").hexdigest()
            self.assertEqual((root / "content" / digest).read_bytes(), b"edf")
            manifest = json.loads((root / "manifest.json").read_text())
            self.assertEqual(manifest["version"], 1)
            self.assertEqual(manifest["entries"], [{
                "card_path": "DATALOG/night one.edf",
                "content_path": f"content/{digest}",
                "first_seen": FIXED_TIME,
                "length": 3,
                "sha256": digest,
            }])

    def test_replay_unchanged_name_and_size_does_not_download(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory, serve(fixture) as origin:
            mirror = Mirror(origin, Path(directory), now=lambda: FIXED_TIME)
            mirror.run()
            effects = mirror.run()
            self.assertEqual(effects.replayed, 1)
            self.assertEqual(effects.downloaded, 0)
            self.assertEqual(fixture.downloads["A:/DATALOG/night one.edf"], 1)

    def test_changed_size_creates_new_immutable_content(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory, serve(fixture) as origin:
            root = Path(directory)
            mirror = Mirror(origin, root, now=lambda: FIXED_TIME)
            mirror.run()
            fixture.files["A:/DATALOG/night one.edf"] = b"new-edf"
            fixture.listing("A:/DATALOG", [("A:/DATALOG/night one.edf", False, 7)])
            effects = mirror.run()
            self.assertEqual(effects.downloaded, 1)
            manifest = json.loads((root / "manifest.json").read_text())
            self.assertEqual(len(manifest["entries"]), 2)
            self.assertEqual(len(list((root / "content").iterdir())), 2)

    def test_traversal_absolute_backslash_and_control_names_rejected(self) -> None:
        bad_values = ["../secret", "/absolute", "bad\\name", "bad\x00name", "bad\nname"]
        for value in bad_values:
            with self.subTest(value=repr(value)):
                html = f'<a href="/download?file={value}" data-size="1">x</a>'.encode()
                with self.assertRaisesRegex(MirrorError, "rejected"):
                    parse_listing(html, "A:")

    def test_cross_origin_redirect_is_rejected_and_partial_removed(self) -> None:
        fixture = standard_fixture()
        fixture.redirect = "http://example.invalid/stolen"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(MirrorError, "origin rejected"):
                self.run_mirror(fixture, root)
            content = root / "content"
            self.assertEqual(list(content.iterdir()), [])

    def test_oversized_listing_rejected(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "listing.*limit 10"):
                self.run_mirror(fixture, Path(directory), max_listing_bytes=10)

    def test_oversized_file_by_listing_and_stream(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "size 3 exceeds limit 2"):
                self.run_mirror(fixture, Path(directory), max_file_bytes=2)
        fixture = standard_fixture()
        fixture.listing("A:/DATALOG", [("A:/DATALOG/night one.edf", False, 2)])
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "Content-Length 3 exceeds limit 2"):
                self.run_mirror(fixture, Path(directory), max_file_bytes=2)

    def test_read_timeout_is_failure(self) -> None:
        fixture = standard_fixture()
        fixture.slow_listing = 0.2
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "HTTP request failed"):
                self.run_mirror(fixture, Path(directory), read_timeout=0.03, connect_timeout=0.03)

    def test_depth_limit(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "depth 1 exceeds limit 0"):
                self.run_mirror(fixture, Path(directory), max_depth=0)

    def test_file_count_limit(self) -> None:
        fixture = Fixture()
        fixture.listing("A:", [("A:/one.edf", False, 1), ("A:/two.edf", False, 1)])
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "2 exceeds limit 1"):
                self.run_mirror(fixture, Path(directory), max_files=1)

    def test_truncated_response_cleans_partial(self) -> None:
        fixture = standard_fixture()
        fixture.truncate.add("A:/DATALOG/night one.edf")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(MirrorError, "truncated response"):
                self.run_mirror(fixture, root)
            self.assertEqual(list((root / "content").iterdir()), [])
            self.assertFalse((root / "manifest.json").exists())

    def test_manifest_recovery_after_interrupted_replacement(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            old = {"version": 1, "generation": 1, "entries": []}
            new = {"version": 1, "generation": 2, "entries": [{"card_path": "x"}]}
            (root / "manifest.json").write_text(json.dumps(old))
            (root / ".manifest.json.pending").write_text(json.dumps(new))
            recovered = load_manifest(root)
            self.assertEqual(recovered, new)
            self.assertEqual(json.loads((root / "manifest.json").read_text()), new)
            self.assertFalse((root / ".manifest.json.pending").exists())

    def test_invalid_origin_and_limits_do_not_expose_password(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(MirrorError) as caught:
                Mirror("http://user:very-secret@127.0.0.1", Path(directory))
            self.assertNotIn("very-secret", str(caught.exception))
            with self.assertRaisesRegex(MirrorError, "connect_timeout rejected"):
                Mirror("http://127.0.0.1", Path(directory), Limits(connect_timeout=0))


if __name__ == "__main__":
    unittest.main()
