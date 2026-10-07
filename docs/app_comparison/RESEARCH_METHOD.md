# RESEARCH_METHOD — how evidence is gathered, labelled and verified

This is the rule register for the app-comparison package. It governs *how* a fact becomes a finding.
`PROMPT1.md` governs *what* is compared; this file governs whether what you write is true.

---

## 1. Posture

**Inspection is a required step, not a permission.** Every candidate's artifact is obtained and read.
The method below assumes the artifact is in hand; when it is not, that is a task failure to fix in step 2,
not a category of result. The only limits recorded are technical, and each names its mechanism and what
was read in its place (`PROMPT1.md` §2).

**A candidate's findings are only as good as the bytes.** Every quotation is verified against the source
bytes; every hash is computed, never recalled; every count is counted, never remembered.

---

## 2. Sourcing

### 2.1 Source hierarchy

When two sources disagree, the more conservative one governs, and **both are recorded with dates**:

1. The artifact's own behaviour (`[OBSERVED]`)
2. The artifact's own code and configuration (`[STATIC]`)
3. The vendor's contractual instruments — terms, EULA, privacy policy, published spec, credential
   documentation (`[DOCUMENTED]`)
4. The vendor's help centre and documentation (`[DOCUMENTED]`)
5. The vendor's marketing — website, store listings, press (`[DOCUMENTED]`)
6. Independent records — regulator filings, trial registries, peer-reviewed papers, another vendor's
   disclosure (`[THIRD-PARTY]`)

A vendor's own material that undercuts the product's headline is a **finding**, quoted and left
unresolved. Do not explain it away, and do not resolve it in the vendor's favour.

### 2.2 What counts as an independent check

| Artifact | Independent? |
|---|---|
| Regulator clearance record (e.g. an openFDA 510(k) summary) and its **product-code classification** | Yes — and the classification is a *scope* fact distinct from the clearance claim |
| Clinical trial registration (ClinicalTrials.gov, ISRCTN, …) | Yes — and a registry classification outranks the noun the vendor or the paper uses |
| Peer-reviewed paper by authors with **no vendor affiliation** | Yes |
| Peer-reviewed paper with vendor employees among the authors | **No.** Label it vendor-affiliated in the same sentence as the claim |
| A vendor-published preprint, webinar, blog or "research page" | No |
| A vendor's self-reported business figures | No |

A vendor publishing its own numbers does not make them independent. A vendor-authored study is the
counter-surface to the vendor's marketing — record both, unresolved.

### 2.3 Registers that must be checked for certain candidates

Regulated device, clinical claim, or study-backed candidate: check the regulator record, the trial
registry, the publication and **its corrigendum**, and the vendor's own research page. A corrigendum is
read as its own artifact, not as a footnote to the original.

---

## 3. Labelling

Every substantive statement carries exactly one class (`PROMPT1.md` §5): `[DOCUMENTED]`, `[STATIC]`,
`[OBSERVED]`, `[PUBLISHED-CLIENT]`, `[THIRD-PARTY]`, `[INFERRED]`, `[UNKNOWN]`.

Rules that the run has had to learn more than once:

- **Absence of evidence is `[UNKNOWN]`, not absence.** A 404 on a URL the vendor publishes on *both*
  storefronts is evidence of absence for that URL, and is recorded as such. A URL you guessed is not.
- **A documented default the code does not implement is a finding** — not a rounding error in either
  direction.
- **A code comment is not a behaviour.** Neither is a string, a bundled SDK, a feature flag, or dead code.
  Reachability is checked in the class map or the claim stays `[STATIC]`.
- **A vendor's internal document is the vendor's statement**, quoted with its date.
- **A published specification's `example` blocks may be generated rather than captured.** Say which.
- **An undocumented value domain is a finding.** So is a sentinel that means *absent*.
- **Never clamp.** An out-of-range value is rejected and named; a clamped reading is indistinguishable
  from a real one downstream. Quote a vendor doing the same as a positive comparator.
- **Missing data contributes nothing.** Three states always: measured-and-good, measured-and-bad, and
  **not measured**. The third is never rendered, logged or scored as either of the others.
- **A taxonomy fact belongs to its author.** If a rule is the linter's, attribute it to the linter; if a
  classification is the regulator's, attribute it to the regulator.
- **Do not overstate a gap in our favour, or against the vendor.** Every "we do not X" and every "they do
  not Y" is checked against the tree or the artifact before it is written.
- **A refuted hypothesis is not a finding.** It is a correction. Corrections are recorded, prominently,
  where the wrong version lived — never silently rewritten.

---

## 4. Quantitative discipline

- **Never state float equality.** Use a tolerance: `(a - b).abs() < 1e-9`.
- **Fixed reference timestamps.** No wall clock in an assertion.
- **Every number is measured or counted.** A figure written from memory is a defect; a tally a reader
  cannot reproduce from the table above it is worse than no tally.
- **Hash everything you fetch**: SHA-256, recorded with the URL and the retrieval date. An unverified
  hash is worse than no hash. A re-fetch that disagrees with the recorded digest is a **lifecycle finding**,
  not a fetch error.
- **Define a "reference count" by its match mode.** Say what was searched and where, or the count is
  meaningless.
- **Do not average a disagreement.** An intra-vendor count discrepancy is recorded as two numbers.

---

## 5. Quotation verification

Every quotation in every deliverable is verified as a fixed byte string against the source it claims to
come from. Two modes, both required:

| Mode | Corpus | Answers |
|---|---|---|
| **Full** | Vendor artifacts + this package + the v1 tree + the v2 tree | "Is this the vendor's wording?" |
| **Vendor-only** | Vendor artifacts only | "Or is it only the wording of another file in this package?" |

Vendor-only mode exists because a quotation can pass the full check merely because a *sibling deliverable*
reproduces the same reconstructed wording. A quote that verifies only against the package is not verified.

**Triage every miss; never auto-accept it.** Three classes, plus one exception:

1. **Checker artefact** — the corpus is incomplete or the normalisation is wrong. Fix the checker.
2. **Author paraphrase wearing quotation marks** — a real defect. Fix the prose, or quote the source.
3. **Corpus gap** — the source is gone or was never fetched. Re-fetch from the recorded URL, verify the
   digest, and re-run. If it cannot be re-fetched, replace the quotation with attributed prose.
4. **Exception:** this package's own superseded wording, quoted inside a correction record, stays verbatim
   but is rendered in a code span, never in quotation marks.

**The corpus is part of the evidence.** Practical requirements learned the hard way:

- Index **every** text file, not a suffix list: an extensionless vendor dump is corpus. Sniff for text
  (no NUL in the first 4 KB) rather than trusting the extension.
- Strip markup only with well-formed tag syntax (`</?[A-Za-z][^<>]*>`). A bare `<` in prose (as in
  `p < 0.05`) otherwise silently deletes a region of the corpus. A file count is not an integrity check.
- Decode escape sequences in both the fragment and the corpus, for `.js` **and** `.json`; unescape HTML
  entities twice (vendor pages double-escape them); collapse markdown links to their text; strip editorial
  brackets **and their contents**; strip blockquote markers; keep inline-code content but drop the backticks.
- Pair quotes per paragraph and per table row, not greedily across the file.
- State the blind spots: quoted fragments shorter than 25 alphanumeric characters are skipped, and fenced
  code is not checked.
- Never silently repair a word inside a quotation. **Rewording a verbatim vendor quote to silence a
  linter is the defect.**

---

## 6. Inspection modes

For each candidate, record which mode produced the implementation file. The mode is a fact about coverage,
not a verdict about permission.

| Mode | What was read |
|---|---|
| **Decompiled artifact** | The package pulled/fetched and read with `jadx`/`apktool`/`baksmali`, or unpacked `.ipa` and its readable resources |
| **Decompiled artifact + runtime** | The above, plus execution on a controlled device with proxy capture |
| **Published source** | The vendor's own repository at a pinned commit (open-source candidates) |
| **Published contract** | A vendor-published specification (OpenAPI, API reference, data schema), no endpoint called |
| **Published client** | JavaScript/configuration the vendor serves to browsers, read as text |
| **Public record** | Regulator filing, trial registry, publication, and store panels, with no artifact available |

"Modes" combine: a candidate may be *decompiled artifact + published contract + public record*. Name each
surface in the file, and name what each surface did **not** cover.

---

## 7. Scratch and evidence retention

Working files (fetched pages, APKs, decompiled trees, capture logs, quote-checker runs) live **outside the
repository** in a per-candidate scratch directory, and are not committed. What is committed is the
**record**: URLs, retrieval dates, byte sizes and SHA-256 digests in the deliverable itself.

Consequences to plan for: a scratch tree can be cleared by the operating system without warning. A
deliverable that cites only a scratch path has no evidence. If a re-fetch is needed, it is a **check** —
compare the digest, and treat a disagreement as a lifecycle finding.
