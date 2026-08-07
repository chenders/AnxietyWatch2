"""Bounded, immutable mirror for the untrusted ez Share HTTP interface."""

from __future__ import annotations

import argparse
import hashlib
import http.client
import json
import os
import re
import socket
import stat
import tempfile
import time
from dataclasses import asdict, dataclass
from html.parser import HTMLParser
from pathlib import Path, PurePosixPath
from typing import Callable
from urllib.parse import parse_qs, quote, urlencode, urljoin, urlsplit

DEFAULT_CONNECT_TIMEOUT = 3.0
DEFAULT_READ_TIMEOUT = 5.0
DEFAULT_MAX_LISTING_BYTES = 256 * 1024
DEFAULT_MAX_FILE_BYTES = 64 * 1024 * 1024
DEFAULT_MAX_FILES = 500
DEFAULT_MAX_DEPTH = 8
MANIFEST_VERSION = 1


class MirrorError(Exception):
    """A safe, operator-readable mirror failure."""


@dataclass(frozen=True)
class Limits:
    connect_timeout: float = DEFAULT_CONNECT_TIMEOUT
    read_timeout: float = DEFAULT_READ_TIMEOUT
    max_listing_bytes: int = DEFAULT_MAX_LISTING_BYTES
    max_file_bytes: int = DEFAULT_MAX_FILE_BYTES
    max_files: int = DEFAULT_MAX_FILES
    max_depth: int = DEFAULT_MAX_DEPTH

    def validate(self) -> None:
        for name in ("connect_timeout", "read_timeout"):
            value = getattr(self, name)
            if not isinstance(value, (int, float)) or not value < float("inf") or value <= 0:
                raise MirrorError(f"{name} rejected: {value!r}; must be finite and > 0")
        for name in ("max_listing_bytes", "max_file_bytes", "max_files"):
            value = getattr(self, name)
            if not isinstance(value, int) or value <= 0:
                raise MirrorError(f"{name} rejected: {value!r}; must be > 0")
        if not isinstance(self.max_depth, int) or self.max_depth < 0:
            raise MirrorError(f"max_depth rejected: {self.max_depth!r}; must be >= 0")


@dataclass
class Effects:
    discovered: int = 0
    downloaded: int = 0
    replayed: int = 0
    rejected: int = 0
    bytes: int = 0


@dataclass(frozen=True)
class ListingEntry:
    path: str
    is_dir: bool
    size: int | None = None


class _ListingParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.entries: list[tuple[str, str | None]] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag.lower() != "a":
            return
        values = dict(attrs)
        href = values.get("href")
        if href is not None:
            self.entries.append((href, values.get("data-size")))


def _validate_relative(value: str) -> str:
    if not isinstance(value, str) or not value:
        raise MirrorError(f"card path rejected: {value!r}; empty or non-text")
    if "\\" in value:
        raise MirrorError(f"card path rejected: {value!r}; backslash")
    if any(ord(ch) < 32 or ord(ch) == 127 for ch in value):
        raise MirrorError(f"card path rejected: {value!r}; NUL/control byte")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in ("", ".", "..") for part in path.parts):
        raise MirrorError(f"card path rejected: {value!r}; absolute or traversal component")
    normalized = path.as_posix()
    if normalized != value.strip("/") or value.startswith("/") or value.endswith("/"):
        raise MirrorError(f"card path rejected: {value!r}; not normalized POSIX relative path")
    return normalized


def _relative_card_value(raw: str, card_root: str) -> str:
    value = raw
    prefix = f"{card_root}/"
    if value.startswith(prefix):
        value = value[len(prefix):]
    return _validate_relative(value)


def parse_listing(data: bytes, card_root: str, directory: str = "") -> list[ListingEntry]:
    try:
        text = data.decode("utf-8", "strict")
    except UnicodeDecodeError as exc:
        raise MirrorError(f"listing rejected: invalid UTF-8 at byte {exc.start}") from exc
    parser = _ListingParser()
    try:
        parser.feed(text)
        parser.close()
    except Exception as exc:
        raise MirrorError(f"listing rejected: malformed HTML ({exc})") from exc
    result: list[ListingEntry] = []
    for href, size_text in parser.entries:
        if any(ord(ch) < 32 or ord(ch) == 127 for ch in href):
            raise MirrorError(f"listing href rejected: {href!r}; NUL/control byte")
        split = urlsplit(href)
        if split.scheme or split.netloc:
            raise MirrorError(f"listing href rejected: {href!r}; absolute URL")
        query = parse_qs(split.query, keep_blank_values=True, strict_parsing=False)
        if split.path.endswith("/dir") or split.path == "dir":
            values = query.get("dir", [])
            if len(values) == 1 and values[0] != card_root:
                value = _relative_card_value(values[0], card_root)
                if "/" not in value and directory:
                    value = _validate_relative(f"{directory}/{value}")
                result.append(ListingEntry(value, True))
        elif split.path.endswith("/download") or split.path == "download":
            values = query.get("file", [])
            if len(values) != 1:
                raise MirrorError(f"listing href rejected: {href!r}; one file parameter required")
            if size_text is None or re.fullmatch(r"[0-9]+", size_text) is None:
                raise MirrorError(f"listing file rejected: {values[0]!r}; missing decimal data-size")
            value = _relative_card_value(values[0], card_root)
            if "/" not in value and directory:
                value = _validate_relative(f"{directory}/{value}")
            result.append(ListingEntry(value, False, int(size_text)))
    return result


class _Response:
    def __init__(self, response: http.client.HTTPResponse, connection: http.client.HTTPConnection):
        self.response = response
        self.connection = connection

    def close(self) -> None:
        self.response.close()
        self.connection.close()


class HttpClient:
    def __init__(self, origin: str, limits: Limits):
        parsed = urlsplit(origin)
        if parsed.scheme != "http" or not parsed.hostname:
            raise MirrorError("origin rejected: fixed http:// host required")
        if parsed.username is not None or parsed.password is not None:
            raise MirrorError("origin rejected: userinfo is forbidden")
        if parsed.query or parsed.fragment:
            raise MirrorError("origin rejected: query/fragment is forbidden")
        if parsed.path not in ("", "/"):
            raise MirrorError("origin rejected: path must be empty")
        try:
            port = parsed.port or 80
        except ValueError as exc:
            raise MirrorError("origin rejected: invalid port") from exc
        self.host = parsed.hostname
        self.port = port
        self.origin = f"http://{self.host}:{self.port}"
        self.limits = limits

    def _open(self, target: str, redirects: int = 0) -> _Response:
        if redirects > 3:
            raise MirrorError(f"redirect rejected: limit 3 for {target!r}")
        split = urlsplit(target)
        port = split.port or 80
        if split.scheme != "http" or split.hostname != self.host or port != self.port or split.username is not None or split.password is not None:
            raise MirrorError(f"redirect origin rejected: expected {self.origin}")
        connection = http.client.HTTPConnection(self.host, self.port, timeout=self.limits.connect_timeout)
        request_target = split.path or "/"
        if split.query:
            request_target += "?" + split.query
        try:
            connection.request("GET", request_target, headers={"Accept": "text/html, application/octet-stream"})
            response = connection.getresponse()
            if connection.sock is not None:
                connection.sock.settimeout(self.limits.read_timeout)
        except (OSError, http.client.HTTPException) as exc:
            connection.close()
            raise MirrorError(f"HTTP request failed for {request_target!r}: {exc}") from exc
        if response.status in (301, 302, 303, 307, 308):
            location = response.getheader("Location")
            response.close()
            connection.close()
            if not location:
                raise MirrorError(f"redirect rejected for {request_target!r}: missing Location")
            return self._open(urljoin(target, location), redirects + 1)
        if response.status != 200:
            response.close()
            connection.close()
            raise MirrorError(f"HTTP status rejected for {request_target!r}: {response.status}")
        return _Response(response, connection)

    def open(self, path: str, params: dict[str, str]) -> _Response:
        query = urlencode(params, quote_via=quote, safe="")
        return self._open(f"{self.origin}{path}?{query}")


def _read_bounded(response: _Response, limit: int, label: str) -> bytes:
    declared = response.response.getheader("Content-Length")
    if declared is not None:
        try:
            declared_value = int(declared)
        except ValueError as exc:
            raise MirrorError(f"{label} rejected: invalid Content-Length {declared!r}") from exc
        if declared_value > limit:
            raise MirrorError(f"{label} rejected: Content-Length {declared_value} exceeds limit {limit}")
    try:
        data = response.response.read(limit + 1)
    except (OSError, http.client.HTTPException, socket.timeout) as exc:
        raise MirrorError(f"{label} read failed: {exc}") from exc
    if len(data) > limit:
        raise MirrorError(f"{label} rejected: bytes exceed limit {limit}")
    if declared is not None and len(data) != int(declared):
        raise MirrorError(f"{label} rejected: truncated response {len(data)} of {declared} bytes")
    return data


def _load_json(path: Path) -> dict | None:
    try:
        with path.open("r", encoding="utf-8") as handle:
            value = json.load(handle)
    except (FileNotFoundError, OSError, UnicodeError, json.JSONDecodeError):
        return None
    if not isinstance(value, dict) or value.get("version") != MANIFEST_VERSION:
        return None
    if not isinstance(value.get("generation"), int) or not isinstance(value.get("entries"), list):
        return None
    return value


def load_manifest(root: Path) -> dict:
    manifest_path = root / "manifest.json"
    pending_path = root / ".manifest.json.pending"
    candidates = [(path, value) for path in (manifest_path, pending_path) if (value := _load_json(path))]
    if not candidates:
        if manifest_path.exists() or pending_path.exists():
            raise MirrorError("manifest rejected: no valid versioned manifest or recovery candidate")
        return {"version": MANIFEST_VERSION, "generation": 0, "entries": []}
    chosen_path, chosen = max(candidates, key=lambda item: item[1]["generation"])
    if chosen_path == pending_path:
        os.replace(pending_path, manifest_path)
        _fsync_directory(root)
    elif pending_path.exists():
        pending_path.unlink()
    return chosen


def _fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def save_manifest(root: Path, manifest: dict) -> None:
    pending = root / ".manifest.json.pending"
    payload = (json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n").encode()
    descriptor = os.open(pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
    except Exception:
        try:
            pending.unlink()
        except FileNotFoundError:
            pass
        raise
    os.replace(pending, root / "manifest.json")
    _fsync_directory(root)


class Mirror:
    def __init__(self, origin: str, root: Path, limits: Limits = Limits(), *, now: Callable[[], int] | None = None, card_root: str = "A:"):
        limits.validate()
        if card_root != "A:":
            raise MirrorError(f"card root rejected: {card_root!r}; expected documented 'A:'")
        self.root = root
        self.limits = limits
        self.client = HttpClient(origin, limits)
        self.now = now or (lambda: int(time.time()))
        self.card_root = card_root

    def _listing(self, directory: str) -> list[ListingEntry]:
        card_directory = self.card_root if not directory else f"{self.card_root}/{directory}"
        response = self.client.open("/dir", {"dir": card_directory})
        try:
            data = _read_bounded(response, self.limits.max_listing_bytes, f"listing {directory or self.card_root!r}")
        finally:
            response.close()
        return parse_listing(data, self.card_root, directory)

    def _download(self, card_path: str, expected_size: int) -> dict:
        if expected_size > self.limits.max_file_bytes:
            raise MirrorError(f"file rejected: {card_path!r} size {expected_size} exceeds limit {self.limits.max_file_bytes}")
        content_dir = self.root / "content"
        content_dir.mkdir(mode=0o700, exist_ok=True)
        descriptor, temp_name = tempfile.mkstemp(prefix=".download-", dir=content_dir)
        temp_path = Path(temp_name)
        response: _Response | None = None
        digest = hashlib.sha256()
        length = 0
        try:
            response = self.client.open("/download", {"file": f"{self.card_root}/{card_path}"})
            declared = response.response.getheader("Content-Length")
            if declared is not None:
                try:
                    declared_value = int(declared)
                except ValueError as exc:
                    raise MirrorError(f"file rejected: {card_path!r} invalid Content-Length {declared!r}") from exc
                if declared_value > self.limits.max_file_bytes:
                    raise MirrorError(f"file rejected: {card_path!r} Content-Length {declared_value} exceeds limit {self.limits.max_file_bytes}")
            with os.fdopen(descriptor, "wb") as handle:
                while True:
                    try:
                        chunk = response.response.read(min(64 * 1024, self.limits.max_file_bytes - length + 1))
                    except (OSError, http.client.HTTPException, socket.timeout) as exc:
                        raise MirrorError(f"file read failed: {card_path!r}: {exc}") from exc
                    if not chunk:
                        break
                    length += len(chunk)
                    if length > self.limits.max_file_bytes:
                        raise MirrorError(f"file rejected: {card_path!r} bytes exceed limit {self.limits.max_file_bytes}")
                    digest.update(chunk)
                    handle.write(chunk)
                handle.flush()
                os.fsync(handle.fileno())
            if declared is not None and length != int(declared):
                raise MirrorError(f"file rejected: {card_path!r} truncated response {length} of {declared} bytes")
            if length != expected_size:
                raise MirrorError(f"file rejected: {card_path!r} listing size {expected_size} differs from bytes {length}")
            hex_digest = digest.hexdigest()
            final_path = content_dir / hex_digest
            if final_path.exists():
                existing_stat = final_path.stat()
                if not stat.S_ISREG(existing_stat.st_mode) or existing_stat.st_size != length:
                    raise MirrorError(f"content path rejected: {str(final_path)!r}; existing evidence mismatch")
                existing_digest = hashlib.sha256()
                with final_path.open("rb") as existing:
                    while chunk := existing.read(64 * 1024):
                        existing_digest.update(chunk)
                if existing_digest.hexdigest() != hex_digest:
                    raise MirrorError(f"content path rejected: {str(final_path)!r}; SHA-256 mismatch")
                temp_path.unlink()
            else:
                try:
                    os.link(temp_path, final_path)
                except FileExistsError:
                    pass
                temp_path.unlink()
                _fsync_directory(content_dir)
            return {"card_path": card_path, "content_path": f"content/{hex_digest}", "length": length, "sha256": hex_digest, "first_seen": self.now()}
        finally:
            if response is not None:
                response.close()
            try:
                os.close(descriptor)
            except OSError:
                pass
            try:
                temp_path.unlink()
            except FileNotFoundError:
                pass

    def run(self) -> Effects:
        self.root.mkdir(parents=True, exist_ok=True, mode=0o700)
        manifest = load_manifest(self.root)
        effects = Effects()
        known = {(entry["card_path"], entry["length"]): entry for entry in manifest["entries"]}
        stack: list[tuple[str, int]] = [("", 0)]
        seen_directories = {""}
        files: list[ListingEntry] = []
        while stack:
            directory, depth = stack.pop()
            for entry in self._listing(directory):
                effects.discovered += 1
                if entry.is_dir:
                    if entry.path in seen_directories:
                        continue
                    child_depth = len(PurePosixPath(entry.path).parts)
                    if child_depth > self.limits.max_depth:
                        raise MirrorError(f"directory rejected: {entry.path!r} depth {child_depth} exceeds limit {self.limits.max_depth}")
                    expected_parent = directory or "."
                    if PurePosixPath(entry.path).parent.as_posix() != expected_parent:
                        raise MirrorError(f"directory rejected: {entry.path!r}; not a direct child of {directory or self.card_root!r}")
                    seen_directories.add(entry.path)
                    stack.append((entry.path, child_depth))
                else:
                    expected_parent = directory or "."
                    if PurePosixPath(entry.path).parent.as_posix() != expected_parent:
                        raise MirrorError(f"file rejected: {entry.path!r}; not a direct child of {directory or self.card_root!r}")
                    files.append(entry)
                    if len(files) > self.limits.max_files:
                        raise MirrorError(f"file count rejected: {len(files)} exceeds limit {self.limits.max_files}")
        for entry in files:
            if entry.size is None:
                raise MirrorError(f"file rejected: {entry.path!r}; missing size")
            key = (entry.path, entry.size)
            if key in known:
                effects.replayed += 1
                continue
            new_entry = self._download(entry.path, entry.size)
            manifest["entries"].append(new_entry)
            known[key] = new_entry
            manifest["generation"] += 1
            save_manifest(self.root, manifest)
            effects.downloaded += 1
            effects.bytes += entry.size
        return effects


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Safely mirror an ez Share card over bounded plain HTTP")
    parser.add_argument("--origin", required=True, help="fixed card origin, e.g. http://192.168.4.1")
    parser.add_argument("--mirror-dir", required=True, type=Path, help="private directory for immutable evidence")
    parser.add_argument("--connect-timeout", type=float, default=DEFAULT_CONNECT_TIMEOUT, help=f"connect timeout seconds (default: {DEFAULT_CONNECT_TIMEOUT})")
    parser.add_argument("--read-timeout", type=float, default=DEFAULT_READ_TIMEOUT, help=f"read timeout seconds (default: {DEFAULT_READ_TIMEOUT})")
    parser.add_argument("--max-listing-bytes", type=int, default=DEFAULT_MAX_LISTING_BYTES, help=f"listing byte limit (default: {DEFAULT_MAX_LISTING_BYTES})")
    parser.add_argument("--max-file-bytes", type=int, default=DEFAULT_MAX_FILE_BYTES, help=f"individual file byte limit (default: {DEFAULT_MAX_FILE_BYTES})")
    parser.add_argument("--max-files", type=int, default=DEFAULT_MAX_FILES, help=f"files per run (default: {DEFAULT_MAX_FILES})")
    parser.add_argument("--max-depth", type=int, default=DEFAULT_MAX_DEPTH, help=f"directory depth (default: {DEFAULT_MAX_DEPTH})")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    limits = Limits(args.connect_timeout, args.read_timeout, args.max_listing_bytes, args.max_file_bytes, args.max_files, args.max_depth)
    try:
        effects = Mirror(args.origin, args.mirror_dir, limits).run()
    except MirrorError as exc:
        print(json.dumps({"error": str(exc), "discovered": 0, "downloaded": 0, "replayed": 0, "rejected": 1, "bytes": 0}), file=os.sys.stderr)
        return 1
    print(json.dumps(asdict(effects), sort_keys=True))
    return 0
