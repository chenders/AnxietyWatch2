#!/usr/bin/env python3
"""Reject self-hosted runners on any workflow a fork could trigger.

`pi-deploy.yml` and `pi-runner-smoke.yml` run on a self-hosted runner on the
Pi. Today only `push` and `workflow_dispatch` trigger them, and both need write
access to this repository. On a public repository, any other trigger --
`pull_request`, `pull_request_target`, `issue_comment`, `workflow_run`, a
reusable `workflow_call` -- can be fired by someone without write access, and a
job of such a workflow on the self-hosted runner would run a stranger's code on
the Pi, on the owner's LAN.

So: a workflow whose triggers include anything outside SAFE_TRIGGERS may use
GitHub-hosted runners only. Both lists are allowlists, so an unknown trigger,
an unknown runner label or a `runs-on` expression that cannot be resolved here
all fail closed. A violation names the file, the job, the trigger and the
runner, per AGENTS.md "Guards must say why".

Usage:
  check-workflow-runners.py [WORKFLOW_DIR]   (default .github/workflows)
  check-workflow-runners.py --self-test
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # fail closed, and say why
    print(
        "check-workflow-runners: blocked: PyYAML is not importable by "
        f"{sys.executable}; install python3-yaml (GitHub's Ubuntu runners ship it)",
        file=sys.stderr,
    )
    sys.exit(2)

# Triggers that require write access to this repository to fire.
SAFE_TRIGGERS = frozenset({"push", "workflow_dispatch", "schedule"})

# The only reusable-workflow form whose runners this script can see.
LOCAL_REUSABLE = "./.github/workflows/"

# GitHub-hosted runner labels: ubuntu-24.04, ubuntu-latest, macos-15, windows-2025, ...
HOSTED_LABEL = re.compile(r"^(ubuntu|macos|windows)-(latest|[0-9][0-9.]*(-arm)?)$")


def triggers(doc: dict) -> list[str]:
    # YAML 1.1 reads a bare `on:` key as the boolean True.
    on = doc.get("on", doc.get(True))
    if on is None:
        return []
    if isinstance(on, str):
        return [on]
    if isinstance(on, list):
        return [str(t) for t in on]
    if isinstance(on, dict):
        return [str(t) for t in on]
    return [f"<unparseable on: {on!r}>"]


def runner_labels(runs_on: object) -> list[str] | None:
    """Return the job's runner labels, or None if they cannot be known statically."""
    if isinstance(runs_on, str):
        return None if "${{" in runs_on else [runs_on]
    if isinstance(runs_on, list) and all(isinstance(x, str) for x in runs_on):
        return None if any("${{" in x for x in runs_on) else list(runs_on)
    return None  # {group: ...} runner groups, expressions, anything else


def check_workflow(name: str, doc: object, local_files: set[str] | None = None) -> list[str]:
    if not isinstance(doc, dict):
        return [f"{name}: blocked: not a YAML mapping, so its triggers cannot be checked"]
    risky = sorted(t for t in triggers(doc) if t not in SAFE_TRIGGERS)
    if not risky:
        return []
    found = []
    for job_id, job in (doc.get("jobs") or {}).items():
        if not isinstance(job, dict):
            found.append(f"{name}: job {job_id}: blocked: not a mapping")
            continue
        if "uses" in job:
            # A reusable workflow's jobs run on THIS repository's runners. A
            # local one is checked as a file in its own right (workflow_call is
            # not a safe trigger); a remote one cannot be checked from here.
            target = job["uses"]
            local = isinstance(target, str) and target.startswith(LOCAL_REUSABLE)
            if not local:
                found.append(
                    f"{name}: job {job_id}: blocked: calls remote reusable workflow {target!r}, "
                    f"whose runners cannot be checked here, and the workflow is triggerable "
                    f"by {', '.join(risky)}"
                )
            elif local_files is not None and target[len(LOCAL_REUSABLE) :] not in local_files:
                found.append(
                    f"{name}: job {job_id}: blocked: calls {target!r}, which is not in the "
                    f"checked workflow directory, so its runners were not checked"
                )
            continue
        labels = runner_labels(job.get("runs-on"))
        if labels is None:
            found.append(
                f"{name}: job {job_id}: blocked: runs-on {job.get('runs-on')!r} cannot be "
                f"proven GitHub-hosted, and the workflow is triggerable by {', '.join(risky)}"
            )
            continue
        bad = [label for label in labels if not HOSTED_LABEL.match(label)]
        if bad:
            found.append(
                f"{name}: job {job_id}: blocked: runs-on {labels!r} includes non-GitHub-hosted "
                f"label(s) {bad!r}, and the workflow is triggerable by {', '.join(risky)}"
            )
    return found


def check_dir(directory: Path) -> list[str]:
    files = sorted([*directory.glob("*.yml"), *directory.glob("*.yaml")])
    if not files:
        return [f"{directory}: blocked: no workflow files found, so nothing was checked"]
    found = []
    for path in files:
        try:
            doc = yaml.safe_load(path.read_text(encoding="utf-8"))
        except yaml.YAMLError as exc:
            found.append(f"{path}: blocked: does not parse as YAML: {exc}")
            continue
        found.extend(check_workflow(str(path), doc, {p.name for p in files}))
    return found


SELF_TESTS: list[tuple[str, str, bool]] = [
    # (name, workflow YAML, expect_violation)
    ("pr-on-pi", "on: pull_request\njobs:\n  a:\n    runs-on: [self-hosted, linux]\n", True),
    ("prt-on-pi", "on: {pull_request_target: {}}\njobs:\n  a:\n    runs-on: self-hosted\n", True),
    ("comment-on-pi", "on: [push, issue_comment]\njobs:\n  a:\n    runs-on: [self-hosted]\n", True),
    ("call-on-pi", "on: workflow_call\njobs:\n  a:\n    runs-on: [self-hosted]\n", True),
    ("pr-expr", "on: pull_request\njobs:\n  a:\n    runs-on: ${{ matrix.os }}\n", True),
    ("pr-group", "on: pull_request\njobs:\n  a:\n    runs-on: {group: pi}\n", True),
    ("pr-custom-label", "on: pull_request\njobs:\n  a:\n    runs-on: vitals-pi\n", True),
    (
        "push-on-pi",
        "on:\n  push:\n    branches: [main]\njobs:\n  a:\n    runs-on: [self-hosted, ARM64]\n",
        False,
    ),
    ("dispatch-on-pi", "on: workflow_dispatch\njobs:\n  a:\n    runs-on: [self-hosted]\n", False),
    (
        "pr-hosted",
        "on: pull_request\njobs:\n  a:\n    runs-on: ubuntu-latest\n  b:\n    runs-on: macos-15\n",
        False,
    ),
    (
        "pr-remote-reusable",
        "on: pull_request\njobs:\n  a:\n    uses: someone/else/.github/workflows/x.yml@main\n",
        True,
    ),
    (
        "pr-missing-local-reusable",
        "on: pull_request\njobs:\n  a:\n    uses: ./.github/workflows/gone.yml\n",
        True,
    ),
    (
        "push-remote-reusable",
        "on: push\njobs:\n  a:\n    uses: someone/else/.github/workflows/x.yml@main\n",
        False,
    ),
    (
        "pr-reusable-caller",
        "on: pull_request\njobs:\n  a:\n    uses: ./.github/workflows/x.yml\n",
        False,
    ),
]


def self_test() -> int:
    failures = 0
    for name, text, expect in SELF_TESTS:
        got = bool(check_workflow(name, yaml.safe_load(text), {"x.yml"}))
        if got != expect:
            print(
                f"self-test FAILED: {name}: expected violation={expect}, got {got}",
                file=sys.stderr,
            )
            failures += 1
    if failures:
        return 1
    print(f"check-workflow-runners self-test: {len(SELF_TESTS)} cases pass")
    return 0


def main(argv: list[str]) -> int:
    if argv[1:] == ["--self-test"]:
        return self_test()
    directory = Path(argv[1]) if len(argv) > 1 else Path(".github/workflows")
    found = check_dir(directory)
    for line in found:
        print(line, file=sys.stderr)
    if found:
        return 1
    print(f"no fork-triggerable workflow in {directory} uses a self-hosted runner")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
