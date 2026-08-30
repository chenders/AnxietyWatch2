#!/usr/bin/env python3
"""Validate the Stage 7 Apple-client specification and executable task queue."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHANGE = ROOT / "openspec/changes/ios-client-stage7"
SPEC = ROOT / "docs/design/SPEC-ios-client.md"
REQUIRED = [
    SPEC,
    CHANGE / "proposal.md",
    CHANGE / "design.md",
    CHANGE / "tasks.md",
    CHANGE / "traceability.md",
    CHANGE / "specs/apple-client/spec.md",
]
TASK_FIELDS = ("**Amends:**", "**Context budget:**", "**Reads:**", "**Do:**", "**Verify:**")
EXPECTED_IDS = {f"T-I{number}" for number in range(22)}


def fail(message: str) -> None:
    print(f"ios-client-plan: FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def check_files() -> None:
    missing = [str(path.relative_to(ROOT)) for path in REQUIRED if not path.is_file()]
    if missing:
        fail(f"missing required files: {', '.join(missing)}")


def check_spec() -> None:
    text = SPEC.read_text(encoding="utf-8")
    required_phrases = (
        "HealthKit bridge in",
        "Journal in",
        "Viewer out",
        "Upload at the earliest opportunity",
        "generation-one HealthKit coverage parity",
        "payload is deleted from local persistence in the same",
        "local detector-to-klaxon path.",
        "does **not**",
        "Stage 7 is complete only when",
    )
    for phrase in required_phrases:
        if phrase not in text:
            fail(f"spec is missing governing phrase: {phrase!r}")
    for heading in range(1, 14):
        if not re.search(rf"^## {heading}\. ", text, flags=re.MULTILINE):
            fail(f"spec is missing numbered section {heading}")


def check_tasks() -> None:
    text = (CHANGE / "tasks.md").read_text(encoding="utf-8")
    matches = list(re.finditer(r"^### (T-I\d+) — .+$", text, flags=re.MULTILINE))
    found = {match.group(1) for match in matches}
    if found != EXPECTED_IDS:
        fail(
            "task brief IDs differ: "
            f"missing={sorted(EXPECTED_IDS - found)}, extra={sorted(found - EXPECTED_IDS)}"
        )
    if len(matches) != len(found):
        fail("duplicate task brief ID")

    for index, match in enumerate(matches):
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        section = text[match.start() : end]
        task_id = match.group(1)
        for field in TASK_FIELDS:
            if field not in section:
                fail(f"{task_id} is missing {field}")
        verify_line = next(
            (line for line in section.splitlines() if line.startswith("**Verify:**")), ""
        )
        if "`" not in verify_line or "cd \"$(git rev-parse --show-toplevel)\"" not in verify_line:
            fail(f"{task_id} Verify must be a root-relative shell command in backticks")

    queue_ids = set(re.findall(r"^- \[ \] (T-I\d+) —", text, flags=re.MULTILINE))
    if queue_ids != EXPECTED_IDS:
        fail(
            "queue IDs differ from briefs: "
            f"missing={sorted(EXPECTED_IDS - queue_ids)}, extra={sorted(queue_ids - EXPECTED_IDS)}"
        )


def check_local_links() -> None:
    link_pattern = re.compile(r"\[[^]]+\]\(([^)]+)\)")
    for document in REQUIRED:
        text = document.read_text(encoding="utf-8")
        for target in link_pattern.findall(text):
            if target.startswith(("http://", "https://", "#")):
                continue
            path_text = target.split("#", maxsplit=1)[0]
            if not path_text:
                continue
            resolved = (document.parent / path_text).resolve()
            if not resolved.exists():
                fail(
                    f"broken local link in {document.relative_to(ROOT)}: {target} "
                    f"-> {resolved}"
                )


def main() -> None:
    check_files()
    check_spec()
    check_tasks()
    check_local_links()
    print("ios-client-plan: PASS — spec, 22 task contracts, traceability, and links are complete")


if __name__ == "__main__":
    main()
