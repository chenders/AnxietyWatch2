# App comparison — research method

**Research date:** 2026-10-06 (session started 2026-10-05 ~17:41 PDT)
**Execution mode:** full sequential — the whole shortlist (10 candidates), one processed end to end at a
time. The pilot (`bearable`) was reviewed by the owner and the run was authorised to continue. **The
shortlist is exhausted: 10 of 10 complete.**
**Governs:** `PROMPT1.md`, the five `PROMPT2*.md` variants, `CANDIDATE_SHORTLIST.md`,
`PROCESS_STATE.md`, and every `{APP_SLUG}/` candidate directory

---

## 1. Research date, model and tools

| Field | Value |
|---|---|
| Research date | 2026-10-06 |
| Model | `deepseek/deepseek-v4-flash` via provider `litellm` (pi coding agent, reasoning level high) |
| Session id | `01a10ea8-4e0e-763c-8212-3009e888c2d2` |
| Local repository | `AnxietyWatch2` (this repo), branch `fix/silence-detector-schema-preflight` |
| Comparison baseline repo | `../AnxietyWatch` (v1) |
| Web retrieval | `curl` against vendor sites, store listings and support articles; Bing and Mojeek for search |
| Traffic inspection tooling (validated, not used against a target app) | mitmproxy/mitmdump **12.2.3** (Python 3.14.4, OpenSSL 3.5.5) |
| Device tooling (validated, not used against a target app) | Android `adb` 1.0.41, Android Emulator 37.2.12.0, AVD `pixel_4_w_gapps` (Android 12 / SDK 31) |
| Packet tooling | TShark (Wireshark) 4.6.5; `tcpdump`; `python3` |
| PDF tooling | `pdftotext`, `pdfinfo`, `pdffonts` and `qpdf` (homebrew poppler/qpdf). **No OCR engine is installed**, which is what makes the `empatica` manual gap a blocker rather than an inconvenience |
| Regulator data | openFDA `device/510k.json` and `device/classification.json`; 510(k) summary PDFs at `accessdata.fda.gov/cdrh_docs/pdf<YY>/K######.pdf`; the Apple lookup API (`bundleId=` form) |
| Android SDK root | `$ANDROID_HOME` (the run used the machine's default Android SDK location) |
| Published-client analysis (used for `mindshift`) | `curl` plus `grep -o -F` for fixed-string counts, `grep -o -E` for route extraction, a small Python extractor for the bundle's string table, and `shasum -a 256` for every artifact. **Every count is reported with its match mode**, because a case-sensitive zero and a case-insensitive zero are different findings |
| Quote verification | A purpose-written checker per candidate (`/tmp/aw2research/{slug}/quotecheck.py`) that machine-compares every quoted span in a deliverable against the retrieved bytes. The corpus is the **fetched artifact tree for that candidate** (`.html`, `.js`, `.json`, `.xml`, `.txt` — and `.py`/`.toml`/`.yaml`/`.yml` where the candidate is source-inspected), the package docs, the V2 tree and the V1 tree, and **the file under check is excluded from its own corpus**. HTML/XML is tag-stripped, JS `\uXXXX` escapes are unescaped, elided quotes are split on `…`/`...`, and pairing resets at every blank line and table row. **A quote-verification sweep is a required pre-commit step** — see §6.1. **Corpus completeness is the check's weak point**: a quote from a vendor page that is not in the artifact tree, and a quote from a `.py` source file when the tree is not indexed for source, both read as MISS for a reason that has nothing to do with the quote. Extend the corpus before triaging a MISS list |

**Model caveat.** Model- and provider-rendered web summaries were treated as **leads only**. Every
material factual claim in a candidate file was traced to a retrieved primary page or a peer-reviewed
record, and the retrieval is dated in the evidence table. Where a page could not be retrieved, the
failure is recorded rather than papered over (see §7).

## 2. Candidate-selection rules

The shortlist in `CANDIDATE_SHORTLIST.md` was built from the AnxietyWatch capability inventory rather
than from app-store browsing, so that candidates were selected for *capability overlap with a stated
project capability* rather than for popularity.

Rules applied:

1. **Start from the capability inventory.** C1–C26 were derived from AnxietyWatch v1 documentation and
   source and from `openspec/` specifications, each labelled V1 documented/implemented, V2 current,
   V2 specified target, or Unresolved.
2. **Select for overlap, not fame.** A candidate qualifies if it shares at least one mechanism with the
   inventory — subjective logging, correlation, medication follow-up, CPAP, oximetry, HRV, alarm,
   self-hosting, clinical reporting, environmental context.
3. **Score on six dimensions (A–F), 0–3 each:**
   A whole-product relevance to v1 · B subsystem overlap · C architectural contrast ·
   D privacy/hosting contrast · E evidence availability · F differentiation value.
4. **Separate score from rank.** The ranking is a *review order*, not a quality order. `bearable`
   ranks first on dimension A despite not having the highest total score; `nightscout` has the highest
   total (15) and ranks sixth. This is deliberate: the pilot needs the candidate that most closely
   mirrors the v1 whole product, not the one that scores highest on a composite.
5. **Reject with a reason.** Seven alternatives-considered-and-rejected candidates are listed with
   reasons. A candidate is rejected for being a duplicate category, for having no retrievable public
   evidence, or for being a component rather than a product.
6. **Reachability check.** A candidate must have a resolvable public presence. `emoods` was dropped
   because `emoods.net` had no DNS A record at research time.

## 3. Search boundaries and assumptions

- **Search surfaces:** vendor product sites and support/knowledge bases, Apple App Store listings,
  Google Play listings, vendor privacy policies and terms, public documentation, peer-reviewed
  literature (via Crossref for metadata when publishers block direct access), and the projects' own
  repositories for the open-source candidates.
- **Geographic assumption:** **US storefront** for App Store and Play Store metadata and pricing.
  Prices, plan names and availability differ by region and are not covered here.
- **Platform assumption:** iOS and Android are treated as the relevant surfaces. macOS via the
  Catalyst/iOS app is noted where the vendor states it; no native desktop build was assumed.
- **Time assumption:** all findings are a snapshot dated 2026-10-06. Freemium products change tiers
  frequently; a tier claim older than the snapshot is dated in the evidence table and must be rechecked
  before being relied on.
- **Language:** English-language sources only.
- **Deliberate exclusion:** no account was created anywhere, no production service was logged into, no
  paid tier was purchased, and therefore **no premium-gated behaviour is verified** anywhere in this
  research. This is a standing coverage gap, not a per-candidate accident.
- **Anthropic/Claude is not used** for this research, per the owner's provider preference. The v1
  application's *product feature* of optional Claude analysis is reported as a feature of the product
  under comparison; that is not a use of Anthropic tooling by the researcher.

## 4. Source-quality rules

Every material statement carries an evidence class:

| Class | Definition |
|---|---|
| **Public claim** | What the vendor says in marketing, store copy or a listing |
| **Documented behaviour** | Official documentation specifying the behaviour in operational detail |
| **Observed / independently reported** | Credible third-party or end-user report; labelled as such, never merged with vendor documentation |
| **Static-only finding** | Implementation evidence not observed at runtime |
| **Inference** | A labelled deduction, never presented as fact |
| **Unknown / unverified** | Cannot be established with available access |
| **Not supported** | A vendor-stated limitation, or a documented absence across all public surfaces examined |

Hierarchy of preference: peer-reviewed record > primary legal document (privacy policy, terms) >
vendor operational documentation > platform-declared metadata (store data-safety, entitlements) >
vendor marketing copy > third-party or user report > search-result snippet.

Rules:

1. **No claim without a source.** Every row in a candidate's evidence table carries URL, publisher,
   date, access date and a confidence label.
2. **Absence of evidence is unknown, not proof of absence.** A finding that no CPAP feature appears
   anywhere examined is recorded as absence-of-evidence and explicitly flagged as *not* proof that no CPAP
   code exists in the binary.
3. **Marketing is not evidence of behaviour.** "over 2M patients & doctors" is labelled unverifiable.
4. **Do not extrapolate across features.** A sleep score is not CPAP support; an AI chatbot is not
   longitudinal multimodal analysis; cloud sync is not self-hosting; a notification is not an
   independent local alarm.
5. **Quote the vendor's most conservative statement when statements conflict in strictness.** Bearable
   markets medication correlation and then warns users not to make medication decisions from it; the
   warning is the operative claim.
6. **Separate website facts from app facts.** Bearable's website loading Google Tag Manager is a
   *website* observation and must not be recorded as app telemetry.
7. **Record conflicts rather than resolving them silently.** The Bearable privacy policy's "no
   third-party sharing" versus the Play Store data-safety declaration's named sharing is recorded as
   an open question (Q7), not smoothed over.
8. **Peer-reviewed scope discipline.** A systematic review that *includes* a product is not evidence
   that the product works. The Bearable citation is recorded as evidence of inclusion in an oncology
   PRO review, with its scope stated.
9. **Never state float equality**, and never convert correlation into causation, in any report.
10. **Medical discipline.** No product is described as a validated diagnostic, medical device or
    emergency monitor unless the source explicitly supports it. Where a project's own documentation
    disclaims this, the disclaimer is reproduced.

## 5. AnxietyWatch status labelling

Three products exist in this comparison and are never conflated:

- **V1 documented/implemented** — represented as implemented by AnxietyWatch v1 code or documentation.
- **V2 current** — present and usable in the current AnxietyWatch2 repository.
- **V2 specified target** — fully specified in `openspec/`, not necessarily implemented.
- **Unresolved** — mentioned but insufficiently specified or evidenced.

The rule that matters most: **a V2 specified target is never presented as a shipped feature.** The
detector and the supervised alarm do not exist, and every report says so where relevant. The `[v2c]`
label is also used conservatively — the project's own README states that the TimescaleDB hypertable
chain is not claimed as verified-deployed, and the reports reflect that rather than implying a
production system.

Each conclusion also states its scope: the v1 app, the v2 Pi platform, the combined vision, or a single
subsystem.

## 6. Authorization rules for package and protocol inspection

Phase 2A permits implementation inspection only under these limits:

**Permitted**
- Published source of open-source candidates.
- A vendor-distributed build of a vendor-distributed app, where installing it is authorized.
- An artifact extracted from a device the owner controls and authorizes.
- An emulator that is disposable, non-personal, and authorized — for example an AVD.
- Installing the mitmproxy CA on such a **disposable, authorized emulator only**, and only to observe
  an app that accepts user-installed CAs.
- **Vendor-published surfaces.** Anything the vendor itself publishes for public use, including a public
  API reference, published datasets or research artifacts, an open-source component or GitHub
  organisation, a public roadmap, and self-published business figures. These require no owner decision,
  because they are public and published for third parties to read.

**Prohibited — not attempted at any point**
- Third-party APK or IPA mirrors. Redistribution is not authorization to inspect, and a mirror cannot
  grant rights the vendor has not.
- Defeating certificate pinning, anti-debugging, obfuscation, DRM or licence checks.
- Circumventing platform protections (Android SafetyNet/Play Integrity, iOS integrity checks).
- Extracting credentials, tokens, keys, personal data, or third-party private data.
- Installing a CA on a personal or production device.
- Creating accounts on target apps or on production health services.
- Sending real user data to a candidate's servers.
- Logging into production services.

**Required handling when inspection is blocked:** record it as an *evidence limitation*, in a dedicated
implementation-inspection file, with the reason, the artifact fields marked not-acquired, and a
statement of what cannot be concluded. Never treat blocked inspection as proof that a feature does or
does not exist, and never substitute marketing copy for verified behaviour.

**Free tooling substitution.** The workflow uses mitmproxy/mitmweb on `127.0.0.1:8080` with the
emulator proxied at `10.0.2.2:8080`, per `FREE-ANDROID-TRAFFIC-INSPECTION.md`. This replaces the
previously documented Charles Proxy workflow (`10.0.2.2:8888`) because the Charles licence expired; the
replacement is free and already installed. The proxy must be configured **before** the target app is
launched or installed, and removed afterwards with the removal verified.

**Encrypted-traffic honesty rule.** A hostname or IP observed without decryption is recorded as
*encrypted traffic to host X observed*, never as *the app called API Y*. Packet-capture fallback uses
the emulator's `-tcpdump` and TShark rather than defeating protections.

**Sequencing rule.** Exactly one candidate is active at a time. No cross-candidate parallelism and no
prefetching a later candidate's research while an earlier one is in progress.

### 6.1 Inspection modes actually used

Five modes have been reached, and the count is stated against the table below rather than from memory.
Which one applies is recorded per candidate, and none is ever silently
upgraded into another.

| Mode | Meaning | Candidates |
|---|---|---|
| **Not performed** | No artifact of any kind was lawfully obtainable | `bearable` |
| **Partially performed** | No product binary, but authorized vendor-published artifacts were inspected | `welltory`, `exist`, `sleephq`, `oura` |
| **Performed** | Product source or a lawfully authorized build was inspected | `oscar` (GPLv3 source, pinned commit); `nightscout` (AGPL-3.0 source, pinned commit); `homeassistant` (Apache-2.0 source, tag `2026.9.4` / commit `9212531f…`, eleven files read and hashed) |
| **Performed — artifact and regulator record** | Neither binary nor source nor API was obtainable. The strongest available surfaces are the vendor's own **contractual and compliance documents** and an **independent regulator's record of the vendor's own submission** | `empatica` |
| **Performed — published client and public records** | Neither binary nor source nor API contract was obtainable. The strongest available surfaces are the vendor's own **served web client** read as text, plus **storefront metadata** and an **independent clinical registry and its peer-reviewed publication** | `mindshift` |

**Vendor-published artifacts differ in strength, and the difference must be stated.** Three distinguishable
strengths have been used:

- **Corroborating artifacts** (`welltory`) — CC0 datasets and research repositories that provide evidence
  *about* the vendor's claims, from outside the product. A dataset is not the product, so these corroborate
  and cannot confirm.
- **Implementing artifacts** (`exist`, `sleephq`) — a public API reference that documents the product's own output
  contract. The correlation record, the insight record, the attribute catalogue and the sparse value model
  are the analysis layer's interface, so these are genuine static findings *of* the implementation.
  `sleephq` extends the mode: its OpenAPI document is complete enough to enumerate every path, every
  serializer attribute and one hash algorithm exactly (`content_hash` = MD5 of content+name, with a worked
  example and a validation endpoint), and candid enough to document its own rate-limit buckets.
  **`oura` extends this mode in a third direction, and adds the first case where the *contract itself* is
  the reason a stronger mode was not attempted.** Oura publishes a complete OpenAPI 3.1 document (72
  paths, 75 schemas, 274 resolvable `$ref`s) under a **public static path**, and its own documentation page
  is a JavaScript shell that merely points at it. Two consequences of method:

  - **The document is large enough to support internal-consistency checking as a first-class step.**
    Reference resolution, security-scheme declaration-versus-use, path-to-schema coverage, the
    `required`/nullable pattern of every collection model, and the complete set of bare-`type: string`
    schemas can all be enumerated and counted. That is how the package's first **verified positive** on a
    vendor contract was produced — and how a plausible dangling-reference hypothesis was **refuted**
    rather than reported.
  - **A vendor's terms can close the stronger mode by contract rather than by availability.** For
    `bearable` the blocker was lawful acquisition; for `empatica` it was the absence of any published
    client. For Oura the terms **prohibit** reverse engineering and defeating subscription restrictions,
    and those clauses are quoted **verbatim** as the authorization basis. That is a third, distinct
    reason for a limited mode and is recorded as such.

  **The limit is sharper here than anywhere else in the package**: a documented contract that is internally
  pristine can still describe behaviour nothing was observed to perform, and **no Oura endpoint — not even
  the sandbox — was called.** Every contract finding is labelled *documented*, never *verified*.

- **The implementation itself** (`oscar`, `nightscout`) — the complete product source under a licence that
  requires publication. This is the strongest static surface available short of running the software, and
  it is qualitatively different from the other two: every claim about how a value is decoded, rejected,
  stored and converted can be read at a named file and line, with no inference from a contract or a
  dataset.

  **`nightscout` extends this mode in one direction `oscar` could not.** Because the project also publishes
  documentation *about* its own implementation — system audits, formal requirements, test specifications,
  an OpenAPI description — the implementation could be checked **against the vendor's description of
  itself**, at a pinned commit that is also the release commit. That produced a class of finding neither
  of the other two modes can produce: **doc/code drift inside one release**, where the
  vendor's security audit, its API description and its changelog each disagree with the code they ship
  with, while the requirements document beside the audit agrees with it. The same mode also produced the
  opposite result twice, which matters just as much: a **sanitisation claim checked and found accurate**,
  and a `TODO` claiming a fallback was broken that was **checked and refuted** — the comment was stale.
  Had it been trusted, the package would have recorded a false defect and missed the real one.

  The limit of this mode is unchanged and must be stated: **a vendor's self-audit is a claim about the
  product, not validation of it.** Nightscout's `docs/audits/*` and `docs/requirements/*` are recorded as
  the project's own statements, dated, and where they disagree with each other or with the code, the
  disagreement is recorded and neither surface is preferred.

**A fourth mode, instantiated by `empatica`.** The three modes above all assume an implementation surface —
source, a published API, or a runnable binary. Empatica publishes **none of the three**: its GitHub
organisation holds five public repos and only one is original (a documentation site, since emptied); the
Cloud API documentation is supplied to clients only; the app is prescription-gated and iOS-only in practice,
with no Android artifact located. What it *does* publish is a **regulated product's paperwork**, and what
exists independently is **the regulator's own record of the vendor's own submission**. That combination is a
stronger evidence class than marketing and a weaker one than source, so it is recorded as its own mode
rather than folded into "partially performed".

Two artifact classes are new to the package and are now first-class evidence surfaces:

- **The regulator's record — queried directly, never inferred.** openFDA `device/510k.json` for every
  clearance held by the applicant and `device/classification.json` for the **product-code definition read
  verbatim**, plus the **510(k) summary PDF** itself. Four things this yields that no other source can: the
  clearance **dates and applicants**; the **metric the regulator accepted** (here PPA and false-alarm rate per
  24 h, not accuracy); the **method** by which the shipped configuration was justified (bench equivalence to
  a predicate, plus retrospective analyses of data collected on *different* hardware); and the vendor's own
  sentence that "**No clinical studies have been performed on the subject device**". **A product code is a
  scope fact distinct from the fact of clearance** — the POS definition states the device "does not predict
  seizure onsets, and is not intended as a standalone seizure monitoring device", which narrows the claim in
  the regulator's language. Never interpret a product code without querying the classification API.
- **The contract — a recordable counter-surface to marketing.** The EULA is quoted as a *contract*, a
  different instrument class from a marketing page, and the disagreement between the two is recorded
  unresolved and without any implication of bad faith. The same applies to a published compliance artifact
  (an ISO 13485 certificate carrying the standard statement that it is not proof of conformity with
  Regulation (UE) 2017/745) appearing next to a "CE Class IIa" claim: **record placement, not impropriety**.

**The limit of this mode is the sharpest of the five, and it must be stated wherever it is used:**
**no runtime behaviour of any kind is observable.** Alert latency, cancellation-window length, out-of-range
handling, lapsed-subscription behaviour and sensor-failure behaviour are all `[UNKNOWN]`, and hypotheses
about them are recorded as **open questions**, never as findings. This is the mode where the
check-before-recording rule costs the most: where an implementation exists, hypotheses get tested and
refuted; where one does not, they get recorded as gaps and nothing more.

**An artifact that exists and cannot be read is a finding, and it blocks reconciliation.** Empatica's
178-page, 112.8 MB User Manual was retrieved in full and has **no fonts and no text layer** (`pdffonts`
returns an empty table; `pdftotext` returns 178 bytes across 178 pages; every page is a raster image). It is
simultaneously an **accessibility** finding and a **verifiability** finding, and no further fetch can close
it. If OCR is ever attempted it must be recorded with the tool, its version and the pages read, and the
output labelled **OCR text, not the vendor's verbatim text**.

The **documented-contract mode has a second, weaker limit that must be stated separately from the first**.
A specification can lack the very thing a reader needs: `sleephq` publishes fields whose *types* are known
and whose *value domains and methods* are not (`status`, `progress`, `usage`, `omitted`, `teams_type`,
`o2_score`, the `duration_90`/`drop_90`/`drop_3`/`drop_4` family, and every `_95` percentile). Discovering
an undefined value domain is a finding, and it is not the same finding as an absent feature. Two further
patterns from `sleephq` are now standing checks: a documented field that is **write-only**
(`active_energy_joules` is accepted by the journal write operations and absent from the journal
serializer), and a specification whose `example` blocks are **generated at fetch time** rather than
captured from a real record (its machine example carries filler values and a timestamp equal to the fetch
moment).

**A fifth mode, instantiated by `mindshift`, inverts the assumption the other four share.** Every mode above
assumes the vendor ships *something* — source, a specification, a binary, regulated paperwork. MindShift ships
an app nobody here may lawfully obtain, publishes no source, and publishes **no API contract at all**. What it
*does* ship to every anonymous visitor is a **741,896-byte JavaScript client**, and what exists independently is an
**independent clinical registry record** of a study the vendor cites everywhere. So the primary evidence
surfaces are, in order of strength: the **published client**, the **registry and the paper**, and the
**storefront metadata**. Three rules make this mode usable, and all three are now standing sub-checks:

- **A corpus of JavaScript served to the browser is a *published client* — a static artifact, neither a binary
  nor a runtime.** Findings from it are labelled `[FOUND IN PUBLISHED CLIENT]`, and the label is not optional.
  It establishes what the code is **written to do**; it does not establish that any endpoint is live, that any
  request was answered, or that any record was stored. This is the class in which the candidate's
  first-party-analytics finding is recorded, and it is stated exactly at that level and no further.
- **A string in the served HTML shell is not a string in the bundle.** `i.replit.com/script.js` appears in
  **every** served document — including both 404 pages — and **0** times in the 741,896-byte client. The
  attribution must name the **shell**, never the client. The same trap applies in reverse to the 404 copy,
  which exists as two distinct surfaces (the served document's `Page not found`, and a separately authored
  client string that the served document does not contain).
- **A reference count is meaningless without its match mode.** `Sentry` case-sensitive is **20**; `sentry`
  case-insensitive is **129**. `Clerk` is **44**; `clerk` is **505**. Every zero in this package is reported
  with the mode that produced it, and a zero from a case-sensitive count is **never** presented as an absence
  of the concept.

**A five-way chain is what makes an absence verified rather than asserted.** Both storefronts declare
`https://mindshiftcbt.com/privacy/` as the privacy-policy URL. That URL returns a genuine HTTP 404, and the
absence is established by **five independent methods**, not by the one probe that happens to be convenient:
(1) the probe, with three control paths returning 200 to prove the fetch is not broken; (2) the site's own
**9**-URL sitemap; (3) the **13**-path SPA router table; (4) a fixed-string sweep of the whole bundle; and
(5) a same-day re-fetch returning **byte-identical** content. **No one of these alone suffices** — a sitemap
can omit a real page, a bundle can build a URL dynamically, and a single 404 can be transient. It is the
conjunction that closes it. This is the strongest verified absence in the package, and the method is now the
standard for any future question of the form *the vendor declares an artifact and the artifact is not there*.

**The vendor's own paper is a first-class evidence surface, and it is the counter-surface to the vendor's own
marketing.** Four rules:

- **A vendor-funded or vendor-authored study is not independent validation**, and the funder's role is quoted
  from the paper's own competing-interest disclosure — not inferred, and not softened. `mindshift`'s paper
  states that its lead author "reports financial support was provided by Anxiety Canada", which is the app's
  original co-developer. **A vendor's own funded, peer-reviewed study, set beside the vendor's own marketing,
  is an intra-vendor conflict** — the sharpest class in the package — and it is recorded with both sides
  quoted and left unresolved, without any implication of misconduct.
- **An independent registry outranks the vendor's own noun.** `studyType: OBSERVATIONAL` with
  `observationalModel: ECOLOGIC_OR_COMMUNITY` and no arms/interventions module is the authoritative descriptor,
  even though the paper's title says "trial" and the vendor's site says "clinical trial". Both surfaces are
  quoted and the conflict is left unresolved.
- **A registered primary-outcome count that differs from the reported count is a sharper, separate finding
  than the study-type noun.** The registry lists **one** primary outcome; the paper says "four primary
  outcomes". Both quoted verbatim.
- **A headline sample size and a verified-install sample size are not the same number, and the difference is a
  finding.** The paper restricts its analysis to the 156 participants whose study code entry "verified that
  they downloaded the smartphone app", so **224 of 380** never demonstrably installed the product, while every
  marketing surface carries 380. **A corrigendum is a separate artifact and is read, not assumed** — and where
  it corrects a measure without restating that measure's effect size, the effect size is `[UNKNOWN]`, not
  carried over.

**An inspection mode is not an observation.** *Performed* means the implementation was **read**. `oscar`
was never executed: no binary was installed, no data imported, no report generated, and no CSV inspected
as output. `nightscout` was never built, deployed or contacted: no Node process was started, no MongoDB
instance was created, no instance — public or private — was requested, and no token, account or key was
created. **Installing and running a candidate is a separate authorization decision from reading its
published source**, and it has not been taken for either candidate. Static findings and runtime
observations remain separate classes throughout the `oscar` and `nightscout` packages, and the
runtime-observation count for both is zero.

A third surface, used for `exist` and `sleephq`, is also worth checking on every remaining candidate: **self-published
business figures** (revenue, paid-user counts). Where present, these are the only scale evidence on the
shortlist that is neither a download count nor a marketing claim — but they remain vendor-published, and
are never treated as independent. For `oscar` the equivalent check produced **no** scale figures at all:
star and fork counts are not user counts, and none were used as such. **`sleephq` is the case where this
surface is present but weaker**: the only scale figure is the vendor's own "over 100,000 patients in over
160 countries", against 10K+ Android installs, with no revenue published — recorded as vendor-published
with the platform mix left as an inference rather than a reconciliation.

**What vendor-published artifacts cannot do.** They reveal a documented contract, never observed
behaviour. An API reference is not a live system; a dataset is not an accuracy test of a product; a
published policy is not an implementation. No vendor endpoint is called without an explicit authorization
decision, and every such finding is labelled *documented*, never *verified* or *working*.

**Standing sub-checks, added as candidates taught them.** Each is now applied to every remaining
candidate, and each has already produced a finding at least once:

| Sub-check | What it looks for | First produced by |
|---|---|---|
| **Undocumented value domain** | A typed field with no enumeration, units, range or description is *unusable*, and that is a finding in its own right | `sleephq` |
| **Write-only field** | A documented field accepted on write and absent from the read model | `sleephq` |
| **Generated `example` blocks** | Specification examples that are filler or timestamped at fetch time, not captured records | `sleephq` |
| **Vendor material undercutting the headline** | The vendor's own document arguing against its own product claim — recorded, never resolved either way | `oscar`, `sleephq` |
| **Two vendor surfaces in conflict** | The most conservative statement governs; the conflict is recorded, not resolved | `bearable`, `sleephq` |
| **Conflict internal to one docs site** | The same rule applied to two pages of a single vendor's site | `nightscout` |
| **Honest boundary statement** | A limit described as a limit — recorded as a positive, not as a missing feature | `sleephq`, `nightscout` |
| **Documented default the code does not implement** | Divergence between a published default and the implementation — a finding independent of which is safer | `nightscout` |
| **Display unit reaching a derived statistic** | A shipped computed value whose result depends on a user display preference | `nightscout` |
| **Falsified hypothesis** | A hypothesis checked against the artifact and refuted is **not** recorded as a finding | `nightscout` |
| **Two answers in one source tree** | Contradictory behaviours for the same operation in one process — record both and name which path a caller takes | `nightscout` |
| **Independent regulator artifact** | For a regulated candidate, query openFDA's 510(k) records **and** the product-code classification before characterising any regulatory claim; quote the regulation text verbatim. A product code is a scope fact distinct from the fact of clearance | `empatica` |
| **Contract as counter-surface** | A vendor's EULA or terms quoted as a *contract*, a different instrument class from marketing; the disagreement is recorded unresolved and without implying bad faith | `empatica` |
| **Compliance artifact disclaiming the adjacent claim** | A published certificate or audit placed beside a claim it does not bear — record placement, not impropriety; every notified body writes the MDR disclaimer | `empatica` |
| **Mislabelled link** | A hyperlink whose text names one document and whose target is another — a directly observable documentation defect | `empatica` |
| **Duplicated line in a published list** | A vendor's own published list repeating an entry — record the duplicate as observed, without interpreting the lifecycle of the standard | `empatica` |
| **Absent item in a non-exhaustive published list** | Record the omission as an **observation** with the reason it is not a non-compliance finding (list not claimed exhaustive; the standard governs a different object than the product's path) | `empatica` |
| **Artifact offered as evidence for a claim** | Whenever a vendor supplies a document *as* the support for a claim, read the document and test whether it supports **that** claim — the certificate, the cited 510(k) numbers and the instruction manual each failed this test for `empatica` | `empatica` |
| **Undefined term in a decisive table** | An undefined term in the table carrying the performance claim (`corrected PPA`, materially lower than the raw values) — the undocumented-domain rule applied to a regulated document | `empatica` |
| **Documentation lifecycle with an exact date** | A vendor deleting its own developer documentation (`developer.empatica.com` emptied 2025-02-18, four days after the 2025-02-14 E4 sunset) — recorded as a lifecycle fact, not a criticism | `empatica` |
| **Image-only artifact** | A published document with no text layer — simultaneously an accessibility and a verifiability finding, and an evidence gap no further fetch can close | `empatica` |
| **Unverified hash** | A hash table row is **computed or marked "not hashed"**; a plausible placeholder is worse than no value. A fabricated placeholder written and then replaced by computed SHA-256 values is recorded inside the `empatica` artifacts | `empatica` |
| **Field-class sweep belongs to the FIRST pass** | Before any conclusion is drafted, enumerate the artifact's **structured classes**: enumerations, `required` lists, nullable unions, bare-`type` schemas, time formats, character-encoded fields, and any pair of fields with near-identical descriptions. `oura`'s first pass read prose and skipped this; a later sweep reversed **five verdicts**, softened a sixth, and produced nine new findings — a verification pass that *reverses* the draft's direction is recorded against the **research process**, not the vendor | `oura` |
| **Quote verification against fetched bytes** | A **separate, required, pre-commit** step: machine-compare every quoted span against the retrieved artifacts, parsing specifications as **JSON string leaves** so `\u`-escapes cannot defeat matching, and HTML both raw and tag-stripped. Nine quotations that had survived two value-checking passes were found inexact — including a **contract clause cited as an authorization basis** whose paraphrase invented verbs. **Paraphrase inside quotation marks is the failure mode this package exists to avoid** | `oura` |
| **Every summary tally is counted, not estimated** | A verdict tally, a claim count or a "N findings" line is reproducible **from the table it summarises**. A wrong tally is corrected in place and the correction is visible | `oura` |
| **Intra-document range conflict** | One document stating one quantity's valid range three ways (global rule `0-100`; `[1, 100]` on a headline field; `[0, 100]` on contributors; **unstated** elsewhere) — record whether `0` is valid as contradictory within the vendor's own text | `oura` |
| **Disagreeing character-encoded alphabets** | Several digit-string fields in one contract whose characters mean different things, where **only one can encode absence** (`class_5_min` `'0'` = non-wear vs sleep-phase/movement strings with no such code). The non-wear code, the measured `non_wear_time` duration and the nullable score are recorded as **positives** | `oura` |
| **Retrieval failure is not absence** | An error shell (JS `__next_error__`), a WAF challenge or a 5xx is **`[UNKNOWN]`, not a 404**. Only a vendor-authored not-found document is a statement that a resource is absent — and two URLs returning byte-identical 404 documents is how that was confirmed | `oura` |
| **Record-provenance trio** | Look for a tombstone value, a retained-rejection-with-reason value, and a per-record algorithm/method version in the same model. Present in `oura`; the honest framing is that **both systems implement V2's rules, and V2 enforces them where Oura documents them** — never equivalence | `oura` |
| **Documented dead field** | A field whose description says it is deprecated and returns null is a **trap the documentation disarms** — record it as documented, and do not count it as an undocumented domain | `oura` |
| **Documented default the envelope contradicts** | A response documented as one shape and declared as `anyOf` with no selection criterion — a contract ambiguity, distinct from an undocumented value | `oura` |
| **Lifecycle fact, exact date** | A contract published but **not yet effective** at the research date (Last Updated 2026-10-02, Effective 2026-11-04, research date 2026-10-06) is a lifecycle fact, **not a defect** | `oura` |
| **Statistic rendered as a headline in a different category** | An `r²` (or an agreement statistic) published as an "Accuracy %" — record **verbatim with both surfaces quoted**, name the basis, and do not resolve | `oura` |
| **Evidence-classification defect** | A gallery or heading asserting independence (`Independently validated sensors`) that contains a study whose lead author was the vendor's own staff — a *classification* defect, quoted from both surfaces, with the distinction already made correctly by the vendor elsewhere | `oura` |
| **Internal org metadata in a public contract** | A published specification carrying internal team names (`x-owner` squad names) — observable and recordable, **not** impropriety | `oura` |
| **Intra-vendor count discrepancy** | The same quantity stated twice by one vendor across its own surfaces (`over 30 biometrics` vs `over 50 health metrics`) — record without averaging | `oura` |
| **Multi-entity surface mapping** | Map which legal entity contracts on which surface (four entities across five surfaces) — record; do not infer intent from the spread | `oura` |
| **Three-state model verified at every layer** | A project having the absent states in its **core** says nothing about the layers downstream. For each layer, ask separately whether it *branches* on absence: `homeassistant` has `unknown`/`unavailable` as first-class states, and its alert path nevertheless treats a sensor going offline as a **cleared condition** and sends the resolution message (`alert/entity.py:98,115-126`). Record the layer, not the platform | `homeassistant` |
| **Nested idiom contrast in one file** | The same operation spelled two ways a few lines apart — here the invalid Python-2 `except ValueError, TypeError:` at `sensor/recorder.py:239` and the correct `with suppress(ValueError, TypeError):` at `:244` — is why a broken line reads as a **typo** rather than a deliberate choice, and it is recorded without any inference about intent | `homeassistant` |
| **Non-parsing shipped file: verify the bytes, bound the consequence** | Confirm identical bytes across the checkout, the git object database, the upstream raw URL at the tag **and** the default branch before recording; then state the runtime consequence as **inference**. A **tension with CI left unexplained is recorded as unresolved** (`homeassistant` gap G14), never guessed at and never used rhetorically | `homeassistant` |
| **Lint config attributed to the right linter** | A `"syntax-error"` entry in a repository's lint configuration is not a reason for a *different* linter to pass: `homeassistant` selects ruff rule `E` with no per-file ignore for the path (`pyproject.toml:658,647,862-879`) while the only `"syntax-error"` entry sits in the separate **pylint** section (`:214`). Attribute the config fact to the tool that owns it | `homeassistant` |
| **Is the failing code tested?** | `grep` the function name in the project's own test file. `homeassistant`'s `_entity_history_to_float_and_state` is referenced by **no** test in the 56-test file that imports its module. That the function is **untested** is verified; that this is *why* the error survived is inference and is not asserted | `homeassistant` |
| **Positive comparators recorded for the vendor** | A comparison that lists only differences is not a comparison. Record where the vendor independently reached the same conclusion as V2, and where its design is ahead: `homeassistant`'s explicit absent-state substitution instead of clamping, its **named blocker** on an un-acknowledgeable alert, its completion message armed only after a message was sent, its `mean_weight` coverage weighting and its **circular-mean** helpers are all recorded as positives | `homeassistant` |
| **A refuted shortlist hypothesis is a correction, not a footnote** | When the inspection disproves the hypothesis the shortlist used to justify the candidate, record the correction in the candidate deliverables **and** in `PROCESS_STATE.md` **and** in `CANDIDATE_SHORTLIST.md` itself, marking the entry inline with the date rather than silently rewriting it — the superseded wording stays visible in the correction block. `homeassistant` refutes the claim that a general-purpose hub has no three-state honesty model (`CANDIDATE_SHORTLIST.md` §9); `mindshift` corrects the operator and the commercial model (§10). Both written back 2026-10-06, commit `bb9fb63` | `homeassistant`, `mindshift` |
| **Transport-boundary statement made by the vendor** | A local-first platform that itself names its own paid cloud as the route to the remote-access case has drawn the boundary of its headline claim. Quote it verbatim and treat it as a **self-limiting claim**, not as an inconsistency: `homeassistant`'s App Store description reads "If you would like to access this app while away from home, the most secure and simple way to get started is Home Assistant Cloud", and its privacy policy places that cloud "outside the scope of this policy" | `homeassistant` |
| **Published client as an evidence class** | A corpus of JavaScript served to the browser is a **published client** — static, not a binary and not a runtime. Label findings `[FOUND IN PUBLISHED CLIENT]`, and state them to the level of what the code is *written to do*: `mindshift`'s first-party analytics pipeline (a per-session UUID `POST`ed to `/api/analytics/session` with `credentials:"include"`, four event categories, both calls ending `.catch(()=>{})`) is recorded as a static finding that no endpoint was called and no event was stored | `mindshift` |
| **Shell string is not a bundle string** | `i.replit.com/script.js` is in **every** served document, including both 404 pages, and **0** times in the 741,896-byte bundle — attribute the shell, never the client. The reverse trap is the 404 copy, which exists as two separate surfaces the served document does not share | `mindshift` |
| **A reference count must state its match mode** | `Sentry` case-sensitive 20 vs `sentry` case-insensitive **129**; `Clerk` 44 vs `clerk` **505**. Every zero is reported with the mode that produced it, and no case-sensitive zero is presented as an absence of the concept | `mindshift` |
| **Five-way verification before an absence is asserted** | When a vendor-declared URL does not resolve, close it with the probe **plus** the sitemap **plus** the router table **plus** a whole-bundle string sweep **plus** a byte-identical re-fetch. No one method suffices; it is the conjunction that makes `https://mindshiftcbt.com/privacy/` a **verified** absence rather than a transient 404 | `mindshift` |
| **Vendor-funded study is not independent validation** | Quote the funder's role from the paper's **own competing-interest disclosure**, never infer it, and never upgrade a preprint to peer-reviewed. A vendor's own funded study set beside the vendor's own marketing is an **intra-vendor** conflict — the sharpest class in the package — recorded with both quotes and left unresolved | `mindshift` |
| **Registry outranks the vendor's noun** | An independent registry's `studyType`/`observationalModel`/outcome counts are authoritative even against the paper's own title and the vendor's site. Record the registry **one** primary outcome against the paper's "four primary outcomes", and the `OBSERVATIONAL` registration against the noun "trial" on both other surfaces — quoted, unresolved | `mindshift` |
| **Headline N vs verified-install n** | A sample-size claim on a marketing surface and the sample the paper actually analysed are different quantities. `mindshift`: 380 recruited vs **156** with a code-verified install, so **224 never demonstrably installed the product**, while every headline surface says 380 | `mindshift` |
| **Corrigendum read as its own artifact** | A published corrigendum is fetched and read, not assumed. Where it corrects a measure and does **not** restate that measure's effect size (`mindshift`: Q-LES-Q-SF baseline corrected, primary conclusions "unchanged", QoL `d = 0.31` not restated), the effect size is **`[UNKNOWN]`** and is not carried over from the original | `mindshift` |
| **Quote checker: pair per paragraph, exclude the target, extend the corpus** | A whole-document DOTALL pairing mis-pairs across table cells (spurious misses); a pure line-based pairing drops multi-line wrapped quotes (false negatives). The working design is a **state machine that resets at every blank line and every table row**. The corpus must include the **candidate's fetched artifact tree** (including `.py`/`.toml`/`.yaml` for source-inspected candidates), the package docs recursively, the V2 tree and the V1 tree, and must **explicitly exclude the target file** so a file can never verify its own quotes. HTML/XML is tag-stripped (after removing `<script>`/`<style>`), JS `\uXXXX`/`\/`/`\n` are unescaped, elided quotes are split on `…`/`...` and each fragment checked, matching is lowercase-alphanumeric, and a markdown link embedded inside a quotation is split out because it breaks verification. **Two known blind spots, both stated rather than hidden:** the checker ignores any quoted fragment shorter than **25 alphanumeric characters** (short strings produce false positives), so a short vendor phrase such as *Always Free* or *2M patients* is **not** swept and must be checked by hand; and **a corpus that omits the vendor bytes makes "0 not found" mean only "reproduced somewhere in the package"** — an internal-consistency check, not a source check | `mindshift`, `homeassistant` |
| **A MISS list is triaged, never auto-accepted** | Each checker MISS is one of three things: a **checker artifact** (quote split across markdown emphasis or a table cell), an **author paraphrase wearing quote marks** — which must be reworded, not justified — or a **real defect** to fix against the source bytes. `mindshift` resolved 3 of 3: two were author
paraphrases (reworded, not justified) and one was a markdown link embedded inside the quotation (the quote
was split around the link and only the link-free fragments quoted). All four deliverables and both index
files now report **0 not found** | `mindshift` |
| **A paraphrase wearing quotation marks is the failure mode** | Reword the author's own characterisations rather than leaving them in quote marks: `mindshift` corrected "pre-**registered**" → "registered", "ambulatory" → "outpatient", a spurious `your` prefix, a dropped "all of", and a missing "ClinicalTrials.gov"; `homeassistant` reworded every author phrase and every piece of hypothetical dialogue that had been presented in quotes (*this receipt does not match anything I sent*, *V2 is better because it says so*, *No analysis of health signals*, *Adds local control and privacy*) | `mindshift`, `homeassistant` |
| **A retrieval failure is not an absence (second form)** | A byte-identical SPA shell returned for *every* path — including `/robots.txt` and `/sitemap.xml` — is a **fetch failure**, not proof that `anxietycanada.com` publishes nothing. Record `[UNKNOWN]` and never write the omission as a finding | `mindshift` |
| **A vendor's own material that undercuts its headline is left standing** | Both quotations are given and neither is resolved: the site's "Clinically proven" / "Proven, not promised." against its own referenced paper's "Improvements were unrelated to the frequency with which participants used the MindShift app" | `mindshift` |

## 7. What remains unverified

**Standing, for every candidate**

- Anything behind a premium tier, a subscription, an account, a trial, or a paywall.
- Anything requiring a real account or a real user's data.
- Internals of closed-source commercial apps: where processing happens, what is transmitted, what
  bundled SDKs exist, what the binary actually contains.
- Region-specific pricing, plan names and feature availability.
- Whether advertised features match reachable features, for every commercial candidate.
- Long-run behaviour: retention, deletion, vendor stability, and what happens if the service ends.

**Specific to `bearable`** — see `PROCESS_STATE.md` §"Residual uncertainty — `bearable`" for Q1–Q10, most
notably: the actual Premium price (unpublished); whether arbitrary imported metrics can enter the
correlation engine as outcomes; missing-data semantics for partially-covered days; the exact CSV schema;
and the unreconciled conflict between the privacy policy's no-sharing statement and the Play Store
data-safety declaration.

**Specific to `welltory`** — see §"Residual uncertainty — `welltory`" for W1–W12, most notably W2: the
value and scope of the "fixed medical threshold" and whether it overrides the personal baseline. Also
unretrieved: the iOS App Store listing, the April 2022 athlete-validation study, and the IEEE EMBC Berlin
presentation.

**Specific to `exist`** — see §"Residual uncertainty — `exist`" for E1–E12, most notably E6 (whether
multiple comparisons are corrected, given hundreds of pairwise tests) and E12 (whether an attribute
freezes at its last value when its source stops reporting, which is the decisive test of whether the
missing-data discipline is principled or merely convenient). Also unretrieved:
`developer.exist.io/reference/object-types-and-terminology/` (404), which would have closed E5 (the unit
model), and the public roadmap in detail.

**Specific to `sleephq`** — see §"Residual uncertainty — `sleephq`" for S1–S17. The four that decide the
analysis comparison are **S1** (whether the session `ahi` is relayed or re-derived), **S2** (the contents
of the eleven `jsonb` summary blocks), **S4** (what every `_95` field actually estimates) and **S3**
(`o2_score`, `duration_90`, `drop_90`, `drop_3`, `drop_4`). All four are behind an account: a single
authorised read-only API session would close them, and none was performed. Beyond those: **S5** (whether
the free tier deletes or hides data past 30 days), **S6** (what AI Insights is), **S7** (whether the sleep
alarm is supervised), **S8** (what a share link contains), and **S17** (whether the O2 Ring Pro holds any
regulatory registration — no registry was searched, so this is *unknown*, not *absent*).

**Important: no API endpoint has been called for any candidate.** Every API finding is from documentation,
not from a live response.

**Specific to `nightscout`** — see §"Residual uncertainty — `nightscout`" for N1–N14, most notably N1
(whether the `readable` default really is in effect for an anonymous caller, and whether the public notice
renders) and N2 (whether the A1c double-conversion defect reproduces at the pinned commit). Both are code
readings; settling either needs a throwaway local instance, which is a separate authorization decision.

**Specific to `empatica`** — see §"Residual uncertainty — `empatica`" for R1–R11. R1 is the User Manual
(178 pages, 112.8 MB, **no text layer** — an OCR decision, not a fetch) and R2 is the 98% figure's
provenance, which no other surface supplies. Nothing in this candidate is a runtime observation.

**Specific to `oura`** — see §"Residual uncertainty — `oura`" for R1–R12, most notably R1 (per-field null
semantics: representability is documented, *cause* is not) and R2 (the four bare-string date/time schemas
whose difference is carried only by the type name). **No endpoint was called, including the sandbox.**

**Specific to `homeassistant`** — see §"Residual uncertainty — `homeassistant`" for R1–R13. R1 (whether
the alert false-all-clear is intended or unhandled) decides the alarm comparison, R2 (what a running
instance does with the non-parsing module) decides the severity of finding 5, and R3 (why CI appears not to
have caught it) is left unresolved rather than guessed at. R12 is now closed — the shortlist correction was
written back on 2026-10-06 — and R13 records that this candidate's deliverables did not go through the
quote-verification sweep until after they were committed: **48 MISSes, of which 33 were corpus gaps and 15
were author phrases or hypothetical dialogue wearing quote marks, reworded rather than justified**. **No
instance was built or contacted; the runtime-observation count is zero.**

**Specific to `mindshift`** — see §"Residual uncertainty — `mindshift`" for R1–R12. R1 (whether the
privacy policy and terms exist **inside the app**) decides the severity of the package's strongest verified
absence; R2 (whether a controlled or comparative study exists, which decides whether "Clinically proven"
has a source) and R3 (whether the quality-of-life effect size survived the corrigendum) are both answered
from documents that were not read. **Every one of the four high-value questions (R1, R3, R6, R7) is behind
the app binary**, which cannot lawfully be obtained for this candidate — the only candidate in the package
where that is the *sole* remaining barrier. The analytics endpoints were **not** called.

**Structural limitation of the whole run, and it is the single most important line in this file.** The free
traffic-inspection toolchain (mitmproxy + a disposable emulator) was **validated against emulator system
components and then never used against a target app**, for any of the ten candidates. Not one candidate's
binary was lawfully obtainable within the recorded authorization policy, so **the runtime-observation count
across the entire package is zero**. Every behavioural statement in every comparison is therefore a reading
of source, a reading of a published client, a documented contract, an independent record, or an explicitly
labelled inference. The toolchain's readiness is a *capability* result, not evidence about any candidate, and
the two are not to be conflated. A first genuine target-app inspection will surface problems this validation
could not reach.

## 8. Paywalls, login requirements and inaccessible material

| Blocker | Affected source | Handling |
|---|---|---|
| Publisher paywall / bot block (HTTP 403) | ScienceDirect article page for the Bearable citation | Metadata obtained via the Crossref API; citation verified as a systematic review of oncology PRO mHealth apps. Paper body not read — so the review's specific assessment of Bearable remains unverified (Q9) |
| **Incorrect DOI in first retrieval attempt** | A guessed DOI (`10.1016/j.conctc.2020.100631`) resolved to a different paper | Caught by verifying title/journal against the Crossref record; the correct DOI is `10.1016/j.adro.2020.09.016`. Recorded because a wrong citation is worse than none |
| Login-required store artifact | Bearable APK/IPA | Not obtained. No mirror used. Recorded in `bearable_IMPLEMENTATION_INSPECTION.md` |
| Account-required app | Bearable's usable surface | Sign-up is a hard gate; the brief forbids creating accounts, so even with an artifact the observable surface would stop at registration |
| Search engine blocking | DuckDuckGo HTML endpoint (HTTP 202/302) | Substituted Bing and Mojeek; no impact on coverage |
| DNS failure | `emoods.net` | Candidate dropped from the shortlist rather than researched speculatively |
| Partially retrieved support content | Bearable's "Are blank entries incorporated into insights?" and "What happens to factors that I leave blank?" articles | Their existence is recorded; their content was not retrieved and is listed as unresolved (Q4) rather than guessed |
| No vendor price publication | Bearable Premium pricing | Only the Play Store IAP range is available; recorded as a range, not a price |
| No browser/desktop client | Bearable | Vendor-stated limitation; recorded as such |

## 9. Reproducing this research

```sh
# Verify the ten candidate packages exist (10 of 10 complete; the shortlist is exhausted)
ls -d docs/app_comparison/*/ | wc -l              # expect 10 candidate directories
ls -l docs/app_comparison/{bearable,welltory,exist,oscar,sleephq,nightscout,empatica,oura,homeassistant,mindshift}/

# The `sleephq` package: the primary artifact is the vendor's OpenAPI document
curl -sS -o /tmp/sleephq-swagger.json https://sleephq.com/api/swagger.json
shasum -a 256 /tmp/sleephq-swagger.json
#   expect 9a4305e663ddc9b65c9b471e8fd04e1085f382814c3540193d85cbd8660536a4
python3 -c "import json;d=json.load(open('/tmp/sleephq-swagger.json'));print(d['swagger'],d['host'],len(d['paths']),len(d['definitions']))"
#   expect: 2.0 sleephq.com 21 11
# No endpoint is called beyond fetching the published specification.

# The `oscar` package: source inspection was performed at a pinned commit
cd /tmp/aw2research && git clone --depth 1 https://gitlab.com/CrimsonNape/oscar-sql.git \
  && cd oscar-sql && git log -1 --format='%H'   # expect 28df2e46e35e6260e186fced997edadcf001cc24

# The `mindshift` package: the primary artifacts are the served client and public records.
# Every command below is a read of a public document. No endpoint is called.
curl -sS -o /tmp/ms_index.js https://www.mindshiftcbt.com/assets/index-BogFfh2L.js
shasum -a 256 /tmp/ms_index.js
#   expect 1bb1681128467a90803d37652bc7eb8d65e209a8dbdd867a5a260e8cba1ef828 (741896 bytes)
wc -c /tmp/ms_index.js
# The privacy-URL absence, verified five ways (see the fifth-mode note in §6.1)
curl -sS -o /tmp/ms_privacy.html -w '%{http_code} %{size_download}\n' https://mindshiftcbt.com/privacy/
#   expect a genuine 404 with <title>Page not found | MindShift CBT</title>
curl -sS https://www.mindshiftcbt.com/sitemap.xml | grep -c '<loc>'
#   expect 9 URL entries, none of which is a privacy or terms page
grep -o -E 'path:"[^"]*"' /tmp/ms_index.js | sort -u | wc -l
#   expect 13 SPA routes, again with no privacy or terms path
grep -o -F 'Privacy Policy' /tmp/ms_index.js | wc -l   # expect 0
#   NOTE: a count is meaningless without its match mode; 'sentry' is 129 case-insensitive
#   and 20 as 'Sentry', and 'clerk' is 505 case-insensitive and 44 as 'Clerk'.

# Quote verification: the target file is EXCLUDED from its own corpus so a file can never verify
# its own quotes. Expect "0 not found" on all four — but only once the corpus actually contains the
# retrieved bytes, i.e. the candidate's artifact tree (/tmp/aw2research/{slug}/) AND, for a
# source-inspected candidate, the .py/.toml/.yaml files. Without those, a MISS is a corpus gap first.
for f in docs/app_comparison/mindshift/*.md; do python3 /tmp/aw2research/mindshift/quotecheck.py "$f"; done
# source-inspected candidate: same checker with the root overridden and source suffixes indexed
for f in docs/app_comparison/homeassistant/*.md; do AW2_ROOT=/tmp/aw2research/homeassistant python3 /tmp/aw2research/qc2.py "$f"; done
#   The homeassistant sweep initially reported 31 + 7 + 4 + 6 = 48 MISSes. All 31 in the claims
#   file and two of the four in the inspection file were corpus gaps (vendor pages, and .py
#   sources the checker did not index). The remaining 15 were author phrases or hypothetical
#   dialogue wearing quote marks, and were reworded rather than justified.

# The `homeassistant` package: the strongest mode ran with no authorization question
# (Apache-2.0). Eleven files read and hashed at tag 2026.9.4 (commit 9212531f…).
```

# Confirm no stale paid-proxy references remain anywhere in the comparison docs
grep -RInE 'Charles|charles|10\.0\.2\.2:8888' docs/app_comparison/ || echo "clean"

# Confirm the shortlist and process state are present
ls -l docs/app_comparison/CANDIDATE_SHORTLIST.md docs/app_comparison/PROCESS_STATE.md

# Public-info gate before any push (tracked files only)
python3 scripts/check-public-info.py --self-test
python3 scripts/check-public-info.py
```

Free inspection stack, when a candidate becomes inspectable — full procedure in
`FREE-ANDROID-TRAFFIC-INSPECTION.md`. Summary:

```sh
# 1. Host proxy, before the target app is launched or installed
mitmweb --listen-host 127.0.0.1 --listen-port 8080

# 2. Disposable emulator
emulator @pixel_4_w_gapps -no-window -no-audio -no-snapshot

# 3. Point the emulator at the host proxy
adb shell settings put global http_proxy 10.0.2.2:8080

# 4. Verify the proxy is listening and the setting took
lsof -nP -iTCP:8080 -sTCP:LISTEN
adb shell settings get global http_proxy

# 5. Cleanup, verified
adb shell settings delete global http_proxy
adb shell settings get global http_proxy   # must print null
```

`FREE-ANDROID-TRAFFIC-INSPECTION.md` remains the authoritative procedure; the summary above is for
orientation only.
