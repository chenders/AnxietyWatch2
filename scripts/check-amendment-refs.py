#!/usr/bin/env python3
"""Check that amendment citations resolve to one amendment, unambiguously.

`AGENTS.md` makes `docs/specs/AMENDMENTS.md` mandatory reading because an
amendment supersedes the original design, and a brief that contradicts an
accepted amendment is wrong. That rule assumes a reader who follows a citation
arrives at the decision the citing text describes. Two things in this
repository break that assumption, and they need different treatment.

**The index carries two parallel numbering series.** `10-A11` (AirSense
acceptance scope) and `11-A11` (H10 live gate) were written on 2026-09-13 and
are still named `amendment-10-...` and `amendment-11-...` on disk. Two days
later the main series created its own Amendment 10 (first-draft private-LAN
transport) and Amendment 11 (optional direct lyrics), and the index
disambiguated the earlier pair with an `-A11` suffix. Text citing the originals
was never updated, so `Amendment 10` in a status update from 2026-09-13 now
resolves to a document about LAN transport. This is structural, decidable
without judgement, and is what this gate blocks on.

**A citation can also name the wrong amendment outright.** On 2026-11-06 three
files cited "Amendment 12" as the source of a durable therapy-session-record
requirement; Amendment 12 is *Provider-aware song search* and contains the word
"therapy" zero times, and no accepted amendment specifies that requirement at
all. One is a `///` doc comment telling a future worker an endpoint "must not
pretend that a transport or time window is a finalized therapy session"
*because of* that amendment — so a worker obeying the mandatory-reading rule
would read about song search, find no such constraint, and could reasonably
conclude the guard was stale and remove it.

That second class is only detectable semantically, and every mechanical proxy
tried here either missed it or buried it:

  * An **existence** check passes it. Amendment 12 exists and is indexed.
  * Demanding every citation carry a title or link flags **111** sites, because
    a bare number is the house style for ordinary cross-references. That is a
    migration, not a gate, and it would be turned off.
  * Requiring a requirement-asserting citation to share distinctive vocabulary
    with its target flags **25**, most of them terse but correct — a
    `**Amends:** Amendment 05` line has almost no context words, so it can
    never reach a two-word threshold. Vocabulary cannot separate "terse and
    right" from "wrong".

So the vocabulary test ships as **advisory**: reported, never gating. It is a
heuristic with a known false-positive rate, and calling it a gate would mean
either editing correct text to satisfy it or turning it off. Its job is to put
candidates in front of a human.

Exit status is the contract:

    0  every citation resolves to exactly one amendment
    1  a citation names an amendment that is not indexed, or names a number
       that has a suffixed sibling without saying which one it means
    2  the index could not be parsed, or no amendment document was readable

Advisory findings never change the exit status. `--advisory-only` reports them
and exits 0 regardless.

Usage:
    scripts/check-amendment-refs.py [--self-test] [--root DIR]
                                    [--advisory | --advisory-only]
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

INDEX_PATH = Path("docs/specs/AMENDMENTS.md")
SPECS_DIR = Path("docs/specs")

EXEMPT_PATTERNS = (
    re.compile(r"^docs/specs/AMENDMENTS\.md$"),
    re.compile(r"^docs/specs/\d{4}-\d{2}-\d{2}-amendment-.*\.md$"),
    re.compile(r"^scripts/check-amendment-refs\.py$"),
    re.compile(r"^openspec/changes/amendment-reference-integrity/"),
)

# A suffixed citation such as "Amendment 10-A11" names its own row and must not
# be read as "Amendment 10".
CITATION_RE = re.compile(r"\bAmendments?[\s ]+(\d{1,2})(-[A-Za-z0-9]+)?\b")

INDEX_ROW_RE = re.compile(
    r"^\|\s*\*{0,2}(?P<num>[0-9]{1,2}(?:-[A-Za-z0-9]+)?)\*{0,2}\s*\|"
    r"\s*\[(?P<title>[^\]]+)\]\((?P<href>[^)]+)\)"
)

# Phrasings that put a requirement on the amendment's authority. A bare
# "see Amendment 11" is deliberately absent. "specifies"/"specified" are listed
# rather than `specif\w*`, which matches the noun in "the iOS specification"
# and asserts nothing.
ASSERTION_RE = re.compile(
    r"""(
        requir\w*  | mandat\w*   | specifies | specified | provide\w*  |
        forbid\w*  | permit\w*   | prohibit\w* | oblig\w* |
        \bSHALL\b  | \bMUST\b    | compliance | conform\w*
    )""",
    re.VERBOSE | re.IGNORECASE,
)

STOPWORDS = frozenset(
    """
    the and for not but with from into onto over under this that these those
    are was were been being have has had will shall should would could may
    might must can does did done than then when where which while who whom
    whose what why how all any both each few more most other some such only
    own same too very just also about against between during before after
    above below again further once here there amendment amendments design
    repository section sections note notes yet still already because
    therefore however rather instead
    """.split()
)

WORD_RE = re.compile(r"[A-Za-z][A-Za-z0-9-]{3,}")

MIN_SHARED_WORDS = 2
CONTEXT_LINES = 1


@dataclass(frozen=True)
class Amendment:
    number: str
    title: str
    href: str
    vocabulary: frozenset[str]


@dataclass(frozen=True)
class Finding:
    path: str
    line: int
    cited: str
    reason: str
    detail: str

    def render(self) -> str:
        return f"{self.path}:{self.line}: Amendment {self.cited} — {self.reason}\n      {self.detail}"


def content_words(text: str) -> frozenset[str]:
    return frozenset(w for w in (m.lower() for m in WORD_RE.findall(text)) if w not in STOPWORDS)


def parse_index(text: str) -> dict[str, tuple[str, str]]:
    rows: dict[str, tuple[str, str]] = {}
    for raw in text.splitlines():
        match = INDEX_ROW_RE.match(raw.strip())
        if match:
            rows[match.group("num")] = (match.group("title").strip(), match.group("href").strip())
    return rows


def colliding_numbers(rows: dict[str, tuple[str, str]]) -> frozenset[str]:
    """Bare numbers that are ambiguous because a suffixed sibling shares them."""
    return frozenset(
        n for n in rows if "-" not in n and any(k.startswith(n + "-") for k in rows)
    )


def build_corpus(rows: dict[str, tuple[str, str]], specs_dir: Path) -> dict[str, Amendment]:
    """Load each amendment's text, keeping only vocabulary distinctive to it.

    A word is distinctive if it is in the amendment's own title or appears in no
    other amendment. An earlier version dropped words appearing in more than a
    fixed FRACTION of the corpus; that is not scale-invariant and it failed
    silently. At three amendments it excluded anything in two of them, but at
    the real repository's twenty it only excluded words in eleven — so generic
    words like "record" and "returns" survived, and two of them were enough to
    pass the very citation this gate was written for. The small test fixture
    could not reproduce it, which is why `self_test` now checks this against a
    corpus large enough to show the effect.
    """
    raw_vocab: dict[str, frozenset[str]] = {}
    kept: dict[str, tuple[str, str]] = {}
    for number, (title, href) in rows.items():
        try:
            body = (specs_dir / Path(href).name).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        raw_vocab[number] = content_words(title + "\n" + body)
        kept[number] = (title, href)

    if not raw_vocab:
        return {}

    counts: Counter[str] = Counter()
    for vocab in raw_vocab.values():
        counts.update(vocab)

    corpus: dict[str, Amendment] = {}
    for number, vocab in raw_vocab.items():
        title, href = kept[number]
        title_words = content_words(title)
        corpus[number] = Amendment(
            number, title, href,
            frozenset(w for w in vocab if counts[w] == 1 or w in title_words),
        )
    return corpus


def is_exempt(rel_path: str) -> bool:
    return any(pattern.match(rel_path) for pattern in EXEMPT_PATTERNS)


def tracked_files(root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z",
         "*.md", "*.rs", "*.toml", "*.yml", "*.yaml", "*.sh", "*.swift"],
        capture_output=True, text=True, check=False,
    )
    return [p for p in result.stdout.split("\0") if p] if result.returncode == 0 else []


def disambiguated(amendment: Amendment, context: str) -> bool:
    """Does the context say which amendment of a colliding number it means?"""
    lowered = context.lower()
    filename = Path(amendment.href).name.lower()
    if filename in lowered or filename.removesuffix(".md") in lowered:
        return True
    return len(content_words(context) & content_words(amendment.title)) >= MIN_SHARED_WORDS


def scan_text(
    rel_path: str,
    text: str,
    corpus: dict[str, Amendment],
    rows: dict[str, tuple[str, str]],
    colliding: frozenset[str],
) -> tuple[list[Finding], list[Finding]]:
    """Return (blocking, advisory)."""
    blocking: list[Finding] = []
    advisory: list[Finding] = []
    lines = text.splitlines()

    for idx, line in enumerate(lines):
        for match in CITATION_RE.finditer(line):
            cited = match.group(1) + (match.group(2) or "")
            lo = max(0, idx - CONTEXT_LINES)
            hi = min(len(lines), idx + CONTEXT_LINES + 1)
            context = "\n".join(lines[lo:hi])

            if cited not in rows:
                blocking.append(Finding(
                    rel_path, idx + 1, cited, "not in the amendment index",
                    f"{INDEX_PATH} has no row for {cited}; "
                    f"indexed: {', '.join(sorted(rows)) or '(none)'}",
                ))
                continue

            amendment = corpus.get(cited)
            if amendment is None:
                blocking.append(Finding(
                    rel_path, idx + 1, cited, "indexed but its document is unreadable",
                    f"{INDEX_PATH} points at {Path(rows[cited][1]).name}, which is absent or "
                    "undecodable; the citation cannot be checked, which is not the same as "
                    "being correct",
                ))
                continue

            # Blocking tier: a bare number that a suffixed sibling makes ambiguous.
            if cited in colliding and not disambiguated(amendment, context):
                siblings = sorted(k for k in rows if k == cited or k.startswith(cited + "-"))
                blocking.append(Finding(
                    rel_path, idx + 1, cited, "ambiguous — two amendments share this number",
                    "could mean "
                    + "; ".join(f'{k} "{rows[k][0]}"' for k in siblings)
                    + f". Name the title, the slug, or link {amendment.href}.",
                ))
                continue

            # Advisory tier: a requirement asserted on an amendment that does
            # not appear to discuss the subject.
            if not ASSERTION_RE.search(context):
                continue
            shared = sorted(content_words(context) & amendment.vocabulary)
            if len(shared) < MIN_SHARED_WORDS:
                advisory.append(Finding(
                    rel_path, idx + 1, cited,
                    "asserts a requirement its target may not discuss",
                    f'indexed title is "{amendment.title}"; shares {len(shared)} '
                    f"distinctive word(s)"
                    + (f" ({', '.join(shared)})" if shared else "")
                    + ". Either the number is wrong, the requirement is in no accepted "
                    "amendment, or the citation is simply terse — a human decides.",
                ))
    return blocking, advisory


def run(root: Path, show_advisory: bool = True, advisory_only: bool = False) -> int:
    index_file = root / INDEX_PATH
    if not index_file.is_file():
        print(f"cannot check amendment references: {INDEX_PATH} is absent", file=sys.stderr)
        return 2

    rows = parse_index(index_file.read_text(encoding="utf-8"))
    if not rows:
        print(f"cannot check amendment references: no rows parsed from {INDEX_PATH}", file=sys.stderr)
        return 2

    corpus = build_corpus(rows, root / SPECS_DIR)
    if not corpus:
        print(
            "cannot check amendment references: no amendment document was readable under "
            f"{SPECS_DIR}; the index lists {len(rows)}",
            file=sys.stderr,
        )
        return 2

    colliding = colliding_numbers(rows)
    unreadable = sorted(set(rows) - set(corpus))
    blocking: list[Finding] = []
    advisory: list[Finding] = []
    citations = 0

    for rel_path in tracked_files(root):
        if is_exempt(rel_path):
            continue
        try:
            text = (root / rel_path).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        citations += len(CITATION_RE.findall(text))
        found_block, found_adv = scan_text(rel_path, text, corpus, rows, colliding)
        blocking.extend(found_block)
        advisory.extend(found_adv)

    # No silent caps: state the scope rather than implying full coverage.
    scope = f"{citations} citation(s) across {len(rows)} indexed amendments"
    if colliding:
        scope += f"; ambiguous numbers: {', '.join(sorted(colliding))}"
    if unreadable:
        scope += f"; unreadable: {', '.join(unreadable)}"

    if blocking:
        print(f"amendment references blocked: {len(blocking)} unresolvable citation(s)")
        for finding in blocking:
            print(f"  {finding.render()}")
    else:
        print(f"amendment references clean: {scope}")

    if advisory and show_advisory:
        print(
            f"\nadvisory — {len(advisory)} requirement citation(s) whose target may not "
            "discuss the subject. This is a heuristic with known false positives "
            "(a terse citation has too little context to confirm); it never changes "
            "the exit status."
        )
        for finding in advisory:
            print(f"  {finding.render()}")

    if blocking:
        print(f"  scanned {scope}")
    return 0 if advisory_only or not blocking else 1


# ---------------------------------------------------------------- self-test

INDEX_FIXTURE = """# Amendment index

| # | Title | Supersedes | Affects |
|---|---|---|---|
| **12** | [Provider-aware song search and provider-independent identity](2026-09-16-amendment-12-provider-aware-song-search.md) | x | y |
| **10** | [First-draft private-LAN transport](2026-09-15-amendment-10-first-draft-lan-http.md) | x | y |
| **10-A11** | [AirSense acceptance scope](2026-09-13-amendment-10-air11-acceptance-scope.md) | x | y |
"""

BODY_12 = (
    "Song search queries a provider catalogue. Provider-independent identity keys the song. "
    "The search returns the catalogue record for the chosen provider."
)
BODY_10 = "The first draft permits plain HTTP to one configured private LAN address only."
BODY_A11 = "AirSense acceptance covers the mask host gates and nothing else. It returns a record."

# Verbatim from crates/vitals-console/src/api/air11.rs — the citation this gate
# was written for. It shares only "record" and "returns" with Amendment 12: the
# first from an unrelated clause, the second from an adjacent line about the
# endpoint's own return value.
REAL_DEFECT_FIXTURE = (
    "/// Returns bounded, one-minute `PostgreSQL` buckets rather than browser-session\n"
    "/// samples. The boundary is deliberately gap-bounded and provisional: Amendment 12\n"
    "/// does not yet provide a durable therapy-session record, so this endpoint must\n"
    "/// not pretend that a transport or time window is a finalized therapy session.\n"
)


def self_test() -> int:
    failures: list[str] = []

    def check(name: str, condition: bool, detail: str = "") -> None:
        if not condition:
            failures.append(f"{name}: {detail}" if detail else name)

    with tempfile.TemporaryDirectory() as tmpname:
        tmp = Path(tmpname)
        specs = tmp / SPECS_DIR
        specs.mkdir(parents=True)
        (specs / "2026-09-16-amendment-12-provider-aware-song-search.md").write_text(BODY_12)
        (specs / "2026-09-15-amendment-10-first-draft-lan-http.md").write_text(BODY_10)
        (specs / "2026-09-13-amendment-10-air11-acceptance-scope.md").write_text(BODY_A11)
        rows = parse_index(INDEX_FIXTURE)
        corpus = build_corpus(rows, specs)
        colliding = colliding_numbers(rows)

        check("three rows parse", len(rows) == 3, f"{sorted(rows)}")
        check("the suffixed series is its own row", "10-A11" in rows, f"{sorted(rows)}")
        check("10 is detected as ambiguous", colliding == frozenset({"10"}), f"{sorted(colliding)}")
        check("12 is not ambiguous", "12" not in colliding, f"{sorted(colliding)}")

        def scan(path: str, text: str) -> tuple[list[Finding], list[Finding]]:
            return scan_text(path, text, corpus, rows, colliding)

        # --- blocking tier -------------------------------------------------
        block, _ = scan("docs/s.md", "Added Amendment 10: live acceptance gates only one path.")
        check("a bare ambiguous citation blocks", len(block) == 1, str(block))
        if block:
            check("the block names both candidates",
                  "10-A11" in block[0].detail and "First-draft" in block[0].detail,
                  block[0].detail)

        ok, _ = scan("docs/s.md", "Amendment 10 permits first-draft private-LAN HTTP.")
        check("title words disambiguate", not ok, str(ok))

        ok_link, _ = scan(
            "docs/s.md",
            "See [Amendment 10](../specs/2026-09-15-amendment-10-first-draft-lan-http.md) for this.",
        )
        check("a link disambiguates", not ok_link, str(ok_link))

        ok_suffix, _ = scan("docs/s.md", "Amendment 10-A11 set the acceptance scope.")
        check("a suffixed citation is already unambiguous", not ok_suffix, str(ok_suffix))

        missing, _ = scan("docs/s.md", "Amendment 13 governs this.")
        check("an unindexed number blocks", len(missing) == 1, str(missing))
        if missing:
            check("and says so", "no row for 13" in missing[0].detail, missing[0].detail)

        # A non-colliding number stays bare without blocking — this is what
        # keeps the gate from becoming a 111-site migration.
        bare, _ = scan("docs/s.md", "See Amendment 12 for background.")
        check("a bare non-colliding citation does not block", not bare, str(bare))

        # --- advisory tier -------------------------------------------------
        _, adv = scan("crates/x/src/air11.rs", REAL_DEFECT_FIXTURE)
        check("the real defect is reported as advisory", len(adv) == 1, str(adv))
        if adv:
            check("it names the indexed title",
                  "Provider-aware song search" in adv[0].detail, adv[0].detail)
            check("it admits terseness as an explanation",
                  "terse" in adv[0].detail, adv[0].detail)

        _, adv_ok = scan(
            "docs/a.md", "The provider catalogue song search identity required by Amendment 12."
        )
        check("a supported requirement is not reported", not adv_ok, str(adv_ok))

        _, adv_bare = scan("docs/a.md", "Therapy session history, see Amendment 12.")
        check("a bare reference is not an advisory either", not adv_bare, str(adv_bare))

        # --- distinctiveness is scale-invariant ----------------------------
        wide = tmp / "wide"
        (wide / SPECS_DIR).mkdir(parents=True)
        rows_wide: dict[str, tuple[str, str]] = {}
        for i in range(1, 9):
            name = f"2026-01-0{i}-amendment-{i}-subject-{i}.md"
            body = f"Subject{i} governs the uniqueword{i} behaviour."
            if i in (1, 2):
                body += " It returns a record of the outcome."
            (wide / SPECS_DIR / name).write_text(body)
            rows_wide[str(i)] = (f"Subject {i} scope", name)
        wide_corpus = build_corpus(rows_wide, wide / SPECS_DIR)
        check("a minority generic word is not distinctive",
              "record" not in wide_corpus["1"].vocabulary,
              f"{sorted(wide_corpus['1'].vocabulary)}")
        check("a unique word is distinctive",
              "uniqueword1" in wide_corpus["1"].vocabulary,
              f"{sorted(wide_corpus['1'].vocabulary)}")
        check("a title word is distinctive even when widely used",
              "subject" in wide_corpus["1"].vocabulary,
              f"{sorted(wide_corpus['1'].vocabulary)}")

        # An index row whose document vanished must drop out, not pass.
        (specs / "2026-09-15-amendment-10-first-draft-lan-http.md").unlink()
        partial = build_corpus(parse_index(INDEX_FIXTURE), specs)
        check("an unreadable amendment leaves the corpus", "10" not in partial,
              f"{sorted(partial)}")
        gone, _ = scan_text(
            "docs/s.md", "Amendment 10 permits first-draft private-LAN HTTP.",
            partial, parse_index(INDEX_FIXTURE), colliding,
        )
        check("and its citations block rather than pass", len(gone) == 1, str(gone))
        if gone:
            check("naming unreadability, not correctness",
                  "not the same as" in gone[0].detail, gone[0].detail)

    with tempfile.TemporaryDirectory() as empty:
        check("an absent index exits 2", run(Path(empty)) == 2, "expected 2")

    with tempfile.TemporaryDirectory() as noneread:
        root = Path(noneread)
        (root / SPECS_DIR).mkdir(parents=True)
        (root / INDEX_PATH).write_text(INDEX_FIXTURE)
        check("an index with no documents exits 2", run(root) == 2, "expected 2")

    check("the index is exempt", is_exempt("docs/specs/AMENDMENTS.md"))
    check("amendment documents are exempt",
          is_exempt("docs/specs/2026-09-16-amendment-12-provider-aware-song-search.md"))
    check("this change's own text is exempt",
          is_exempt("openspec/changes/amendment-reference-integrity/proposal.md"))
    check("ordinary docs are in scope",
          not is_exempt("docs/integration/resmed-air11-ble-stream.md"))

    if failures:
        print("self-test FAILED:")
        for failure in failures:
            print(f"  {failure}")
        return 1
    print("amendment-reference self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true", help="run the built-in fixtures")
    parser.add_argument("--root", default=".", help="repository root to scan")
    parser.add_argument("--advisory", action="store_true", default=True,
                        help="show advisory findings (default)")
    parser.add_argument("--no-advisory", dest="advisory", action="store_false",
                        help="suppress advisory findings")
    parser.add_argument("--advisory-only", action="store_true",
                        help="report everything but always exit 0")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    return run(Path(args.root).resolve(), args.advisory, args.advisory_only)


if __name__ == "__main__":
    sys.exit(main())
