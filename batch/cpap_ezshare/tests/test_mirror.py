"""Hostile-fixture tests for the ez Share mirror.

Every test drives an in-process HTTP server that is free to behave like an
adversary. Timestamps are injected, never read from the clock, and the run
budget's clock is injectable so deadline behaviour is deterministic rather than
timing-dependent.

Several tests exist because an adversarial review reproduced the break first;
those carry a one-line note naming what was broken, so a later reader can tell
a regression test from a speculative one.
"""

from __future__ import annotations

import contextlib
import hashlib
import io
import json
import os
import stat
import tempfile
import threading
import time
import unittest
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

from batch.cpap_ezshare.mirror import (
    LOCK_NAME,
    Limits,
    Mirror,
    MirrorError,
    load_manifest,
    main,
    parse_listing,
)

FIXED_TIME = 1_786_017_600


class Fixture:
    def __init__(self) -> None:
        self.listings: dict[str, bytes] = {}
        self.files: dict[str, bytes] = {}
        self.downloads: dict[str, int] = {}
        self.listing_hits: dict[str, int] = {}
        self.redirect: str | None = None
        self.redirect_raw: bytes | None = None
        self.slow_listing = 0.0
        self.truncate: set[str] = set()
        self.raw_listings: dict[str, bytes] = {}
        # Body behaviour, for the timeout and encoding cases.
        self.body_stall = 0.0
        self.omit_content_length = False
        self.close_connection = False
        self.content_encoding: str | None = None
        # A generated wide tree: every listing offers `fanout` legal children.
        self.fanout = 0

    def listing(self, directory: str, links: list[tuple[str, bool, int | None]]) -> None:
        rows = ["<html><body><pre>"]
        for path, is_dir, size in links:
            if is_dir:
                rows.append(f'<a href="/dir?dir={path}">{path}</a>')
            else:
                rows.append(f'<a href="/download?file={path}" data-size="{size}">{path}</a>')
        rows.append("</pre></body></html>")
        self.listings[directory] = "\n".join(rows).encode()

    def wide_listing(self, directory: str) -> bytes:
        """A legal listing offering `fanout` subdirectories, generated on demand."""
        base = "" if directory == "A:" else directory[len("A:/"):]
        rows = ["<html><body><pre>"]
        for index in range(self.fanout):
            child = f"{base}/d{index}" if base else f"d{index}"
            rows.append(f'<a href="/dir?dir=A:/{child}">d{index}</a>')
        rows.append("</pre></body></html>")
        return "\n".join(rows).encode()


@contextmanager
def serve(fixture: Fixture):
    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def _listing_body(self, key: str) -> bytes | None:
            if key in fixture.raw_listings:
                return fixture.raw_listings[key]
            if fixture.fanout and (key == "A:" or key.startswith("A:/")):
                return fixture.wide_listing(key)
            return fixture.listings.get(key)

        def do_GET(self) -> None:
            split = urlsplit(self.path)
            query = parse_qs(split.query)
            if split.path == "/dir":
                if fixture.slow_listing:
                    time.sleep(fixture.slow_listing)
                key = query.get("dir", [""])[0]
                fixture.listing_hits[key] = fixture.listing_hits.get(key, 0) + 1
                body = self._listing_body(key)
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
                if fixture.redirect_raw is not None:
                    # Hand-rolled, so a non-ASCII Location can be sent at all.
                    self.wfile.write(
                        b"HTTP/1.1 302 Found\r\nLocation: "
                        + fixture.redirect_raw
                        + b"\r\nContent-Length: 0\r\n\r\n"
                    )
                    self.close_connection = True
                    return
                if fixture.redirect:
                    self.send_response(302)
                    self.send_header("Location", fixture.redirect)
                    self.send_header("Content-Length", "0")
                    self.end_headers()
                    return
                body = fixture.files.get(key)
                if body is None:
                    self.send_error(404)
                    return
                self.send_response(200)
                if fixture.content_encoding:
                    self.send_header("Content-Encoding", fixture.content_encoding)
                if not fixture.omit_content_length:
                    declared = len(body) + 5 if key in fixture.truncate else len(body)
                    self.send_header("Content-Length", str(declared))
                if fixture.close_connection:
                    self.send_header("Connection", "close")
                    self.close_connection = True
                self.end_headers()
                if fixture.body_stall:
                    # One byte, then stall well past the client's read timeout.
                    self.wfile.write(body[:1])
                    self.wfile.flush()
                    time.sleep(fixture.body_stall)
                    return
                self.wfile.write(body)
                if key in fixture.truncate:
                    self.close_connection = True
                return
            self.send_error(404)

        def log_message(self, format: str, *args: object) -> None:  # noqa: A002
            pass

        def handle_one_request(self) -> None:
            # Several tests make the client hang up mid-response on purpose —
            # that is the behaviour under test. Without this the server thread
            # prints a ConnectionResetError traceback per case and buries the
            # actual result in CI output.
            try:
                super().handle_one_request()
            except (ConnectionError, OSError):
                self.close_connection = True

        def handle_error(self, *args: object) -> None:
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


def standard_fixture() -> Fixture:
    fixture = Fixture()
    fixture.listing("A:", [("A:/DATALOG", True, None)])
    fixture.listing("A:/DATALOG", [("A:/DATALOG/night one.edf", False, 3)])
    fixture.files["A:/DATALOG/night one.edf"] = b"edf"
    return fixture


class _FakeClock:
    """A monotonic clock that only moves when a test moves it."""

    def __init__(self) -> None:
        self.value = 0.0

    def __call__(self) -> float:
        return self.value

    def advance(self, seconds: float) -> None:
        self.value += seconds


class MirrorTests(unittest.TestCase):
    def limits(self, **changes: object) -> Limits:
        values: dict[str, object] = {
            "connect_timeout": 1.0,
            "read_timeout": 1.0,
            "max_listing_bytes": 4096,
            "max_file_bytes": 4096,
            "max_files": 10,
            "max_depth": 4,
        }
        values.update(changes)
        return Limits(**values)  # type: ignore[arg-type]

    def run_mirror(self, fixture: Fixture, root: Path, **limit_changes: object):
        with serve(fixture) as origin:
            return Mirror(origin, root, self.limits(**limit_changes), now=lambda: FIXED_TIME).run()

    # ---------------------------------------------------------------- happy path

    def test_nested_discovery_space_encoding_and_manifest(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            effects = self.run_mirror(fixture, root)
            self.assertEqual((effects.discovered, effects.downloaded, effects.bytes), (2, 1, 3))
            self.assertEqual(effects.rejected, 0, effects.blockers)
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
            effects = Mirror(origin, Path(directory), now=lambda: FIXED_TIME).run()
            self.assertEqual(effects.replayed, 1)
            self.assertEqual(effects.downloaded, 0)
            self.assertEqual(fixture.downloads["A:/DATALOG/night one.edf"], 1)

    def test_changed_size_creates_new_immutable_content(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory, serve(fixture) as origin:
            root = Path(directory)
            Mirror(origin, root, now=lambda: FIXED_TIME).run()
            first = sorted(p.name for p in (root / "content").iterdir())
            fixture.files["A:/DATALOG/night one.edf"] = b"new-edf"
            fixture.listing("A:/DATALOG", [("A:/DATALOG/night one.edf", False, 7)])
            effects = Mirror(origin, root, now=lambda: FIXED_TIME).run()
            self.assertEqual(effects.downloaded, 1)
            manifest = json.loads((root / "manifest.json").read_text())
            self.assertEqual(len(manifest["entries"]), 2)
            after = sorted(p.name for p in (root / "content").iterdir())
            self.assertEqual(len(after), 2)
            # Old evidence is retained byte-for-byte, never replaced.
            self.assertTrue(set(first).issubset(set(after)))
            self.assertEqual((root / "content" / first[0]).read_bytes(), b"edf")

    # ------------------------------------------------------- names and traversal

    def test_traversal_absolute_backslash_and_control_names_rejected(self) -> None:
        bad_values = ["../secret", "/absolute", "bad\\name", "bad\x00name", "bad\nname"]
        for value in bad_values:
            with self.subTest(value=repr(value)):
                html = f'<a href="/download?file={value}" data-size="1">x</a>'.encode()
                with self.assertRaisesRegex(MirrorError, "rejected"):
                    parse_listing(html, "A:")

    def test_dot_is_rejected_as_a_card_path(self) -> None:
        """Review: `PurePosixPath('.').parts` is empty, so `.` passed the
        traversal check and was re-walked even at max_depth=0."""
        for href in ('/dir?dir=A:/.', '/download?file=A:/.'):
            with self.subTest(href=href):
                extra = ' data-size="3"' if "download" in href else ""
                html = f'<a href="{href}"{extra}>x</a>'.encode()
                with self.assertRaisesRegex(MirrorError, "names no component"):
                    parse_listing(html, "A:")

    def test_non_utf8_percent_encoding_is_rejected_not_mangled(self) -> None:
        """Review: parse_qs defaults to errors='replace', so a non-UTF-8 name
        became U+FFFD and the mirror requested a different resource than the
        card offered, recorded under a name the card does not have."""
        html = b'<a href="/download?file=A:/caf%e9.edf" data-size="3">x</a>'
        with self.assertRaisesRegex(MirrorError, "not valid UTF-8"):
            parse_listing(html, "A:")

    def test_unrecognized_anchors_are_reported_not_silently_skipped(self) -> None:
        """Review: a firmware change renaming the interface yielded
        discovered=0, exit 0, forever — indistinguishable from an idle card."""
        fixture = Fixture()
        fixture.raw_listings["A:"] = (
            b'<html><body><a href="/bogus?x=1">a</a><a href="/also-bogus">b</a></body></html>'
        )
        with tempfile.TemporaryDirectory() as directory:
            effects = self.run_mirror(fixture, Path(directory))
            self.assertEqual(effects.discovered, 0)
            self.assertEqual(effects.rejected, 1)
            self.assertRegex(effects.blockers[0], "matched no documented route")

    def test_uppercase_routes_are_recognized(self) -> None:
        html = b'<a href="/DOWNLOAD?file=A:/x.edf" data-size="3">x</a>'
        entries, unrecognized = parse_listing(html, "A:")
        self.assertEqual([(e.path, e.is_dir, e.size) for e in entries], [("x.edf", False, 3)])
        self.assertEqual(unrecognized, 0)

    # ------------------------------------------------------------ origin / SSRF

    def test_cross_origin_redirect_is_rejected_and_partial_removed(self) -> None:
        fixture = standard_fixture()
        fixture.redirect = "http://example.invalid/stolen"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            effects = self.run_mirror(fixture, root)
            self.assertEqual(effects.downloaded, 0)
            self.assertEqual(effects.rejected, 1)
            self.assertRegex(effects.blockers[0], "origin rejected")
            self.assertEqual(list((root / "content").iterdir()), [])

    def test_non_ascii_location_is_a_named_failure_not_a_traceback(self) -> None:
        """Review: a non-ASCII Location raised UnicodeEncodeError out of
        http.client, which is a ValueError and escaped the OSError handler."""
        fixture = standard_fixture()
        fixture.redirect_raw = "/dir?dir=caf\u00e9".encode("latin-1")
        with tempfile.TemporaryDirectory() as directory:
            effects = self.run_mirror(fixture, Path(directory))
            self.assertEqual(effects.rejected, 1)
            self.assertEqual(effects.downloaded, 0)

    def test_ipv6_origin_round_trips(self) -> None:
        """Review: the rebuilt origin dropped the brackets, so the first _open
        raised ValueError outside any handler."""
        with tempfile.TemporaryDirectory() as directory:
            mirror = Mirror("http://[::1]:8080", Path(directory))
            self.assertEqual(mirror.client.origin, "http://[::1]:8080")
            self.assertEqual(mirror.client.host, "::1")
            self.assertEqual(mirror.client.port, 8080)

    def test_port_zero_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "port 0"):
                Mirror("http://127.0.0.1:0", Path(directory))

    def test_content_encoding_is_rejected(self) -> None:
        """A gzipping card would otherwise have its compressed stream hashed and
        stored as the file's immutable evidence."""
        fixture = standard_fixture()
        fixture.content_encoding = "gzip"
        with tempfile.TemporaryDirectory() as directory:
            effects = self.run_mirror(fixture, Path(directory))
            self.assertEqual(effects.rejected, 1)
            self.assertRegex(effects.blockers[0], "Content-Encoding")

    def test_invalid_origin_and_limits_do_not_expose_password(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(MirrorError) as caught:
                Mirror("http://user:very-secret@127.0.0.1", Path(directory))
            self.assertNotIn("very-secret", str(caught.exception))
            with self.assertRaisesRegex(MirrorError, "connect_timeout rejected"):
                Mirror("http://127.0.0.1", Path(directory), Limits(connect_timeout=0))

    # ------------------------------------------------------------ bounds / time

    def test_oversized_listing_rejected(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "listing.*limit 10"):
                self.run_mirror(fixture, Path(directory), max_listing_bytes=10)

    def test_oversized_file_by_listing_and_stream(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            effects = self.run_mirror(fixture, Path(directory), max_file_bytes=2)
            self.assertEqual(effects.downloaded, 0)
            self.assertEqual(effects.rejected, 1)
            self.assertRegex(effects.blockers[0], "size 3 exceeds limit 2")
        fixture = standard_fixture()
        fixture.listing("A:/DATALOG", [("A:/DATALOG/night one.edf", False, 2)])
        with tempfile.TemporaryDirectory() as directory:
            effects = self.run_mirror(fixture, Path(directory), max_file_bytes=2)
            self.assertEqual(effects.rejected, 1)
            self.assertRegex(effects.blockers[0], "Content-Length 3 exceeds limit 2")

    def test_header_read_timeout_is_failure(self) -> None:
        fixture = standard_fixture()
        fixture.slow_listing = 5.0
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "HTTP request failed|read failed"):
                self.run_mirror(fixture, Path(directory), read_timeout=0.05, connect_timeout=0.05)

    def test_body_read_timeout_applies_when_the_card_closes_the_connection(self) -> None:
        """Review: getresponse() nulls connection.sock whenever the response
        will_close, so `if connection.sock is not None` skipped settimeout and
        the card chose which timeout governed its body by sending
        `Connection: close` — the one case with no Content-Length to bound it."""
        fixture = standard_fixture()
        fixture.close_connection = True
        fixture.omit_content_length = True
        fixture.body_stall = 5.0
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            started = time.monotonic()
            effects = self.run_mirror(
                fixture, root, read_timeout=0.1, connect_timeout=2.0
            )
            elapsed = time.monotonic() - started
            self.assertEqual(effects.downloaded, 0)
            self.assertEqual(effects.rejected, 1)
            self.assertRegex(effects.blockers[0], "read failed")
            # The point of the test: it gave up on the read timeout, not the
            # much larger connect timeout, and not the 5s stall.
            self.assertLess(elapsed, 3.0)
            self.assertEqual(list((root / "content").iterdir()), [])

    def test_run_deadline_is_enforced(self) -> None:
        """Per-read timeouts bound one recv, not the run. A card trickling bytes
        inside the per-read timeout held a single read open indefinitely."""
        fixture = standard_fixture()
        clock = _FakeClock()
        with tempfile.TemporaryDirectory() as directory, serve(fixture) as origin:
            mirror = Mirror(
                origin,
                Path(directory),
                self.limits(run_timeout=10.0),
                now=lambda: FIXED_TIME,
                clock=clock,
            )
            clock.advance(11.0)
            with self.assertRaisesRegex(MirrorError, "run deadline exceeded"):
                mirror.run()

    def test_wide_tree_is_bounded_by_the_directory_budget(self) -> None:
        """Review: seen_directories never dropped a path and nothing capped
        total listings. Ten legal children per listing at the depth limit is
        ~10^8 legal directories — measured at 141 MiB -> 1043 MiB RSS in 80s,
        and 16302 TCP connections in 10s against a real socket."""
        fixture = Fixture()
        fixture.fanout = 6
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "directory budget exhausted"):
                self.run_mirror(
                    fixture, Path(directory), max_depth=8, max_directories=12
                )
            # Bounded, not merely slow.
            self.assertLessEqual(sum(fixture.listing_hits.values()), 13)

    def test_request_budget_is_enforced(self) -> None:
        fixture = Fixture()
        fixture.fanout = 4
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "request budget exhausted"):
                self.run_mirror(
                    fixture, Path(directory), max_depth=8, max_directories=1000, max_requests=5
                )

    def test_total_byte_budget_is_enforced(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "total byte budget exhausted"):
                self.run_mirror(fixture, Path(directory), max_total_bytes=10)

    def test_free_space_floor_blocks_a_download(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            effects = self.run_mirror(fixture, root, min_free_bytes=2**60)
            self.assertEqual(effects.downloaded, 0)
            self.assertEqual(effects.rejected, 1)
            self.assertRegex(effects.blockers[0], "bytes free, needs")

    def test_depth_limit_prunes_and_keeps_mirroring(self) -> None:
        """Review: an over-depth entry aborted the whole run, so one hostile
        deep name denied mirroring for every other file on the card."""
        fixture = Fixture()
        fixture.listing(
            "A:", [("A:/DATALOG", True, None), ("A:/keep.edf", False, 4)]
        )
        fixture.listing("A:/DATALOG", [("A:/DATALOG/x.edf", False, 3)])
        fixture.files["A:/keep.edf"] = b"keep"
        with tempfile.TemporaryDirectory() as directory:
            effects = self.run_mirror(fixture, Path(directory), max_depth=0)
            self.assertEqual(effects.rejected, 1)
            self.assertRegex(effects.blockers[0], "depth 1 exceeds limit 0")
            # The sibling file was still mirrored.
            self.assertEqual(effects.downloaded, 1)

    def test_file_count_limit(self) -> None:
        fixture = Fixture()
        fixture.listing("A:", [("A:/one.edf", False, 1), ("A:/two.edf", False, 1)])
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(MirrorError, "more than 1 files"):
                self.run_mirror(fixture, Path(directory), max_files=1)

    def test_truncated_response_cleans_partial(self) -> None:
        fixture = standard_fixture()
        fixture.truncate.add("A:/DATALOG/night one.edf")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            effects = self.run_mirror(fixture, root)
            self.assertEqual(effects.downloaded, 0)
            self.assertEqual(effects.rejected, 1)
            self.assertRegex(effects.blockers[0], "truncated response|read failed")
            self.assertEqual(list((root / "content").iterdir()), [])
            self.assertFalse((root / "manifest.json").exists())

    # ------------------------------------------------- evidence and concurrency

    def test_dangling_symlink_at_content_path_is_rejected(self) -> None:
        """Review: `except FileExistsError: pass` accepted the link failure
        without verifying, then unlinked the downloaded bytes — exit 0,
        downloaded=6, manifest recording a path that was unreadable forever, and
        name+size matching so no later run ever re-fetched it."""
        fixture = standard_fixture()
        digest = hashlib.sha256(b"edf").hexdigest()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            content = root / "content"
            content.mkdir(parents=True, mode=0o700)
            os.symlink(str(root / "nowhere"), str(content / digest))
            effects = self.run_mirror(fixture, root)
            self.assertEqual(effects.downloaded, 0)
            self.assertEqual(effects.rejected, 1)
            self.assertRegex(effects.blockers[0], "not a regular file|cannot stat|cannot read")
            self.assertFalse((root / "manifest.json").exists())

    def test_existing_content_with_wrong_bytes_is_rejected(self) -> None:
        fixture = standard_fixture()
        digest = hashlib.sha256(b"edf").hexdigest()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            content = root / "content"
            content.mkdir(parents=True, mode=0o700)
            (content / digest).write_bytes(b"XXX")
            effects = self.run_mirror(fixture, root)
            self.assertEqual(effects.rejected, 1)
            self.assertRegex(effects.blockers[0], "SHA-256 mismatch")
            # Pre-existing evidence is never overwritten, even when wrong.
            self.assertEqual((content / digest).read_bytes(), b"XXX")

    def test_a_second_concurrent_run_is_blocked_by_the_lock(self) -> None:
        """Review: two concurrent runs crashed on the fixed pending-manifest
        name and took a lost update — both loaded generation 0, both appended,
        and one run's recorded evidence vanished while the generation did not
        advance, so nothing downstream could detect the loss."""
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            root.mkdir(parents=True, exist_ok=True)
            (root / LOCK_NAME).write_text("99999\n")
            with self.assertRaisesRegex(MirrorError, f"{LOCK_NAME} is held"):
                self.run_mirror(fixture, root)

    def test_lock_is_released_after_a_successful_run(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.run_mirror(fixture, root)
            self.assertFalse((root / LOCK_NAME).exists())

    def test_mirror_directory_mode_is_tightened(self) -> None:
        """Review: mkdir(mode=..., exist_ok=True) does not tighten an existing
        directory, so a 0777 mirror root stayed 0777 while the README called it
        private. This store holds PHI."""
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "mirror"
            root.mkdir(mode=0o777)
            os.chmod(root, 0o777)
            self.run_mirror(fixture, root)
            self.assertEqual(stat.S_IMODE(root.stat().st_mode), 0o700)
            self.assertEqual(stat.S_IMODE((root / "content").stat().st_mode), 0o700)

    def test_manifest_recovery_after_interrupted_replacement(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            digest = hashlib.sha256(b"x").hexdigest()
            entry = {
                "card_path": "x.edf",
                "content_path": f"content/{digest}",
                "first_seen": FIXED_TIME,
                "length": 1,
                "sha256": digest,
            }
            old = {"version": 1, "generation": 1, "entries": []}
            new = {"version": 1, "generation": 2, "entries": [entry]}
            (root / "manifest.json").write_text(json.dumps(old))
            (root / ".manifest.json.pending").write_text(json.dumps(new))
            recovered = load_manifest(root)
            self.assertEqual(recovered, new)
            self.assertEqual(json.loads((root / "manifest.json").read_text()), new)
            self.assertFalse((root / ".manifest.json.pending").exists())

    def test_malformed_manifest_entry_names_its_blocker(self) -> None:
        """Review: run() indexes card_path and length directly, so a manifest
        whose entries lacked them escaped as a KeyError traceback."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "manifest.json").write_text(
                json.dumps({"version": 1, "generation": 1, "entries": [{"card_path": "x"}]})
            )
            with self.assertRaisesRegex(MirrorError, "entry 0 is malformed"):
                load_manifest(root)

    def test_verify_replayed_detects_mutation_under_unchanged_name_and_size(self) -> None:
        """Requirement 5's two clauses are in tension: skipping a download on
        unchanged name+size cannot detect changed bytes. --verify-replayed is
        the opt-in resolution; see README."""
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory, serve(fixture) as origin:
            root = Path(directory)
            Mirror(origin, root, self.limits(), now=lambda: FIXED_TIME).run()
            # Same name, same size, different bytes.
            fixture.files["A:/DATALOG/night one.edf"] = b"zzz"
            quiet = Mirror(origin, root, self.limits(), now=lambda: FIXED_TIME).run()
            self.assertEqual((quiet.downloaded, quiet.replayed, quiet.rejected), (0, 1, 0))
            loud = Mirror(
                origin, root, self.limits(), now=lambda: FIXED_TIME, verify_replayed=True
            ).run()
            self.assertEqual(loud.downloaded, 1)
            self.assertEqual(loud.rejected, 1)
            self.assertRegex(loud.blockers[0], "content changed under an unchanged name and size")
            # Both evidences retained.
            self.assertEqual(len(list((root / "content").iterdir())), 2)

    # ----------------------------------------------------------------- main/CLI

    def test_main_reports_measured_effects_after_a_partial_failure(self) -> None:
        """Review: run()'s real effects were discarded when it raised, so a run
        that had committed four files reported downloaded=0 — measured work
        presented as nothing happened."""
        fixture = Fixture()
        fixture.listing(
            "A:", [("A:/a.edf", False, 4), ("A:/b.edf", False, 4)]
        )
        fixture.files["A:/a.edf"] = b"aaaa"
        fixture.files["A:/b.edf"] = b"bbbb"
        with tempfile.TemporaryDirectory() as directory, serve(fixture) as origin:
            err = io.StringIO()
            with contextlib.redirect_stderr(err):
                # One listing plus one download fits in the request budget; the
                # second download's charge is what ends the run.
                code = main([
                    "--origin", origin,
                    "--mirror-dir", directory,
                    "--max-requests", "2",
                    "--max-listing-bytes", "4096",
                ])
            self.assertEqual(code, 1)
            payload = json.loads(err.getvalue())
            self.assertIn("error", payload)
            self.assertRegex(payload["error"], "request budget exhausted")
            # The point: a real count, not a fabricated zero.
            self.assertEqual(payload["downloaded"], 1)
            self.assertEqual(payload["bytes"], 4)

    def test_main_exits_nonzero_when_anything_was_rejected(self) -> None:
        fixture = standard_fixture()
        fixture.content_encoding = "gzip"
        with tempfile.TemporaryDirectory() as directory, serve(fixture) as origin:
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = main(["--origin", origin, "--mirror-dir", directory])
            self.assertEqual(code, 1)
            payload = json.loads(out.getvalue())
            self.assertEqual(payload["rejected"], 1)
            self.assertEqual(payload["downloaded"], 0)

    def test_main_exits_zero_on_a_clean_run(self) -> None:
        fixture = standard_fixture()
        with tempfile.TemporaryDirectory() as directory, serve(fixture) as origin:
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = main(["--origin", origin, "--mirror-dir", directory])
            self.assertEqual(code, 0)
            payload = json.loads(out.getvalue())
            self.assertEqual(payload["downloaded"], 1)
            self.assertEqual(payload["rejected"], 0)
            self.assertEqual(payload["blockers"], [])


if __name__ == "__main__":
    unittest.main()
