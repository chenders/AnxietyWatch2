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

Calibration, measured on 871 tracked paths after the cleanup that introduced
this gate: 4 binary files (all PNG, in the two locations below) and 4 empty
files (all `__init__.py` / `.gitkeep`). On the tree before that cleanup the
same rules reject 41 paths: the 40 accidents above, plus xe125/README.md, whose
one-file top-level directory the cleanup folded into docs/hardware/.
"""

from __future__ import annotations

import argparse
import fnmatch
import subprocess
import sys
from dataclasses import dataclass
from pathlib import PurePosixPath

# The only files allowed at the repository root.
ROOT_FILES = frozenset(
    {
        ".clang-format",  # firmware/ is Zephyr C
        ".editorconfig",
        ".gitignore",
        ".swiftlint.yml",
        "AGENTS.md",
        "CLAUDE.md",  # symlink to AGENTS.md
        "Cargo.lock",
        "Cargo.toml",
        "LICENSE",
        "README.md",
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
        "firmware",
        "fuzz",
        "openspec",
        "scripts",
        "tools",
        "vitals",
    }
)

# Where binary blobs may live. Matched with fnmatch, where `*` crosses `/`.
BINARY_ALLOWED = (
    "docs/images/*.png",
    "clients/apple/*/Assets.xcassets/*.png",
)

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


@dataclass(frozen=True)
class Entry:
    path: str
    binary: bool
    empty: bool


def _local_or_secret(name: str) -> str | None:
    """Name the reason a filename reads as machine-local or secret, if it does."""
    if name.endswith(".example"):
        return None
    if name.endswith(".local") or ".local." in name:
        return "machine-local file (*.local); commit a *.example instead"
    if name == ".env" or name.startswith(".env."):
        return "environment file; commit a *.example instead"
    if name.endswith((".pem", ".key", ".p12", ".mobileprovision")):
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

        local = sorted(LOCAL_STATE_COMPONENTS.intersection(parts))
        if local:
            found.append(f"{entry.path}: local tool state under {local[0]!r}")

        reason = _local_or_secret(path.name)
        if reason:
            found.append(f"{entry.path}: {reason}")

        if entry.binary and not any(fnmatch.fnmatchcase(entry.path, g) for g in BINARY_ALLOWED):
            found.append(
                f"{entry.path}: binary file outside the binary allowlist {list(BINARY_ALLOWED)}"
            )

        if entry.empty and path.name not in EMPTY_ALLOWED_NAMES:
            found.append(f"{entry.path}: empty file, usually an unquoted shell expansion")
    return found


def tracked_entries() -> list[Entry]:
    """Read every tracked path with git's own view of the *committed* content.

    `git ls-files --eol` reports `i/-text` for content git treats as binary and
    `i/none` for an empty blob. Using the index, not the work tree, means CI and
    the pre-push hook judge exactly what is committed.
    """
    raw = subprocess.check_output(["git", "ls-files", "--eol", "-z"])
    entries: list[Entry] = []
    for record in raw.split(b"\0"):
        if not record:
            continue
        info, _, path = record.partition(b"\t")
        index_eol = info.split()[0].decode() if info.split() else ""
        entries.append(
            Entry(
                path=path.decode("utf-8", "surrogateescape"),
                binary=index_eol == "i/-text",
                empty=index_eol == "i/none",
            )
        )
    return entries


def self_test() -> None:
    def text(path: str) -> Entry:
        return Entry(path, binary=False, empty=False)

    def blob(path: str) -> Entry:
        return Entry(path, binary=True, empty=False)

    def empty(path: str) -> Entry:
        return Entry(path, binary=False, empty=True)

    # Each fixture is a shape that actually reached main, or its nearest variant.
    bad = [
        blob("health.jpg"),
        blob("journal entry.PNG"),
        blob("h10_vitals.mp4"),
        blob("aw2-ios-thin-client-chatgpt-context.zip"),
        text("PI-KICKOFF.md"),
        text("CHATGPT-WEB-PROMPT.md"),
        blob("xe125/README.pdf"),
        blob("docs/hardware/datasheet.pdf"),
        blob("docs/images/readme/screenshot.jpg"),
        blob("crates/vitalsd/fixture.bin"),
        text(".idea/misc.xml"),
        text("docs/.DS_Store"),
        text("tools/__pycache__/x.cpython-312.pyc"),
        text("clients/apple/Packages/AWDomain/.build/x"),
        text("etc/opentel.env.local"),
        text(".githooks/pii-denylist.local.txt"),
        text("deploy/.env"),
        text("deploy/tls/server.key"),
        empty("[^"),
        empty("scripts/notes.md"),
    ]
    for entry in bad:
        if not violations([entry]):
            raise AssertionError(f"tracked-paths self-test: missed {entry}")

    good = [
        text("README.md"),
        text("CLAUDE.md"),
        text("crates/vitalsd/src/main.rs"),
        text(".githooks/pii-denylist.local.txt.example"),
        blob("docs/images/readme/three-states.png"),
        blob("clients/apple/AnxietyWatchApp/Assets.xcassets/AppIcon.appiconset/AppIcon.png"),
        empty("batch/cpap_edf/__init__.py"),
        empty("artifacts/.gitkeep"),
        text("docs/design/admin-console/claude-design/a/A - Clinical.dc.html"),
    ]
    for entry in good:
        if found := violations([entry]):
            raise AssertionError(f"tracked-paths self-test: rejected {entry}: {found}")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Reject tracked files that belong on a workstation, not in the repository."
    )
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        print("tracked-paths self-test passed")

    entries = tracked_entries()
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
