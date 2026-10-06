# App comparison — research method

**Research date:** 2026-10-06 (session started 2026-10-05 ~17:41 PDT)
**Execution mode:** pilot — full shortlist, one candidate processed end to end
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
| Android SDK root | `$ANDROID_HOME` (the run used the machine's default Android SDK location) |

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
2. **Absence of evidence is unknown, not proof of absence.** "Bearable shows no CPAP feature anywhere
   examined" is recorded as absence-of-evidence and explicitly flagged as *not* proof that no CPAP code
   exists in the binary.
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

Four modes have been reached. Which one applies is recorded per candidate, and none is ever silently
upgraded into another.

| Mode | Meaning | Candidates |
|---|---|---|
| **Not performed** | No artifact of any kind was lawfully obtainable | `bearable` |
| **Partially performed** | No product binary, but authorized vendor-published artifacts were inspected | `welltory`, `exist`, `sleephq` |
| **Performed** | Product source or a lawfully authorized build was inspected | **`oscar`** (GPLv3 source, pinned commit); **`nightscout`** (AGPL-3.0 source, pinned commit); expected for `homeassistant` |

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

**Structural limitation of this pilot.** The traffic-inspection toolchain was validated only against
emulator system components, never against a real target app. A first genuine inspection may surface
problems that this validation could not reach.

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
# Verify the candidate packages exist
ls -l docs/app_comparison/bearable/ docs/app_comparison/welltory/ docs/app_comparison/exist/ \
      docs/app_comparison/oscar/ docs/app_comparison/sleephq/

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
