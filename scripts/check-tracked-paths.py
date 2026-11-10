#!/usr/bin/env python3
"""Reject tracked files that belong on a workstation, not in the repository.

scripts/check-public-info.py judges a file by what its text *says*. It cannot
see a file it cannot decode, and it skips those silently. Every accident this
gate was written for was invisible to it for that reason, and every one was
recognisable from its *path* alone:

- 33 screenshots and a screen recording committed at the repository root,
  including Health and HealthKit screens;
- a 5.7 MB whole-repo ZIP snapshot packed as context for a web chat review,
  which re-bundled those same screenshots plus .idea/ and 1,877 fuzz inputs, so
  deleting the screenshots alone did not remove them;
- two agent kickoff prompts whose own text said "deliberately untracked --
  delete once consumed";
- a PDF export sitting beside the Markdown it was exported from.

The idea is borrowed from a sibling project's no-PII gate, which blocks by path
before reading content. That gate also *skips* archives and images when
scanning content, so it has the same blind spot; here they are rejected
instead.

Every rule is an ALLOWLIST, following scripts/tests/test-tracked-filenames.sh:
a denylist must anticipate every hazard, while an allowlist fails closed on
anything nobody thought of. Widen a list deliberately, in the same commit as
the file that needs it.

Binary files are recognised two independent ways, because each alone has a
hole: by content (git's own `i/-text` classification of the committed blob)
and by extension (a media, document or archive suffix). An all-ASCII PDF or a
Git LFS pointer named `*.png` is text by content, and a binary with no
extension has no suffix; either signal places the file under the binary
location allowlist.

Runs in CI only (the `public-info` job), alongside the #104 filename gate. It
reads the index, which in a CI checkout is exactly the commit under test.

Calibration, measured on 872 tracked paths after the cleanup that introduced
this gate: 4 binary files (all PNG, in the two locations below), 4 empty blobs
(all `__init__.py` / `.gitkeep`), 1 symlink (CLAUDE.md), 0 submodules. On the
tree before that cleanup the same rules reject 41 paths: the 40 accidents
above, plus xe125/README.md, whose one-file top-level directory the cleanup
folded into docs/hardware/.
"""

from __future__ import annotations

import argparse
import fnmatch
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

# The only files allowed at the repository root.
ROOT_FILES = frozenset(
    {
        ".clang-format",
        ".editorconfig",
        ".gitignore",
        ".swiftlint.yml",
        "AGENTS.md",
        "CLAUDE.md",  # symlink to AGENTS.md
        "Cargo.lock",
        "Cargo.toml",
        "LICENSE",
        "README.md",
        "SECURITY.md",  # where to report a vulnerability (public-release-readiness T-PR-4)
        "pyproject.toml",
        "rust-toolchain.toml",
    }
)

# The only directories allowed at the repository root.
ROOT_DIRS = frozenset(
    {
        ".cargo",
        ".claude",
        ".githooks",
        ".github",
        ".xcodebuildmcp",
        "api",
        "artifacts",
        "batch",
        "clients",
        "crates",
        "db",
        "deploy",
        "docs",
        "fuzz",
        "openspec",
        "scripts",
        "tools",
        "vitals",
    }
)

# Where binary files may live. Matched with fnmatch, where `*` crosses `/`.
BINARY_ALLOWED = (
    "docs/images/*.png",
    "clients/apple/*/Assets.xcassets/*.png",
)

# Suffixes that mark a file as binary-kind whatever its bytes say.
BINARY_SUFFIXES = frozenset(
    {
        # images and recordings
        ".png", ".jpg", ".jpeg", ".gif", ".heic", ".webp", ".bmp", ".tif", ".tiff",
        ".mp4", ".mov", ".m4v", ".webm", ".mp3", ".m4a", ".wav",
        # documents and exports
        ".pdf", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx", ".key", ".numbers",
        ".pages",
        # archives
        ".zip", ".tar", ".gz", ".tgz", ".bz2", ".xz", ".7z", ".rar", ".zst",
        # databases and dumps
        ".sqlite", ".sqlite3", ".db", ".dump", ".bin",
    }
)  # fmt: skip

# Zero-byte files are almost always an accident (see `[^` and `]*` in
# test-tracked-filenames.sh). These names are empty by design.
EMPTY_ALLOWED_NAMES = frozenset({"__init__.py", ".gitkeep"})

# Path components that are only ever local tool state. These are rejected at
# any depth, so a nested copy cannot slip past the root allowlist.
LOCAL_STATE_COMPONENTS = frozenset(
    {
        ".DS_Store",
        ".build",
        ".idea",
        ".pytest_cache",
        ".remember",
        ".ruff_cache",
        ".venv",
        ".vscode",
        "__pycache__",
        "node_modules",
        "target",
    }
)

GITLINK_MODE = "160000"


@dataclass(frozen=True)
class Entry:
    path: str
    binary: bool = False  # git classifies the committed blob as non-text
    empty: bool = False  # the committed blob has zero bytes
    gitlink: bool = False  # a submodule pointer, not a file


def _local_or_secret(name: str) -> str | None:
    """Name the reason a filename reads as machine-local or secret, if it does."""
    if name.endswith(".example"):
        return None
    if name.endswith(".local") or ".local." in name:
        return "machine-local file (*.local); commit a *.example instead"
    if name == ".env" or name.startswith(".env."):
        return "environment file; commit a *.example instead"
    if name.endswith((".pem", ".p12", ".mobileprovision")):
        return "key or credential material"
    return None


def violations(entries: list[Entry]) -> list[str]:
    found: list[str] = []
    for entry in entries:
        path = PurePosixPath(entry.path)
        parts = path.parts

        if len(parts) == 1 and entry.path not in ROOT_FILES:
            found.append(f"{entry.path}: not in the root-file allowlist {sorted(ROOT_FILES)}")
        elif len(parts) > 1 and parts[0] not in ROOT_DIRS:
            found.append(f"{entry.path}: top-level directory {parts[0]!r} is not in the allowlist")

        if entry.gitlink:
            found.append(f"{entry.path}: submodule; this repository vendors nothing by gitlink")
            continue

        local = sorted(LOCAL_STATE_COMPONENTS.intersection(parts))
        if local:
            found.append(f"{entry.path}: local tool state under {local[0]!r}")

        reason = _local_or_secret(path.name)
        if reason:
            found.append(f"{entry.path}: {reason}")

        suffix = path.suffix.lower()
        if entry.binary or suffix in BINARY_SUFFIXES:
            kind = "binary content" if entry.binary else f"binary-kind suffix {suffix!r}"
            if not any(fnmatch.fnmatchcase(entry.path, g) for g in BINARY_ALLOWED):
                found.append(
                    f"{entry.path}: {kind} outside the binary allowlist {list(BINARY_ALLOWED)}"
                )

        if entry.empty and path.name not in EMPTY_ALLOWED_NAMES:
            found.append(f"{entry.path}: empty file, usually an unquoted shell expansion")
    return found


def _git(repo: Path, *args: str, stdin: bytes | None = None) -> bytes:
    return subprocess.run(
        ["git", "-C", str(repo), *args], input=stdin, check=True, capture_output=True
    ).stdout


def tracked_entries(repo: Path) -> list[Entry]:
    """Read every tracked path with git's own view of the *committed* content.

    Three facts per path, each from the index rather than the work tree:
    - mode and object id from `ls-files -s` (gitlinks are mode 160000);
    - empty iff the object id equals the empty blob's id, computed with
      `hash-object` so it is right in SHA-1 and SHA-256 repositories alike;
    - binary iff `ls-files --eol` reports `i/-text`. (`i/none` is NOT "empty":
      it means "no line terminator", which a one-line file also has.)
    """
    empty_oid = _git(repo, "hash-object", "-t", "blob", "--stdin", stdin=b"").strip().decode()

    staged: dict[bytes, tuple[str, str]] = {}
    for record in _git(repo, "ls-files", "-s", "-z").split(b"\0"):
        if record:
            meta, _, path = record.partition(b"\t")
            mode, oid, _stage = meta.decode().split(" ")
            staged[path] = (mode, oid)

    binary: set[bytes] = set()
    for record in _git(repo, "ls-files", "--eol", "-z").split(b"\0"):
        if record:
            info, _, path = record.partition(b"\t")
            fields = info.split()
            if fields and fields[0] == b"i/-text":
                binary.add(path)

    return [
        Entry(
            path=path.decode("utf-8", "surrogateescape"),
            binary=path in binary,
            empty=mode != GITLINK_MODE and oid == empty_oid,
            gitlink=mode == GITLINK_MODE,
        )
        for path, (mode, oid) in sorted(staged.items())
    ]


def _expect(entry: Entry, reason: str | None) -> None:
    """Assert `entry` yields exactly one violation containing `reason`, or none."""
    found = violations([entry])
    if reason is None:
        if found:
            raise AssertionError(f"tracked-paths self-test: rejected {entry}: {found}")
        return
    if len(found) != 1 or reason not in found[0]:
        raise AssertionError(
            f"tracked-paths self-test: {entry} should trip only {reason!r}, got {found}"
        )


def _self_test_rules() -> None:
    # One fixture per rule, placed so that it trips ONLY that rule (under an
    # allowed top-level directory, as text). If a rule is deleted its fixture
    # yields nothing, and if two rules overlap the count is wrong. Both fail.
    isolated = [
        (Entry("notes-for-later.md"), "root-file allowlist"),
        (Entry("PI-KICKOFF.md"), "root-file allowlist"),
        (Entry("notallowed/readme.md"), "top-level directory"),
        (Entry("docs/.idea/misc.xml"), "'.idea'"),
        (Entry("docs/.DS_Store"), "'.DS_Store'"),
        (Entry("clients/apple/Packages/AWDomain/.build/x"), "'.build'"),
        (Entry("tools/.pytest_cache/v"), "'.pytest_cache'"),
        (Entry("docs/.remember/now.md"), "'.remember'"),
        (Entry("tools/.ruff_cache/x"), "'.ruff_cache'"),
        (Entry("tools/.venv/pyvenv.cfg"), "'.venv'"),
        (Entry("docs/.vscode/settings.json"), "'.vscode'"),
        (Entry("tools/__pycache__/x.pyc"), "'__pycache__'"),
        (Entry("tools/node_modules/x/index.js"), "'node_modules'"),
        (Entry("crates/x/target/debug/x"), "'target'"),
        (Entry("scripts/opentel.env.local"), "machine-local"),
        (Entry(".githooks/pii-denylist.local.txt"), "machine-local"),
        (Entry("deploy/.env"), "environment file"),
        (Entry("deploy/.env.prod"), "environment file"),
        (Entry("deploy/tls/server.pem"), "key or credential"),
        (Entry("clients/apple/dist.p12"), "key or credential"),
        (Entry("clients/apple/dev.mobileprovision"), "key or credential"),
        (Entry("crates/vitalsd/fixture", binary=True), "binary content"),
        (Entry("docs/images/readme/fig.bin", binary=True), "binary content"),
        (Entry("docs/hardware/datasheet.pdf"), "suffix '.pdf'"),  # all-ASCII PDF
        (Entry("docs/shot.png"), "suffix '.png'"),  # LFS pointer shape
        (Entry("docs/research/snapshot.zip"), "suffix '.zip'"),
        (Entry("docs/research/night.MP4"), "suffix '.mp4'"),
        (Entry("scripts/notes.md", empty=True), "empty file"),
        (Entry("docs/sub", gitlink=True), "submodule"),
    ]
    for entry, reason in isolated:
        _expect(entry, reason)

    legitimate = [
        Entry("README.md"),
        Entry("CLAUDE.md"),
        Entry("crates/vitalsd/src/main.rs"),
        Entry(".githooks/pii-denylist.local.txt.example"),
        Entry("docs/images/readme/three-states.png", binary=True),
        Entry(
            "clients/apple/AnxietyWatchApp/Assets.xcassets/AppIcon.appiconset/AppIcon.png",
            binary=True,
        ),
        Entry("batch/cpap_edf/__init__.py", empty=True),
        Entry("artifacts/.gitkeep", empty=True),
        Entry("docs/design/admin-console/claude-design/a/A - Clinical.dc.html"),
        Entry("docs/oneline.txt"),  # no trailing newline: git says i/none; not empty
    ]
    for entry in legitimate:
        _expect(entry, None)


def _self_test_classifier() -> None:
    """Prove tracked_entries() classifies real git objects, not just fixtures.

    Without this, a parsing slip that set binary/empty to False everywhere
    would pass the rule tests above AND a clean tree: "no violations" would be
    indistinguishable from "classification never ran".
    """
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        _git(repo, "init", "-q")
        (repo / "blob.dat").write_bytes(b"\x89PNG\r\n\x1a\n\x00\x00binary")
        (repo / "empty.md").write_bytes(b"")
        (repo / "oneline.txt").write_bytes(b"no newline")
        (repo / "text.md").write_bytes(b"ordinary text\n")
        (repo / "link.md").symlink_to("text.md")
        _git(repo, "add", "--", "blob.dat", "empty.md", "oneline.txt", "text.md", "link.md")
        got = {e.path: (e.binary, e.empty, e.gitlink) for e in tracked_entries(repo)}

    want = {
        "blob.dat": (True, False, False),
        "empty.md": (False, True, False),
        "oneline.txt": (False, False, False),
        "text.md": (False, False, False),
        "link.md": (False, False, False),
    }
    if got != want:
        raise AssertionError(f"tracked-paths self-test: classifier got {got}, want {want}")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Reject tracked files that belong on a workstation, not in the repository."
    )
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        _self_test_rules()
        _self_test_classifier()
        print("tracked-paths self-test passed")

    root = Path(_git(Path.cwd(), "rev-parse", "--show-toplevel").decode().strip())
    entries = tracked_entries(root)
    found = violations(entries)
    if found:
        print(f"tracked-paths: {len(found)} violation(s):", file=sys.stderr)
        for line in found:
            print(f"  {line}", file=sys.stderr)
        print(
            "\nIf the file belongs in the repository, widen the matching allowlist in\n"
            "scripts/check-tracked-paths.py in the same commit, and say why. If it does\n"
            "not, `git rm --cached -- '<path>'` and add it to .gitignore.",
            file=sys.stderr,
        )
        return 1
    print(f"tracked-paths: all {len(entries)} tracked paths are in policy")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
