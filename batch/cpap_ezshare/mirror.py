"""Bounded, immutable mirror for the untrusted ez Share HTTP interface.

The card is hostile. It serves unauthenticated plain HTTP, and every listing
byte and filename is attacker-controlled. This module therefore assumes the
card, or anything able to impersonate it on the card's subnet, is an adversary
whose goals are: escape the mirror directory, reach a host other than the card,
exhaust memory, disk, sockets or wall-clock time, or make a failed run look
like a successful one.

Two design notes that carry most of the safety:

* **No card-supplied string ever becomes a filesystem path.** Every write goes
  to a `tempfile.mkstemp` name or to `content/<sha256>` — 64 lowercase hex
  characters. The whole encoded/normalised/unicode traversal family is
  therefore inert by construction rather than by validation. Card paths are
  still validated, but only so the *evidence record* is meaningful.
* **Individual bounds are not run bounds.** Capping one file, one listing and
  the file count still permits an unbounded walk: a card serving ten legal
  subdirectories per listing at the depth limit yields ~10^8 legal
  directories. So the run itself is bounded — total requests, total
  directories, total bytes, a wall-clock deadline, and free-space headroom —
  and every one of those is a named blocker when it fires.
"""

from __future__ import annotations

import argparse
import errno
import hashlib
import http.client
import json
import os
import re
import stat
import sys
import tempfile
import time
from dataclasses import asdict, dataclass, field
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

# Run-level bounds. Without these the per-object limits above are satisfiable
# by a walk that never terminates in any practical sense.
#
# max_total_bytes exists because max_files * max_file_bytes is 31.25 GiB, which
# is not a conservative default on a Pi that also runs the alarm. 2 GiB is far
# more than a night of AirSense EDF and still leaves the card unable to fill a
# disk in one run. Content is immutable and never pruned, so the free-space
# floor is checked per download as well.
DEFAULT_MAX_DIRECTORIES = 512
DEFAULT_MAX_REQUESTS = 2048
DEFAULT_MAX_TOTAL_BYTES = 2 * 1024 * 1024 * 1024
DEFAULT_RUN_TIMEOUT = 900.0
DEFAULT_MIN_FREE_BYTES = 1024 * 1024 * 1024

MANIFEST_VERSION = 1
LOCK_NAME = ".mirror.lock"
PENDING_NAME = ".manifest.json.pending"
MANIFEST_NAME = "manifest.json"
_HEX64 = re.compile(r"\A[0-9a-f]{64}\Z")


class MirrorError(Exception):
    """A safe, operator-readable mirror failure.

    Carries the effects measured before the failure. Reporting zeroes after a
    partial run would present measured work as "nothing happened", which is the
    unknown-rendered-as-known failure the project forbids.
    """

    def __init__(self, message: str, effects: "Effects | None" = None):
        super().__init__(message)
        self.effects = effects


class BudgetExhausted(MirrorError):
    """A run-level budget or deadline was spent.

    Its own type rather than a substring of the message: the per-file handler
    has to distinguish "this one file is bad, keep mirroring the rest" from
    "the run is over", and deciding that by searching the message for the word
    "budget" would silently change behaviour the first time someone rephrases
    an error.
    """


def _redact_target(target: str) -> str:
    """Render a request target without echoing query *values*.

    The ez Share convention passes its password as a query parameter, so a
    future task may put a credential here. Error text must name the rejected
    value and the reason, and a query key is enough for that; the value never
    is. Keys are kept, values become `=…`.
    """
    split = urlsplit(target)
    if not split.query:
        return split.path or "/"
    keys = []
    for pair in split.query.split("&"):
        key = pair.split("=", 1)[0]
        keys.append(f"{key}=…" if key else "…")
    return f"{split.path or '/'}?{'&'.join(keys)}"


@dataclass(frozen=True)
class Limits:
    connect_timeout: float = DEFAULT_CONNECT_TIMEOUT
    read_timeout: float = DEFAULT_READ_TIMEOUT
    max_listing_bytes: int = DEFAULT_MAX_LISTING_BYTES
    max_file_bytes: int = DEFAULT_MAX_FILE_BYTES
    max_files: int = DEFAULT_MAX_FILES
    max_depth: int = DEFAULT_MAX_DEPTH
    max_directories: int = DEFAULT_MAX_DIRECTORIES
    max_requests: int = DEFAULT_MAX_REQUESTS
    max_total_bytes: int = DEFAULT_MAX_TOTAL_BYTES
    run_timeout: float = DEFAULT_RUN_TIMEOUT
    min_free_bytes: int = DEFAULT_MIN_FREE_BYTES

    def validate(self) -> None:
        for name in ("connect_timeout", "read_timeout", "run_timeout"):
            value = getattr(self, name)
            if not isinstance(value, (int, float)) or not value < float("inf") or value <= 0:
                raise MirrorError(f"{name} rejected: {value!r}; must be finite and > 0")
        for name in (
            "max_listing_bytes",
            "max_file_bytes",
            "max_files",
            "max_directories",
            "max_requests",
            "max_total_bytes",
        ):
            value = getattr(self, name)
            if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
                raise MirrorError(f"{name} rejected: {value!r}; must be > 0")
        if not isinstance(self.min_free_bytes, int) or self.min_free_bytes < 0:
            raise MirrorError(f"min_free_bytes rejected: {self.min_free_bytes!r}; must be >= 0")
        if not isinstance(self.max_depth, int) or isinstance(self.max_depth, bool) or self.max_depth < 0:
            raise MirrorError(f"max_depth rejected: {self.max_depth!r}; must be >= 0")


@dataclass
class Effects:
    discovered: int = 0
    downloaded: int = 0
    replayed: int = 0
    rejected: int = 0
    bytes: int = 0
    # Every rejection names itself here. A run that rejected something and said
    # only "rejected: 3" would fail the project's rule that a guard names its
    # blocker and the value that blocked it.
    blockers: list[str] = field(default_factory=list)

    def reject(self, reason: str) -> None:
        self.rejected += 1
        if len(self.blockers) < 64:
            self.blockers.append(reason)
        elif len(self.blockers) == 64:
            self.blockers.append("… further blockers suppressed; see rejected count")


@dataclass(frozen=True)
class ListingEntry:
    path: str
    is_dir: bool
    size: int | None = None


class _Budget:
    """Run-level request, byte and time budget.

    Separate from Limits because it is mutable state for one run, and because
    every charge point must be able to name which budget it exhausted.
    """

    def __init__(self, limits: Limits, clock: Callable[[], float]):
        self.limits = limits
        self.clock = clock
        self.started = clock()
        self.requests = 0
        self.bytes = 0
        self.directories = 0

    def remaining_seconds(self) -> float:
        return self.limits.run_timeout - (self.clock() - self.started)

    def charge_request(self, label: str) -> None:
        if self.remaining_seconds() <= 0:
            raise BudgetExhausted(
                f"run deadline exceeded before {label}: limit {self.limits.run_timeout}s"
            )
        self.requests += 1
        if self.requests > self.limits.max_requests:
            raise BudgetExhausted(
                f"request budget exhausted at {label}: {self.requests} exceeds limit "
                f"{self.limits.max_requests}"
            )

    def charge_directory(self, path: str) -> None:
        self.directories += 1
        if self.directories > self.limits.max_directories:
            raise BudgetExhausted(
                f"directory budget exhausted at {path!r}: {self.directories} exceeds limit "
                f"{self.limits.max_directories}"
            )

    def charge_bytes(self, count: int, label: str) -> None:
        self.bytes += count
        if self.bytes > self.limits.max_total_bytes:
            raise BudgetExhausted(
                f"total byte budget exhausted at {label}: {self.bytes} exceeds limit "
                f"{self.limits.max_total_bytes}"
            )

    def read_timeout(self) -> float:
        """Per-read timeout, clamped by whatever remains of the run deadline.

        A card that trickles one byte just inside the per-read timeout can hold
        a single read open indefinitely: the socket timeout is per-recv, not
        per-body. Clamping each read by the remaining run time converts that
        from unbounded to bounded.
        """
        remaining = self.remaining_seconds()
        if remaining <= 0:
            raise BudgetExhausted(f"run deadline exceeded: limit {self.limits.run_timeout}s")
        return min(self.limits.read_timeout, remaining)


class _ListingParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.entries: list[tuple[str, str | None]] = []
        self.anchors = 0

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag.lower() != "a":
            return
        self.anchors += 1
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
    # `PurePosixPath(".").parts` is the empty tuple, so an `any(...)` over parts
    # never sees the dot and "." passed the traversal check. It has no
    # filesystem effect — content is addressed by hash — but it re-walks the
    # current directory even at depth 0 and writes a meaningless card_path into
    # the evidence record.
    if not path.parts:
        raise MirrorError(f"card path rejected: {value!r}; names no component")
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


def _query(split_query: str, href: str) -> dict[str, list[str]]:
    """Percent-decode a query strictly.

    `parse_qs` defaults to `errors="replace"`, which silently turns a non-UTF-8
    filename into U+FFFD. The mirror would then request a *different* resource
    than the card offered and record it under a name the card does not have.
    """
    try:
        return parse_qs(
            split_query,
            keep_blank_values=True,
            strict_parsing=False,
            encoding="utf-8",
            errors="strict",
        )
    except UnicodeDecodeError as exc:
        raise MirrorError(
            f"listing href rejected: {href!r}; query is not valid UTF-8 at byte {exc.start}"
        ) from exc


def parse_listing(
    data: bytes, card_root: str, directory: str = ""
) -> tuple[list[ListingEntry], int]:
    """Parse one listing. Returns (entries, unrecognized_anchor_count).

    The unrecognized count is returned rather than discarded so a firmware
    change that renames the interface shows up as a named blocker instead of an
    indefinite run of "discovered 0, exit 0", which is indistinguishable from
    an idle card.
    """
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
    recognized = 0
    for href, size_text in parser.entries:
        if any(ord(ch) < 32 or ord(ch) == 127 for ch in href):
            raise MirrorError(f"listing href rejected: {href!r}; NUL/control byte")
        split = urlsplit(href)
        if split.scheme or split.netloc:
            raise MirrorError(f"listing href rejected: {href!r}; absolute URL")
        # Case-insensitive: the tag match is already case-insensitive, so a card
        # serving /DOWNLOAD would otherwise be silently skipped.
        route = (split.path or "").rsplit("/", 1)[-1].lower()
        query = _query(split.query, href)
        if route == "dir":
            values = query.get("dir", [])
            if len(values) != 1:
                raise MirrorError(
                    f"listing href rejected: {href!r}; exactly one dir parameter required, got "
                    f"{len(values)}"
                )
            recognized += 1
            if values[0] == card_root:
                continue
            value = _relative_card_value(values[0], card_root)
            if "/" not in value and directory:
                value = _validate_relative(f"{directory}/{value}")
            result.append(ListingEntry(value, True))
        elif route == "download":
            values = query.get("file", [])
            if len(values) != 1:
                raise MirrorError(f"listing href rejected: {href!r}; one file parameter required")
            if size_text is None or re.fullmatch(r"[0-9]+", size_text) is None:
                raise MirrorError(
                    f"listing file rejected: {values[0]!r}; missing decimal data-size"
                )
            recognized += 1
            value = _relative_card_value(values[0], card_root)
            if "/" not in value and directory:
                value = _validate_relative(f"{directory}/{value}")
            result.append(ListingEntry(value, False, int(size_text)))
    return result, parser.anchors - recognized


class _Response:
    def __init__(
        self,
        response: http.client.HTTPResponse,
        connection: http.client.HTTPConnection,
        sock,
    ):
        self.response = response
        self.connection = connection
        # Held separately: http.client drops `connection.sock` to None whenever
        # the response `will_close`, which a card controls with a single
        # `Connection: close` header. The response keeps reading through a file
        # object over this same socket, so the timeout has to be set on the
        # object we captured, not on `connection.sock`.
        self.sock = sock

    def set_timeout(self, seconds: float) -> None:
        if self.sock is not None:
            try:
                self.sock.settimeout(seconds)
            except OSError:
                pass

    def close(self) -> None:
        self.response.close()
        self.connection.close()


class HttpClient:
    def __init__(self, origin: str, limits: Limits, budget: _Budget):
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
            port = parsed.port
        except ValueError as exc:
            raise MirrorError("origin rejected: invalid port") from exc
        if port == 0:
            raise MirrorError("origin rejected: port 0")
        self.host = parsed.hostname
        self.port = port or 80
        # An IPv6 literal must keep its brackets, or re-parsing the rebuilt
        # origin raises ValueError outside any handler.
        display_host = f"[{self.host}]" if ":" in self.host else self.host
        self.origin = f"http://{display_host}:{self.port}"
        self.limits = limits
        self.budget = budget

    def _open(self, target: str, redirects: int = 0) -> _Response:
        if redirects > 3:
            raise MirrorError(f"redirect rejected: limit 3 for {_redact_target(target)!r}")
        try:
            split = urlsplit(target)
            port = split.port or 80
        except ValueError as exc:
            raise MirrorError(f"redirect rejected: unparseable target ({exc})") from exc
        if (
            split.scheme != "http"
            or split.hostname != self.host
            or port != self.port
            or split.username is not None
            or split.password is not None
        ):
            raise MirrorError(f"redirect origin rejected: expected {self.origin}")
        safe_target = _redact_target(target)
        self.budget.charge_request(repr(safe_target))
        connection = http.client.HTTPConnection(
            self.host, self.port, timeout=self.limits.connect_timeout
        )
        request_target = split.path or "/"
        if split.query:
            request_target += "?" + split.query
        sock = None
        try:
            connection.request(
                "GET", request_target, headers={"Accept": "text/html, application/octet-stream"}
            )
            # Capture and arm the socket BEFORE getresponse(). getresponse()
            # calls connection.close() when the response will_close, after
            # which connection.sock is None and the read timeout would never be
            # applied — leaving the card's body governed by the connect
            # timeout, which it selects by sending `Connection: close`.
            sock = connection.sock
            if sock is not None:
                sock.settimeout(self.budget.read_timeout())
            response = connection.getresponse()
        except (OSError, http.client.HTTPException, ValueError, UnicodeError) as exc:
            # ValueError/UnicodeError: a non-ASCII or malformed Location reaches
            # http.client's request encoder, which raises neither OSError nor
            # HTTPException. Card-controlled, so it must not escape as a
            # traceback.
            connection.close()
            raise MirrorError(f"HTTP request failed for {safe_target!r}: {exc}") from exc
        encoding = response.getheader("Content-Encoding")
        if encoding:
            response.close()
            connection.close()
            raise MirrorError(
                f"response rejected for {safe_target!r}: Content-Encoding {encoding!r} is not "
                "accepted; stored evidence must be the file's own bytes"
            )
        if response.status in (301, 302, 303, 307, 308):
            location = response.getheader("Location")
            response.close()
            connection.close()
            if not location:
                raise MirrorError(f"redirect rejected for {safe_target!r}: missing Location")
            try:
                resolved = urljoin(target, location)
            except ValueError as exc:
                raise MirrorError(
                    f"redirect rejected for {safe_target!r}: unresolvable Location ({exc})"
                ) from exc
            return self._open(resolved, redirects + 1)
        if response.status != 200:
            response.close()
            connection.close()
            raise MirrorError(f"HTTP status rejected for {safe_target!r}: {response.status}")
        return _Response(response, connection, sock)

    def open(self, path: str, params: dict[str, str]) -> _Response:
        query = urlencode(params, quote_via=quote, safe="")
        return self._open(f"{self.origin}{path}?{query}")


def _read_bounded(response: _Response, limit: int, label: str, budget: _Budget) -> bytes:
    declared = response.response.getheader("Content-Length")
    if declared is not None:
        try:
            declared_value = int(declared)
        except ValueError as exc:
            raise MirrorError(f"{label} rejected: invalid Content-Length {declared!r}") from exc
        if declared_value > limit:
            raise MirrorError(
                f"{label} rejected: Content-Length {declared_value} exceeds limit {limit}"
            )
    chunks: list[bytes] = []
    total = 0
    while total <= limit:
        response.set_timeout(budget.read_timeout())
        try:
            chunk = response.response.read(min(64 * 1024, limit + 1 - total))
        except (OSError, http.client.HTTPException) as exc:
            raise MirrorError(f"{label} read failed: {exc}") from exc
        if not chunk:
            break
        total += len(chunk)
        chunks.append(chunk)
    data = b"".join(chunks)
    budget.charge_bytes(len(data), label)
    if len(data) > limit:
        raise MirrorError(f"{label} rejected: bytes exceed limit {limit}")
    if declared is not None and len(data) != int(declared):
        raise MirrorError(f"{label} rejected: truncated response {len(data)} of {declared} bytes")
    return data


def _valid_entry(entry: object) -> bool:
    if not isinstance(entry, dict):
        return False
    if not isinstance(entry.get("card_path"), str) or not entry["card_path"]:
        return False
    if not isinstance(entry.get("length"), int) or isinstance(entry["length"], bool):
        return False
    if entry["length"] < 0:
        return False
    if not isinstance(entry.get("sha256"), str) or not _HEX64.match(entry["sha256"]):
        return False
    if not isinstance(entry.get("content_path"), str):
        return False
    return entry["content_path"] == f"content/{entry['sha256']}"


def _load_json(path: Path) -> tuple[dict | None, str | None]:
    """Return (manifest, reason-it-was-rejected). One of the two is None."""
    try:
        with path.open("r", encoding="utf-8") as handle:
            value = json.load(handle)
    except FileNotFoundError:
        return None, None
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        return None, f"{path.name}: unreadable or not JSON ({exc})"
    if not isinstance(value, dict):
        return None, f"{path.name}: top level is {type(value).__name__}, not an object"
    if value.get("version") != MANIFEST_VERSION:
        return None, f"{path.name}: version {value.get('version')!r} != {MANIFEST_VERSION}"
    if not isinstance(value.get("generation"), int) or isinstance(value.get("generation"), bool):
        return None, f"{path.name}: generation is not an integer"
    entries = value.get("entries")
    if not isinstance(entries, list):
        return None, f"{path.name}: entries is not a list"
    # Validated per entry, because run() indexes card_path and length directly;
    # a malformed entry otherwise escapes as a KeyError traceback.
    for index, entry in enumerate(entries):
        if not _valid_entry(entry):
            return None, f"{path.name}: entry {index} is malformed"
    return value, None


def load_manifest(root: Path) -> dict:
    manifest_path = root / MANIFEST_NAME
    pending_path = root / PENDING_NAME
    reasons: list[str] = []
    candidates: list[tuple[Path, dict]] = []
    for path in (manifest_path, pending_path):
        value, reason = _load_json(path)
        if value is not None:
            candidates.append((path, value))
        elif reason is not None:
            reasons.append(reason)
    if not candidates:
        if reasons:
            raise MirrorError("manifest rejected: " + "; ".join(reasons))
        return {"version": MANIFEST_VERSION, "generation": 0, "entries": []}
    chosen_path, chosen = max(candidates, key=lambda item: item[1]["generation"])
    if chosen_path == pending_path:
        os.replace(pending_path, manifest_path)
        _fsync_directory(root)
    elif pending_path.exists():
        # Safe to discard: the run lock guarantees no other process owns it.
        pending_path.unlink()
    return chosen


def _fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _private_directory(path: Path) -> None:
    """Create the directory if absent and enforce 0700 if it already existed.

    `mkdir(mode=..., exist_ok=True)` does not tighten an existing directory, so
    a mirror root created by hand at 0755 stayed world-readable while the README
    called it private. This store holds PHI.
    """
    path.mkdir(parents=True, exist_ok=True, mode=0o700)
    try:
        current = stat.S_IMODE(path.stat().st_mode)
        if current != 0o700:
            os.chmod(path, 0o700)
    except OSError as exc:
        raise MirrorError(f"mirror directory rejected: {str(path)!r}; cannot secure mode ({exc})")


def save_manifest(root: Path, manifest: dict) -> None:
    pending = root / PENDING_NAME
    payload = (json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n").encode()
    try:
        descriptor = os.open(pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise MirrorError(
            f"manifest write blocked: {PENDING_NAME} already exists; a previous run left it "
            "behind or another run holds it"
        ) from exc
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
    os.replace(pending, root / MANIFEST_NAME)
    _fsync_directory(root)


class _RunLock:
    """Single-instance guard for one mirror directory.

    Two concurrent runs both crashed on the fixed pending-manifest name and,
    worse, took a lost update: both loaded generation 0, both appended, and one
    run's recorded evidence vanished from the manifest while the generation did
    not advance — so nothing downstream could detect the loss.
    """

    def __init__(self, root: Path):
        self.path = root / LOCK_NAME
        self.descriptor: int | None = None

    def __enter__(self) -> "_RunLock":
        try:
            self.descriptor = os.open(self.path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        except FileExistsError as exc:
            raise MirrorError(
                f"run blocked: {LOCK_NAME} is held in {str(self.path.parent)!r}; another mirror "
                "run is in progress, or a previous run was killed and the file must be removed "
                "by the operator"
            ) from exc
        os.write(self.descriptor, f"{os.getpid()}\n".encode())
        return self

    def __exit__(self, *exc_info: object) -> None:
        if self.descriptor is not None:
            try:
                os.close(self.descriptor)
            except OSError:
                pass
        try:
            self.path.unlink()
        except FileNotFoundError:
            pass


class Mirror:
    def __init__(
        self,
        origin: str,
        root: Path,
        limits: Limits = Limits(),
        *,
        now: Callable[[], int] | None = None,
        clock: Callable[[], float] | None = None,
        card_root: str = "A:",
        verify_replayed: bool = False,
    ):
        limits.validate()
        if card_root != "A:":
            raise MirrorError(f"card root rejected: {card_root!r}; expected documented 'A:'")
        self.root = root
        self.limits = limits
        self.now = now or (lambda: int(time.time()))
        self.budget = _Budget(limits, clock or time.monotonic)
        self.client = HttpClient(origin, limits, self.budget)
        self.card_root = card_root
        self.verify_replayed = verify_replayed

    def _listing(self, directory: str, effects: Effects) -> list[ListingEntry]:
        card_directory = self.card_root if not directory else f"{self.card_root}/{directory}"
        label = f"listing {directory or self.card_root!r}"
        response = self.client.open("/dir", {"dir": card_directory})
        try:
            data = _read_bounded(response, self.limits.max_listing_bytes, label, self.budget)
        finally:
            response.close()
        entries, unrecognized = parse_listing(data, self.card_root, directory)
        if unrecognized:
            effects.reject(
                f"{label}: {unrecognized} anchor(s) matched no documented route; the card "
                "interface may have changed and files may be going unseen"
            )
        return entries

    def _free_bytes(self) -> int | None:
        try:
            info = os.statvfs(self.root)
        except (OSError, AttributeError):
            return None
        return info.f_bavail * info.f_frsize

    def _check_headroom(self, need: int, card_path: str) -> None:
        free = self._free_bytes()
        if free is None:
            return
        if free - need < self.limits.min_free_bytes:
            raise MirrorError(
                f"download blocked for {card_path!r}: {free} bytes free, needs {need} and must "
                f"leave {self.limits.min_free_bytes}"
            )

    def _accept_existing(self, final_path: Path, length: int, hex_digest: str) -> None:
        """Accept pre-existing content only after proving it is the same bytes.

        Used by both the pre-check and the link-EEXIST race. Skipping the proof
        on the race path let a dangling symlink planted at `content/<sha256>`
        swallow the download: the run exited 0 with `downloaded=6`, the manifest
        recorded the content path, the bytes were unlinked, and the path was
        unreadable forever — and because name+size then matched, no later run
        ever re-fetched it.
        """
        try:
            existing_stat = final_path.lstat()
        except OSError as exc:
            raise MirrorError(
                f"content path rejected: {str(final_path)!r}; cannot stat existing evidence ({exc})"
            ) from exc
        if not stat.S_ISREG(existing_stat.st_mode):
            raise MirrorError(
                f"content path rejected: {str(final_path)!r}; existing evidence is not a regular "
                f"file (mode {stat.S_IFMT(existing_stat.st_mode):#o})"
            )
        if existing_stat.st_size != length:
            raise MirrorError(
                f"content path rejected: {str(final_path)!r}; existing evidence is "
                f"{existing_stat.st_size} bytes, expected {length}"
            )
        existing_digest = hashlib.sha256()
        try:
            with final_path.open("rb") as existing:
                while chunk := existing.read(64 * 1024):
                    existing_digest.update(chunk)
        except OSError as exc:
            raise MirrorError(
                f"content path rejected: {str(final_path)!r}; cannot read existing evidence ({exc})"
            ) from exc
        if existing_digest.hexdigest() != hex_digest:
            raise MirrorError(f"content path rejected: {str(final_path)!r}; SHA-256 mismatch")

    def _download(self, card_path: str, expected_size: int) -> dict:
        if expected_size > self.limits.max_file_bytes:
            raise MirrorError(
                f"file rejected: {card_path!r} size {expected_size} exceeds limit "
                f"{self.limits.max_file_bytes}"
            )
        content_dir = self.root / "content"
        _private_directory(content_dir)
        self._check_headroom(expected_size, card_path)
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
                    raise MirrorError(
                        f"file rejected: {card_path!r} invalid Content-Length {declared!r}"
                    ) from exc
                if declared_value > self.limits.max_file_bytes:
                    raise MirrorError(
                        f"file rejected: {card_path!r} Content-Length {declared_value} exceeds "
                        f"limit {self.limits.max_file_bytes}"
                    )
            with os.fdopen(descriptor, "wb") as handle:
                # fdopen now owns the descriptor; the finally must not close it
                # a second time. The previous double-close landed on a free
                # number only by luck, and this function recycles numbers.
                descriptor = -1
                while True:
                    response.set_timeout(self.budget.read_timeout())
                    try:
                        chunk = response.response.read(
                            min(64 * 1024, self.limits.max_file_bytes - length + 1)
                        )
                    except (OSError, http.client.HTTPException) as exc:
                        raise MirrorError(f"file read failed: {card_path!r}: {exc}") from exc
                    if not chunk:
                        break
                    length += len(chunk)
                    if length > self.limits.max_file_bytes:
                        raise MirrorError(
                            f"file rejected: {card_path!r} bytes exceed limit "
                            f"{self.limits.max_file_bytes}"
                        )
                    digest.update(chunk)
                    try:
                        handle.write(chunk)
                    except OSError as exc:
                        if exc.errno == errno.ENOSPC:
                            raise MirrorError(
                                f"download blocked for {card_path!r}: no space left on device "
                                f"after {length} bytes"
                            ) from exc
                        raise MirrorError(
                            f"download blocked for {card_path!r}: write failed after {length} "
                            f"bytes ({exc})"
                        ) from exc
                handle.flush()
                os.fsync(handle.fileno())
            self.budget.charge_bytes(length, f"file {card_path!r}")
            if declared is not None and length != int(declared):
                raise MirrorError(
                    f"file rejected: {card_path!r} truncated response {length} of {declared} bytes"
                )
            if length != expected_size:
                raise MirrorError(
                    f"file rejected: {card_path!r} listing size {expected_size} differs from "
                    f"bytes {length}"
                )
            hex_digest = digest.hexdigest()
            final_path = content_dir / hex_digest
            if os.path.lexists(final_path):
                self._accept_existing(final_path, length, hex_digest)
            else:
                try:
                    os.link(temp_path, final_path)
                except FileExistsError:
                    # Raced with another writer. Same proof as the pre-check —
                    # never an unverified accept.
                    self._accept_existing(final_path, length, hex_digest)
                _fsync_directory(content_dir)
            return {
                "card_path": card_path,
                "content_path": f"content/{hex_digest}",
                "length": length,
                "sha256": hex_digest,
                "first_seen": self.now(),
            }
        finally:
            if response is not None:
                response.close()
            if descriptor != -1:
                try:
                    os.close(descriptor)
                except OSError:
                    pass
            try:
                temp_path.unlink()
            except FileNotFoundError:
                pass

    def run(self) -> Effects:
        effects = Effects()
        try:
            _private_directory(self.root)
            with _RunLock(self.root):
                self._run_locked(effects)
        except MirrorError as exc:
            # Re-raise carrying what was actually measured, so the caller never
            # has to invent zeroes.
            raise MirrorError(str(exc), effects) from exc
        return effects

    def _run_locked(self, effects: Effects) -> None:
        manifest = load_manifest(self.root)
        known = {(entry["card_path"], entry["length"]): entry for entry in manifest["entries"]}
        stack: list[tuple[str, int]] = [("", 0)]
        seen_directories = {""}
        files: list[ListingEntry] = []
        while stack:
            directory, _ = stack.pop()
            self.budget.charge_directory(directory or self.card_root)
            for entry in self._listing(directory, effects):
                effects.discovered += 1
                expected_parent = directory or "."
                if PurePosixPath(entry.path).parent.as_posix() != expected_parent:
                    effects.reject(
                        f"{'directory' if entry.is_dir else 'file'} rejected: {entry.path!r}; not "
                        f"a direct child of {directory or self.card_root!r}"
                    )
                    continue
                if entry.is_dir:
                    if entry.path in seen_directories:
                        continue
                    child_depth = len(PurePosixPath(entry.path).parts)
                    if child_depth > self.limits.max_depth:
                        # Prune, do not abort. Aborting let one hostile deep
                        # name deny all mirroring for every other file on the
                        # card, which is a denial of service the card chooses.
                        effects.reject(
                            f"directory pruned: {entry.path!r} depth {child_depth} exceeds limit "
                            f"{self.limits.max_depth}"
                        )
                        continue
                    seen_directories.add(entry.path)
                    stack.append((entry.path, child_depth))
                else:
                    if entry.size is None:
                        effects.reject(f"file rejected: {entry.path!r}; missing size")
                        continue
                    if len(files) >= self.limits.max_files:
                        raise MirrorError(
                            f"file count rejected: more than {self.limits.max_files} files "
                            f"discovered; stopped at {entry.path!r}"
                        )
                    files.append(entry)
        for entry in files:
            assert entry.size is not None
            key = (entry.path, entry.size)
            if key in known and not self.verify_replayed:
                effects.replayed += 1
                continue
            try:
                new_entry = self._download(entry.path, entry.size)
            except BudgetExhausted:
                raise
            except MirrorError as exc:
                # One bad file is a rejection, not the end of the run: the other
                # files on the card are still real evidence worth mirroring.
                effects.reject(str(exc))
                continue
            if key in known:
                existing = known[key]
                if existing["sha256"] == new_entry["sha256"]:
                    effects.replayed += 1
                    continue
                effects.reject(
                    f"content changed under an unchanged name and size: {entry.path!r} "
                    f"({entry.size} bytes) was {existing['sha256'][:12]}… and is now "
                    f"{new_entry['sha256'][:12]}…; both are retained"
                )
            manifest["entries"].append(new_entry)
            known[key] = new_entry
            manifest["generation"] += 1
            save_manifest(self.root, manifest)
            effects.downloaded += 1
            effects.bytes += entry.size


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Safely mirror an ez Share card over bounded plain HTTP"
    )
    parser.add_argument(
        "--origin",
        required=True,
        help=(
            "fixed card origin, e.g. http://CARD-IP — the card's access-point address, "
            "recorded in docs/integration/hms-cpap-ezshare-integration.md"
        ),
    )
    parser.add_argument(
        "--mirror-dir", required=True, type=Path, help="private directory for immutable evidence"
    )
    parser.add_argument(
        "--connect-timeout",
        type=float,
        default=DEFAULT_CONNECT_TIMEOUT,
        help=f"connect timeout seconds (default: {DEFAULT_CONNECT_TIMEOUT})",
    )
    parser.add_argument(
        "--read-timeout",
        type=float,
        default=DEFAULT_READ_TIMEOUT,
        help=f"per-read timeout seconds (default: {DEFAULT_READ_TIMEOUT})",
    )
    parser.add_argument(
        "--run-timeout",
        type=float,
        default=DEFAULT_RUN_TIMEOUT,
        help=f"total wall-clock budget for one run, seconds (default: {DEFAULT_RUN_TIMEOUT})",
    )
    parser.add_argument(
        "--max-listing-bytes",
        type=int,
        default=DEFAULT_MAX_LISTING_BYTES,
        help=f"listing byte limit (default: {DEFAULT_MAX_LISTING_BYTES})",
    )
    parser.add_argument(
        "--max-file-bytes",
        type=int,
        default=DEFAULT_MAX_FILE_BYTES,
        help=f"individual file byte limit (default: {DEFAULT_MAX_FILE_BYTES})",
    )
    parser.add_argument(
        "--max-files",
        type=int,
        default=DEFAULT_MAX_FILES,
        help=f"files per run (default: {DEFAULT_MAX_FILES})",
    )
    parser.add_argument(
        "--max-depth",
        type=int,
        default=DEFAULT_MAX_DEPTH,
        help=f"directory depth (default: {DEFAULT_MAX_DEPTH})",
    )
    parser.add_argument(
        "--max-directories",
        type=int,
        default=DEFAULT_MAX_DIRECTORIES,
        help=f"directories listed per run (default: {DEFAULT_MAX_DIRECTORIES})",
    )
    parser.add_argument(
        "--max-requests",
        type=int,
        default=DEFAULT_MAX_REQUESTS,
        help=f"HTTP requests per run (default: {DEFAULT_MAX_REQUESTS})",
    )
    parser.add_argument(
        "--max-total-bytes",
        type=int,
        default=DEFAULT_MAX_TOTAL_BYTES,
        help=f"total bytes read per run (default: {DEFAULT_MAX_TOTAL_BYTES})",
    )
    parser.add_argument(
        "--min-free-bytes",
        type=int,
        default=DEFAULT_MIN_FREE_BYTES,
        help=f"free space that must remain after a download (default: {DEFAULT_MIN_FREE_BYTES})",
    )
    parser.add_argument(
        "--verify-replayed",
        action="store_true",
        help=(
            "re-fetch files already recorded under the same name and size and compare hashes. "
            "Off by default because it re-downloads the whole card; see README for the residual "
            "risk it addresses."
        ),
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    limits = Limits(
        args.connect_timeout,
        args.read_timeout,
        args.max_listing_bytes,
        args.max_file_bytes,
        args.max_files,
        args.max_depth,
        args.max_directories,
        args.max_requests,
        args.max_total_bytes,
        args.run_timeout,
        args.min_free_bytes,
    )
    try:
        effects = Mirror(
            args.origin, args.mirror_dir, limits, verify_replayed=args.verify_replayed
        ).run()
    except MirrorError as exc:
        # The measured effects, never fabricated zeroes: a run that committed
        # four files and then rejected a fifth must not report downloaded=0.
        payload = asdict(exc.effects) if exc.effects is not None else {}
        payload["error"] = str(exc)
        print(json.dumps(payload, sort_keys=True), file=sys.stderr)
        return 1
    print(json.dumps(asdict(effects), sort_keys=True))
    # A rejection is a nonzero exit even when the run otherwise completed: the
    # task requires a nonzero process exit for listing, download or validation
    # failure, and a partially-mirrored card is not a success.
    return 1 if effects.rejected else 0
