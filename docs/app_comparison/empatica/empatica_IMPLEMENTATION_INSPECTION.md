# Empatica EpiMonitor — implementation inspection

**Slug:** `empatica`
**Research date:** 2026-10-06
**Phase:** 2A (authorized implementation inspection)
**Outcome:** **Performed — artifact and regulator record.** App binary **not** inspected. Source **not**
inspected. API **not** inspected. Static-source inspection: **not applicable** (closed source, no published
client or SDK). Runtime observation: **none.**

**Inspection mode.** `PROMPT1.md` Phase 2A recognises several modes for candidates whose binary cannot
lawfully be obtained (`docs/app_comparison/PROMPT1.md`, "Execution modes and sequencing"). For this candidate
two were available and were used:

| Mode | Available? | Used? |
|---|---|---|
| Runtime observation on an authorized emulator | No — no lawful binary; iOS-only consumer app; prescription-gated | No |
| Decompilation / static analysis of the app binary | No — no lawful acquisition path | No |
| Vendor-published source (GPL/AGPL) | No — nothing published | No |
| **Vendor-published artifact inspection** (docs, contracts, certificates, architecture pages) | Yes | **Yes** |
| **Independent regulator record inspection** (openFDA) | Yes | **Yes** |
| Vendor-published API reference / output contract | **No** — not published; supplied only to commercial Cloud API clients | No |

This is a **new mode instance** for the package. `oscar` and `nightscout` reached "Performed" through
published source; `welltory` through a vendor dataset; `exist` and `sleephq` through a published API
reference. Empatica is the first candidate where the **strongest available artifact is a regulator's
record of the vendor's own submission**, and the first where a **contract (EULA)** is a primary evidence
surface. Recorded in `RESEARCH_METHOD.md`.

---

## 1. Summary of outcome

| Target | Inspected? | Depth | Basis |
|---|---|---|---|
| `com.empatica.healthmonitor.alert` (EpiMonitor iOS) — binary | **No** | — | No lawful acquisition path |
| `com.empatica.healthmonitor` (Empatica Care iOS) — binary | **No** | — | No lawful acquisition path |
| Android APK | **No** | — | Listing not located; see `…_CLAIMED_FEATURES_AND_PURPOSE.md` §4 |
| Empatica Cloud API endpoints | **No** | — | Not published; "provided" to clients commercially |
| Empatica GitHub organisation (`empatica`) | Yes | Repository list, commit history | 5 public repos; 1 original (emptied), 4 unrelated forks |
| `developer.empatica.com` (historic docs host) | Yes | Deletion dated to a commit | Three commits, 2025-02-18 |
| EpiMonitor User Manual (PDF) | **Retrieved, not read** | 178 pages / 112.8 MB / image-only | `pdffonts` empty; `pdftotext` returns 178 bytes |
| EULA | Yes | Full text, 68 lines | `s3.amazonaws.com/box.empatica.com/.../0.2-black.html` |
| US privacy notice | Yes | Full text | `empatica.com/epimonitor/privacy-policy-us` |
| `/legal/` (security + standards + certificates) | Yes | Full text | `empatica.com/legal/` |
| FDA 510(k) records, all 9 clearances | Yes | Database fields | openFDA `device/510k.json` |
| FDA product-code classifications (POS/DQA/MWI/LEL/GYD) | Yes | Definition text | openFDA `device/classification.json` |
| **K250515 510(k) summary (EpiMonitor)** | **Yes** | **Full document, incl. performance tables** | `accessdata.fda.gov/cdrh_docs/pdf25/K250515.pdf` |
| K232915 510(k) summary (EpiMonitor, earlier) | **No** | — | Path returns 404 (`pdf24` for K232915) |
| `/cloud-api/` architecture and contract description | Yes | Full text | `empatica.com/cloud-api/` |
| Support-centre articles | **No** | — | HTTP 403 behind Cloudflare |

**What this mode can and cannot establish.** It establishes, with a primary document, what the vendor told
a regulator, what the vendor promises and disclaims contractually, and what the vendor publishes about its
own architecture. It **cannot** establish any runtime behaviour: not whether an alert fires, not whether a
call is placed, not how the app handles a missing sensor, not what it stores on the phone. Every finding
below is labelled *documented* or *measured-artifact*, never *verified*, and no finding in this file is a
runtime observation.

---

## 2. Authorization basis

The `PROMPT1.md` Phase 2A authorization covers inspection of artifacts the vendor publishes. Its limits are
recorded and were observed:

- **No bypass of any protection.** Nothing was bypassed. No app binary was obtained, so no certificate
  pinning, encryption, obfuscation, anti-debugging or platform control was ever encountered.
- **No credential extraction, no private data.** No account, no trial, no prescription, no device, no
  login. No vendor endpoint was called with any credential.
- **No vendor API called.** `api.empatica.com`, the Care Portal, Care Lab Portal, Research Portal and any
  Cloud API host were **not contacted**. Their existence is recorded from the vendor's own links; nothing
  about their behaviour is asserted.
- **Independent regulator data queried, not scraped around any control.** openFDA and the FDA's own document
  server are public, unauthenticated, and intended for exactly this use. Product codes were **queried**, not
  interpreted (`RESEARCH_METHOD.md` standing sub-check).
- **No reverse engineering.** No decompilation, no disassembly, no memory inspection, no traffic
  interception on this candidate. The mitmproxy stack (`FREE-ANDROID-TRAFFIC-INSPECTION.md`) was **not used**
  for Empatica, because there is no lawfully obtainable target to point it at.
- **Retrieved artifacts are stored outside the repository** under `/tmp/aw2research/empatica/` and are
  identified by SHA-256 so the inspection is reproducible without redistributing the vendor's documents.

**One judgement call, recorded.** The EpiMonitor User Manual was retrieved in full because the vendor
publishes it openly next to a Download control on a public page with no gate, no login and no acceptance
step. It was not read, because it contains no text (below). Nothing gated was fetched.

---

## 3. Artifact record

All artifacts in `/tmp/aw2research/empatica/`, fetched 2026-10-06 with a Chrome user agent. Hashes are
SHA-256 and were computed after the last fetch of each file.

| Artifact | Bytes | SHA-256 |
|---|---|---|
| `web/empatica_com_epimonitor.html` | — | `cb2f283af331887427086570d96c38a0cf1cf6c0f8cdc98595460fadcc264b91` |
| `web/empatica_com_embraceplus.html` | — | `48f616432a4a2e3530104b6c15d05ef8ec4b242dbd9751181c6045c212562018` |
| `web/empatica_com_store_epimonitor.html` | — | `bdb7a9b95f5295b40b5b800b8ef2338d6a1041c9e2994b89205d61e6603b4150` |
| `web/empatica_com_epimonitor_privacy-policy-us.html` | — | `6a7903e8d59c811912e9de6e1af45d3828fd361a4883f14882592bca8c459c6a` |
| `web/eula.html` | — | `66be3a0ab865b3a4ce60153c108f86732ef50c4306d094a3f6ee6be9cabb8567` |
| `web/empatica_com_legal.html` / `…_legal2.html` (byte-identical) | 381,093 | `1fb744154d84eeaf890676f8381f558f78357bbeec3b07392a4d68a9c2ffc1b7` |
| `web/empatica_com_cloud-api.html` | 357,176 | `6158d0eeb7b23675ca34056305eb81a92544b6ecd193feb9673b6975741ba67b` |
| `web/empatica_com_embracemini.html` | 346,573 | `092ce72adf516b5441a2cb5b424fc048dfdfd6c47f6502c4289629d87b8643e7` |
| `web/empatica_com_manuals.html` | 345,149 | `630614edbbc96b0c19f7fabd4865ec06c746f92f17c829cf819e7304d6d79c1a` |
| `web/empatica_com_epimonitor_seizure-forecasting.html` | — | `48131208534ac56ea467580408c878b62de7500a6742da693bdfbd9f7cc00c50` |
| `web/e4_sunset.html` | 309,008 | `cedc9f92a6567835d8b4df136625248653778399f4cb015d91c221fe5a85ad0b` |
| `web/developer_empatica.html` (identical to `gh_pages.html`) | 264 | `1e5e48cd6df1640e2219385f4096b54a45f0bbcb441d71cf01b1bd83c88d8845` |
| `web/epimonitor_manual.pdf` | 112,765,579 | `4bd5eedda64871096b29e4d7c1e96bd15b9b78cfaede1c4d8a1392489f6a5702` |
| `web/empatica_tc.pdf` (ISO 13485 certificate) | 584,744 | `cf38a846502333c759e6900b2067a49ace34f79e3c6839df7b1a23b54b214b` |
| `fda_K250515.pdf` | 2,661,556 | `595c5048b848b4dfc9a0cd3ca113e3f078d1afc6a3be7764934fcd6a18cbfd9e` |
| `fda_K250515.txt` (derived) | — | `a6b880654bbde8a8bed4dcd2da0c3ee058c941bee073f5ed2da6a36302a7eb55` |
| `fda_K232915.pdf` (**404 body**, not a summary) | — | `e49125b35874947bf03fcee5bfc055000b543c7c38d17077056582e3e8d336c6` |
| `fda_empatica_all.json` | — | `f35cd9bdcd5e0bd714c97c55da2cb0d9004b5766ba60ebd172bd6757c2df50c7` |
| `itunes_epimonitor.json` | — | `73c66f5b1a4fcb2b48abb70ec909d2a12b9dccc0fe1cf53c1cd81eef31648cbc` |

**Two hashes that are themselves evidence.** `developer_empatica.html` and `gh_pages.html` are
byte-identical (264 bytes), which is how the CNAME chain was confirmed rather than inferred: the custom
domain and the GitHub Pages URL serve the same redirect. `empatica_com_legal.html` and
`…_legal2.html` were fetched in separate passes and are byte-identical, confirming that the page carrying
two of this candidate's findings was stable across the research window.

**A note on the manual.** 112,765,579 bytes for a user manual is 634 KB per page across 178 pages, which is
the shape of a scanned or image-exported document. That was a hypothesis, not a finding. §5.1 records how it
was tested.

---

## 4. Static findings — architecture (as documented)

**A1 — The alert path is a four-hop cloud chain with a physical precondition.**
`[PUBLIC CLAIM]` + `[INFERENCE]`. The vendor states the chain; the inference is only that each hop is a
failure point.

```
[EmbracePlus watch]
   │  EDA · PPG(4ch) · accel · gyro · skin temp · EpiAlgo v2.1
   │  detection: "unusual patterns … generalized tonic-clonic seizures lasting 20 seconds or longer"
   ▼  Bluetooth 5   ◄── REQUIRES the phone within 15 ft / ~5 m
[dedicated smartphone + EpiMonitor app]
   │  Bluetooth + internet connectivity required
   ▼  phone's internet connection
[Empatica Cloud]  ── call + SMS ─►  caregiver 1 … N   (N = 1 Basic / 3 Standard / 10 Plus)
   ▲
   └── active subscription required for the device to work at all
```

**A2 — The research platform uses the same shape, and the vendor publishes it.** `[PUBLIC CLAIM]`. Quoted
from `/cloud-api/`: EmbracePlus → "Bluetooth® to a dedicated mobile app" → "the phone's internet connection
to automatically transfer all data to the Empatica Cloud" → "Empatica Cloud securely stores data and makes
it available for transfer to your preferred system". **There is no local, offline or self-hosted variant in
either product line.** For the comparison this is the single clearest architectural contrast with
AnxietyWatch2, whose stated design keeps the store and the alarm on owner-controlled hardware
(`docs/specs/AMENDMENTS.md`; `openspec/changes/pi5-vitals-core/`).

**A3 — The research integration surface is object storage, not an API.** `[PUBLIC CLAIM]`. "uses two formats
of files, **Avro and CSV** … Data will be available in **S3**", with "full developer documentation" provided
to clients. So a consumer of the platform receives **datasets**, not pollable current values. No other
candidate in this package has that integration shape, and it has a direct consequence for the comparison's
"real-time" section: this platform is explicitly not built to deliver a live value to a third party.

**A4 — Data is captured continuously on-watch and synced via the phone; the watch has on-board flash.**
`[PUBLIC CLAIM]`. This means a temporary phone absence is a **delay**, not necessarily data loss. The
vendor says this about the research path ("non-stop data recording synced with the Care or Care Lab App")
but does **not** state it for the alert path, where the phone is a hard requirement for the alert itself.
Recorded as an asymmetry in the vendor's own descriptions, not as a defect.

**A5 — A remote stop-collection control and a wearing-detection signal exist.** `[PUBLIC CLAIM]`:
"remotely end the data collection with EmbracePlus"; "Monitor wearing detection information to understand
your trial compliance and if intervention is needed". These are operational features with no equivalent in
any other shortlisted candidate. Wearing-detection is notable because **the same absent-data problem that
Nightscout handles badly and AnxietyWatch2 specifies a three-state rule for is here surfaced as a
first-class signal** — a researcher can see whether the device is on the body.

**A6 — The consumer app is a thin client over the vendor's cloud.** `[INFERENCE]` from `[PUBLIC CLAIM]`.
The app is described as the thing that "communicate[s] with" the watch and contacts caregivers; the call and
SMS are the vendor's function, not the phone's. No local alerting, no local storage guarantee, and no
offline mode is described anywhere in the surfaces read. Labelled **inference** because the vendor does not
say it in those words.

---

## 5. Static findings — reconciliation points

### 5.1 The EpiMonitor User Manual has no text layer

`[THIRD PARTY]`, measured locally. This began as a hypothesis and was tested rather than assumed.

**Hypothesis:** the manual is a scanned or image-exported PDF, so its text cannot be extracted.

**Test and result — the hypothesis is confirmed, more strongly than expected:**

| Probe | Result |
|---|---|
| `pdfinfo web/epimonitor_manual.pdf` | 178 pages; 112,765,579 bytes; created 2025-05-16; Adobe InDesign 20.3 (Macintosh); page size 424.677 × 424.677 pt; `Tagged: yes`; not encrypted |
| `pdffonts web/epimonitor_manual.pdf` | **empty font table — the document contains no fonts at all** |
| `pdftotext -layout` (all pages) | **0 lines** |
| `pdftotext -raw -f 1 -l 178` | **178 bytes total** — one form-feed per page, i.e. no text |
| XObject subtype scan | `Image` 14, `XML` 12, `Form` 8; **no `Type0`**, no `ToUnicode` |
| Filter scan | `FlateDecode` 204, `DCTDecode` 8 (JPEG) |

**Findings.**

1. **The published instructions for use of an FDA-cleared medical device are not text-searchable.** A user
   looking for a warning cannot search for it.
2. **They are not screen-reader accessible.** With no font resources and no text layer there is nothing for
   assistive technology to read. `Tagged: yes` is reported, but a tag tree with no text content cannot
   convey text.
3. **They are not machine-checkable**, which is why the manual contributes **no rows** to the
   claim-reconciliation table in §7 and why the question in §5.3 stays open.
4. **112.8 MB** is the download size for a document a caregiver may want on a phone.
5. **The manual was created 2025-05-16**, which precedes the K250515 clearance decision (2025-06-19).
   Recorded as a date; it is normal for documentation to be finalised before a decision date and no
   inference is drawn.

**And the honest limit of this finding:** whether the PDF *displays* correctly is not tested here. A PDF with
no fonts and no text is a legitimate way to publish a designed booklet, and it renders. The finding is about
**searchability, accessibility and machine-checkability**, not about whether the document works as a
picture.

### 5.2 "No clinical studies have been performed on the subject device"

`[REGULATOR RECORD]`. This is the decisive reconciliation point for the whole candidate. Quoted from
K250515, page 9:

> "**No clinical studies have been performed on the subject device.** Clinical studies were performed on
> EpiAlgo ver 2.1 to support clearance of the predicate device (K181861)."

Reconciled against the vendor's own product page:

> "demonstrating a **98% detection accuracy** against the gold standard of hospital-based EEG measurement"

**The finding is not that the number is wrong.** It is that:

- The regulator document for the device reports **PPA and FAR**, not accuracy, and reports that the device's
  performance on the shipped hardware was established by **bench testing for equivalence** to a 2018
  predicate, with the two low-sensitivity analyses being **retrospective re-analyses of previously collected
  data**.
- The real-world PPA figures (0.80–0.87) come from **Embrace2** hardware, which the document states
  explicitly.
- The nearest figure to 98% is an **uncorrected** PPA of 1.000 in a subgroup of **12 patients / 17 seizures**
  (corrected 0.905, CI 0.891–0.917).
- **`corrected PPA` is used without definition** in the document. An undefined term in the decisive table is
  the same finding class as SleepHQ's undefined value domains, in a document of higher evidentiary weight.
- **FAR does not appear on the vendor's page at all.** Paediatric real-world FAR of 0.34/24 h implies roughly
  one false caregiver alert every three days. The vendor's own optional low-sensitivity mode is described as
  existing "to reduce false alarm incidents" — the vendor conceding the burden.

**What would resolve it:** the K181861 clinical study, or a peer-reviewed paper reporting 98%. Neither was
retrieved. The claim is recorded as **untraceable from the page that makes it**, which is a different
statement from false, and this file does not make the stronger one.

### 5.3 The "do not rely on this alone" question is open, and why

`[PUBLIC CLAIM]` + `[UNKNOWN]`. The cleared product code's own definition states the device "does not
predict seizure onsets, and is not intended as a **standalone** seizure monitoring device" (21 CFR 882.1580,
POS). The vendor's marketing pages state the detection scope limit and the 15-foot requirement, but no
"do not rely on this alone" statement was found on the **four marketing pages read**.

The obvious place for it is the manual. The manual was retrieved (178 pages, complete) and **has no text**
(§5.1). So:

- The question is **open**, not answered negatively.
- The blocker is **an artifact that exists and could not be read**, not a missing artifact.
- Closing it requires an OCR decision — 178 image pages — which is a separate judgement with its own error
  rate, and was not taken.

This distinction is recorded because "we didn't find it" and "it isn't there" are different claims, and the
package treats them differently.

### 5.4 Documentation lifecycle: the developer site was emptied on a date

`[THIRD PARTY]`. See `…_CLAIMED_FEATURES_AND_PURPOSE.md` §4.1 for the full record. In brief: the repository
`empatica/empatica.github.io` received three commits on **2025-02-18** — "Redirect to sunset page",
"**Remove all the pages**", "Restore CNAME file" — and the hostname `developer.empatica.com`, whose CNAME is
unchanged, now returns a 264-byte redirect to `/research/e4-sunset/`. The sunset notice states the previous
generation's software "can no longer collect and store data, rendering services inoperative" as of
**2025-02-14**. The Internet Archive's CDX index independently shows the content that used to be there
(`android-sdk-tutorial-*`, `empatica-android-sdk-javadoc.zip`, `E4_streaming_server_scheme.jpg`, captures
from 2016 onward).

**Recorded as a lifecycle fact**: an end-of-life product being withdrawn is legitimate and the vendor
published a sunset notice and a migration path. What is recorded is the observable shape — **the canonical
developer hostname was emptied four days after the sunset date and now serves marketing content, so the
public record of the previous generation's data interface survives only in third-party archives**. This is
the same class as the `oscar` header growth breaking v1's positional parser and SleepHQ's unpublishable
session write path: **a pipeline's upstream contract can change or vanish, and the pipeline has no recourse**.

### 5.5 A publishable contract that cannot bear the adjacent claim

`[PUBLIC CLAIM]` + `[THIRD PARTY]`. On `/legal/`, the link labelled "Read our Terms & Conditions" resolves to
a **TÜV Rheinland ISO 13485 certificate PDF** (cert. no. 39 05 0072201, Empatica S.r.l., valid
2025-05-08–2028-05-24), and that certificate carries the standard TÜV disclaimer that it "does not
represent proof that the statutory requirements of the Directives" it names (93/42/EEC, 90/385/EEC,
98/79/EC, Regulations (UE) 2017/745 and (UE) 2017/746) have been fulfilled — while the EmbracePlus page
states "CE-certified as a class IIa medical device".

**Recorded fairly.** Every notified body writes that sentence and it is correct: ISO 13485 is a quality
system standard, not an MDR conformity certificate, and the vendor almost certainly holds the separate CE
marking the marketing page claims. The finding is about **placement**: the artifact the vendor offers as
its compliance evidence cannot bear the claim the adjacent page makes, and the MDR certificate and
notified-body details that would are not published. **Mislabelled link + unresolved tension, most
conservative governs.**

Also recorded in the same section: the published standards list contains a **duplicated line**
(ISO 10993-1:2018 appears twice consecutively), and **IEC 60601-1-8 (alarm systems) is absent** from a list
that does include usability (60601-1-6, 62366-1), home-healthcare (60601-1-11), risk management (14971) and
software lifecycle (62304). The list is explicitly not claimed to be exhaustive, and 60601-1-8 governs alarm
signals generated by equipment rather than cloud caregiver notification. Recorded as an observation about a
published list, **not** as a non-compliance finding.

### 5.6 Contract versus marketing — a cross-instrument conflict

`[PUBLIC CLAIM]`, both sides the vendor's own current text. The product page: "designed to keep you and your
loved ones safe". The EULA it links to: the vendor "**cannot guarantee that a call or text message will
reach any Caregiver**"; the app "may fail to work with unexpected interruptions, including … loss of Internet
connectivity, power outages, equipment failure … telecommunication service provider failures"; the vendor
"**DOES NOT REPRESENT OR WARRANT THAT THE APP, ITS USE, OR ANY RESULTS GENERATED THEREFROM ARE ACCURATE,
COMPLETE, RELIABLE, CURRENT, OR ERROR-FREE**"; liability "**SHALL NOT EXCEED THE PURCHASE PRICE PAID**"; no
liability under any circumstance for "**PERSONAL INJURY OR DEATH**".

This is a **different class** from items 1–3 in the conflicts table of the claimed-features file (internal
inconsistency on one page). A marketing claim and a licence are two instruments, and a user who reads only
one receives a materially different picture of what the product commits to. **Recorded, not resolved.**

### 5.7 The privacy notice for the device service reads like an ad-tech notice

`[PUBLIC CLAIM]`. The US notice expressly covers "the EmbracePlus watch, the associated App, and the
associated EpiMonitor Account page", was last updated **2024-12-01**, and in addition to health data
describes Google Analytics, pixel tags, cross-device tracking, advertising networks, and sharing "customer
list information (such as your name, email address and other contact information)" with third parties for
targeted advertising.

**Two ambiguities recorded rather than resolved:** whether the advertising language is intended to reach
watch-derived health data or only the web Portal's browsing activity is not determinable from the document;
and the notice states that additional notices control on conflict, so the consumer notice may not be the
governing instrument at all. **Documentation finding only** — this file makes no claim about actual
processing.

### 5.8 Two numbers for one quantity, three times

`[PUBLIC CLAIM]`. EmbracePlus battery is "up to 7 days" (EpiMonitor page) and "up to 28 days" (Cloud API
page); EmbraceMini battery is "at least 7 days" and "14+ days" on one page; "No black box algorithms" and
"AI-powered" sit on one page. **Editorial defects, most conservative value governs.** The 28-day figure has a
plausible benign explanation (a research configuration without continuous seizure detection draws far less
power), which is exactly why it is recorded as a conflict to be *explained* rather than a contradiction to
be *scored*.

### 5.9 What the vendor's own words concede

`[PUBLIC CLAIM]`. Collected because a vendor's honest boundary statement is a positive to record:

- "It is **not designed to detect** seizures that do not generate movements such as complex partial
  seizures, absence seizures or non-convulsive seizures."
- "The phone needs to be kept **within 15 feet** of the EmbracePlus watch for alerts to be sent."
- "Use in children **younger than 6** should be evaluated by a clinician on a case-by-case basis."
- "The User must have a Device that is compatible with the App … EMPATICA does not warrant that the App will be
  compatible with all mobile devices."
- "**EMPATICA shall not be obliged to support obsolete versions** of the App or to update or provide versions
  of the App for use on any particular mobile device or operating system."
- "The User is **solely responsible** for compliance with applicable data protection laws when providing the
  Caregiver(s) contact information."
- "Empatica … work[s] with researchers … to assist them with leveraging our product documentation in **their
  own validation objectives**."

---

## 6. Runtime observation

**None.** No Empatica app was installed, launched, configured or observed. No device was worn. No account was
created. No alert was triggered or cancelled. No network traffic was captured or intercepted. No vendor
endpoint was contacted.

Consequences, stated plainly:

- Nothing in this file says what the app does.
- Nothing in this file says whether an alert reaches a caregiver.
- Nothing in this file says how the app behaves when the phone is out of range, when Bluetooth drops, when
  the subscription lapses, or when a sensor fails. **These are the questions the comparison most wants
  answered for this candidate and this inspection cannot answer any of them.**
- The package's evidence classes hold: there are **no `[runtime observation]` rows** for `empatica`, and a
  future reader must not infer any.

**The one runtime-adjacent fact available** is third-party store metadata — the iOS version, release date,
size, rating and rating count for `com.empatica.healthmonitor.alert` and `com.empatica.healthmonitor` — which
establishes that the app exists, when it last shipped, and that the EpiMonitor app is ~6.5 months old at the
research date. That is platform metadata, not observed behaviour.

---

## 7. Claim reconciliation

Claims from `empatica_CLAIMED_FEATURES_AND_PURPOSE.md` §14.1 that can be checked against a primary artifact.
"Match" means the artifact supports the claim as stated, in the register the vendor stated it. A claim
recorded as **narrower** is one where the artifact supports *part* of what the page implies.

| # | Claim | Checked against | Result |
|---|---|---|---|
| 1 | FDA-cleared | openFDA | **Match** — 9 records, incl. K232915 and K250515 naming EpiMonitor; product code POS |
| 2 | "510(k) … ( K221282 - K230457 )" | openFDA | **Incomplete** — both numbers exist and describe the Health Monitoring Platform, but neither names EpiMonitor; the EpiMonitor clearances are uncited |
| 3 | CE-certified Class IIa under MDR 2017/745 | `/legal/` artifacts | **Unverified** — the certificate the vendor links is ISO 13485 and disclaims MDR conformity; no MDR certificate or notified-body details published |
| 4 | ISO 13485 | certificate PDF | **Match** — cert. 39 05 0072201, TÜV Rheinland Italia, valid 2025-05-08–2028-05-24 |
| 5 | "98% detection accuracy … against … hospital-based EEG" | K250515 | **Untraceable** — document reports PPA/FAR, states no clinical study on the subject device, real-world data from Embrace2; no figure reconstructs to 98% |
| 6 | Detects "generalized tonic-clonic seizures lasting 20 seconds or longer" | K250515; POS definition | **Match**, with scope narrowed by the regulator — POS states the device "does not predict seizure onsets, and is not intended as a standalone seizure monitoring device" |
| 7 | "Not designed to detect" non-motor seizures | — | **Match** (vendor states its own limit) |
| 8 | Sends "a phone call and a text message" to caregivers | — | **Documented only** — no runtime observation; the EULA explicitly declines to guarantee delivery |
| 9 | Adjustable sensing precision, high/low | K250515 | **Match, and independently described** — the clearance describes High-Sensitivity (rest) and Low-Sensitivity (low-intensity activity) modes and reports separate performance per mode |
| 10 | "Easily cancel false alerts before your caregivers are contacted" | — | **Documented only** — implies a cancellation window whose length the vendor does not publish; unobservable here |
| 11 | Subscription required for the device to work | EULA + store | **Match** — store states it; EULA requires registration for certain features |
| 12 | Caregiver count by plan (1/3/10) | store page | **Match** — stated in the plan comparison |
| 13 | "The phone needs to be kept within 15 feet" | — | **Match** (vendor states it) |
| 14 | "Over 300 digital measures" | — | **Unverifiable** — no enumeration, no definitions, no units published on the surfaces read |
| 15 | Sensors: ventral EDA, 4-channel multi-wavelength PPG, 3D accel/gyro, digital temperature | — | **Documented only** |
| 16 | "Bluetooth 5 with encrypted pairing and data transfer" | — | **Documented only** — no implementation evidence available |
| 17 | "No black box algorithms" | same page's "AI-powered" | **Conflict** — recorded, unresolved, conservative reading governs |
| 18 | Battery "up to 7 days" | Cloud API page's "up to 28 days" | **Conflict** — recorded, conservative value governs |
| 19 | "compliance with industry standards including HIPAA, GDPR and ISO" | `/legal/` | **Partially supported** — ISO/IEC 27001:2022 and ISO 13485 statements published; HIPAA and GDPR compliance asserted on a marketing page with no artifact published for either |
| 20 | "over 100,000 devices deployed", "160+ countries", "40+ languages" | — | **Unverifiable** — vendor-published figures, no independent source |
| 21 | Privacy notice covers watch, App and Portal; updated 2024-12-01 | notice text | **Match** — quoted verbatim from the notice |
| 22 | EULA: no delivery guarantee, no accuracy warranty, liability capped, no liability for injury or death | EULA text | **Match** — quoted verbatim |
| 23 | Data available as Avro/CSV in S3; full developer documentation to clients | `/cloud-api/` | **Match** as a documented contract — **no endpoint, schema, auth or rate-limit detail published**, so nothing is verified |
| 24 | "Data will be available in S3" + remote stop-collection + wearing detection | `/cloud-api/` | **Documented only** — no behaviour observed |
| 25 | E4/SDK withdrawn; software "rendering services inoperative" | sunset notice + commit history | **Match** — sunset 2025-02-14; docs emptied 2025-02-18 |
| 26 | The published User Manual is readable/searchable | local measurement | **Does not match** — no fonts, no text layer, 178 bytes extracted across 178 pages |
| 27 | User is "solely responsible" for caregiver data lawfulness; vendor "cannot verify or influence" caregiver conduct | EULA | **Match** — quoted verbatim |

**Tally: 27 checkable claims.** **12 match** as stated (1, 4, 6, 7, 9, 11, 12, 13, 21, 22, 25, 27 — counting
scope-narrowed items as matches with the narrowing recorded). **7 are documented-only or unverifiable** (8,
10, 14, 15, 16, 20, 24). **2 are partially supported** (19, 23). **6 do not match as stated** (2, 3, 5, 17,
18, 26).

**The pattern matters more than the tally.** Four of the eight failures — items 2, 3, 5 and 26 — are about
**the vendor's own evidence artifacts not supporting the vendor's own text**: an incomplete clearance
citation, a certificate that disclaims the claim beside it, a performance figure with no traceable source
against a regulator document that reports different metrics, and a manual that cannot be read. Items 17 and
18 are internal editorial inconsistency. Items 19 and 23 are partial support where the strong version is
asserted on a marketing page and the published artifact supports only the weak version. This is the same
shape recorded for `oscar` (header relabel; compression-level doc drift), `sleephq` (typo'd fields; a
"completely secure" FAQ against a privacy policy still "progressing towards" certification) and `nightscout`
(an audit that contradicts the code and the co-published requirements document).

**Documentation drift is a category of defect, not a one-off** — now confirmed on a **regulated medical
device**, where the artifacts are stronger, the stakes are higher, and the drift is still there.

---

## 8. Evidence gaps

Ordered by how much they would change the comparison.

| # | Gap | Would be resolved by | Not attempted because |
|---|---|---|---|
| G1 | **The manual's content** — does it say not to rely on the alert alone? | OCR of 178 image pages, or a text-format manual | No OCR tool installed; OCR error rate is a separate decision |
| G2 | **The 98% figure's source** | K181861 clinical study, or a peer-reviewed paper | Publications page not consulted; literature search not performed |
| G3 | **Runtime behaviour of any kind** — alert latency, cancellation window, out-of-range handling, lapsed-subscription behaviour | Running the app on a device with a subscription | No lawful binary; iOS-only; prescription-gated in the US |
| G4 | **The Cloud API's actual contract** | A client-facing API document | "Provided" only to commercial clients; no public artifact |
| G5 | **Android presence** | Play listing | Four candidate identifiers returned 404; Play returns 403 to non-browser clients, so this is not evidence of absence |
| G6 | **Plan detail** (annual pricing, feature differences) | The support-centre article | HTTP 403 behind Cloudflare |
| G7 | **K232915 summary** | `accessdata.fda.gov/cdrh_docs/pdf24/K232915.pdf` | Returned 404 at this date; K250515 covers the same device name and product code |
| G8 | **HIPAA/GDPR artifacts** | A DPA, BAA or compliance report | The EHMP **Data Processing Agreement** is listed on `/legal/` but was not fetched |
| G9 | **What "corrected PPA" means** | The full statistical methods section | Outside the excerpt read; the term is undefined in the summary as read |
| G10 | **Whether a security.txt, pen-test report or vulnerability-disclosure programme exists** | Vendor publication | Not published on the surfaces read; absence is not evidence of absence |

**G1 and G2 are the two worth closing**, and both are cheap relative to their value. G1 because it is the one
surface where the vendor speaks to the wearer in plain language and it is the only route to the
standalone-reliance question; G2 because an 8-point gap between a headline accuracy figure and the
regulator's reported agreement statistics deserves either a source or an explicit record that none exists.

---

## 9. Ethical and legal limits

- **No protection was circumvented.** No binary was acquired, so the question never arose. If an APK or IPA
  had been obtainable, certificate pinning, DRM, obfuscation and anti-debugging would have been respected as
  limits, not obstacles — consistent with the standing rule in `PROMPT1.md`.
- **No credentials, no accounts, no trials, no purchases.** No prescription was sought or implied.
- **No vendor API was contacted.** Not the Care Portal, not the Research Portal, not any Cloud API host.
  Existence is recorded from the vendor's own links; behaviour is asserted nowhere.
- **No personal data was collected or processed.** No caregiver contact list, no participant data, no device
  identifiers. The EULA's clause making the *user* solely responsible for the lawfulness of caregiver contact
  data is itself a finding, and this inspection collected none.
- **Third-party developer-machine paths and personal identifiers are not reproduced.** As with `oscar`, any
  such string found in a public artifact would be cited as "a tester's machine path (redacted here)".
- **Retrieved documents are not redistributed.** They live in `/tmp/aw2research/empatica/` and are identified
  by SHA-256 so the inspection is reproducible without republishing the vendor's material or a 112.8 MB
  device manual. Quotations are short, attributed and necessary to the finding.
- **The regulator record was queried, not interpreted.** Product codes were looked up in the classification
  database and their definition text is quoted verbatim; no classification was inferred from a device name.
- **Nothing here is medical advice, and nothing here characterises the product as unsafe.** The findings are
  about the traceability and completeness of the vendor's published evidence, not about clinical merit. A
  510(k) clearance with bench-equivalence and retrospective analyses is a lawful and common route to market;
  the observation is that the marketing page does not describe the route it took.

---

## 10. Reproduction

```sh
mkdir -p /tmp/aw2research/empatica && cd /tmp/aw2research/empatica
UA='Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/120.0 Safari/537.36'

# Vendor marketing, store, contract and compliance surfaces
for u in empatica.com/embraceplus/ empatica.com/epimonitor/ empatica.com/store/epimonitor/ \
         empatica.com/cloud-api/ empatica.com/embracemini/ empatica.com/manuals/ \
         empatica.com/legal/ empatica.com/epimonitor/privacy-policy-us \
         empatica.com/epimonitor/seizure-forecasting/; do
  f=$(echo "$u" | tr '/.' '__'); curl -sS -A "$UA" -L "https://www.$u" -o "web_$f.html"
done
curl -sS -A "$UA" -L 'https://s3.amazonaws.com/box.empatica.com/embrace/alert/agreements/eula/0.2-black.html' -o web/eula.html

# Independent regulator records — queried, never inferred
curl -sS 'https://api.fda.gov/device/510k.json?search=applicant:"empatica"&limit=100' -o fda_empatica_all.json
for pc in POS DQA MWI LEL GYD; do
  curl -sS "https://api.fda.gov/device/classification.json?search=product_code:$pc" -o "pc_$pc.json"
done
curl -sS 'https://accessdata.fda.gov/cdrh_docs/pdf25/K250515.pdf' -o fda_K250515.pdf
pdftotext -layout fda_K250515.pdf fda_K250515.txt

# App metadata (third-party, platform-level only)
curl -sS 'https://itunes.apple.com/lookup?bundleId=com.empatica.healthmonitor.alert&country=us' -o itunes_epimonitor.json
curl -sS 'https://itunes.apple.com/lookup?bundleId=com.empatica.healthmonitor&country=us'       -o itunes_care.json
# note: a bundleId lookup also returns unrelated apps; match on bundleId, not on position
# EpiMonitor trackId 1669234452 · Empatica Care trackId 1510409984

# Documentation lifecycle
curl -sS 'https://api.github.com/repos/empatica/empatica.github.io/commits?per_page=100' -o gh_commits.json
curl -sS 'https://raw.githubusercontent.com/empatica/empatica.github.io/HEAD/CNAME'
curl -sS 'https://api.github.com/orgs/empatica/repos?per_page=100' -o gh_repos.json

# The manual: retrieve, then test the text-layer hypothesis rather than assuming it
curl -sS -A "$UA" -L 'https://empatica.com/go/epimonitor-user-manual' -o web/epimonitor_manual.pdf
pdfinfo web/epimonitor_manual.pdf      # expect: 178 pages, 112,765,579 bytes
pdffonts web/epimonitor_manual.pdf     # expect: empty font table
pdftotext -raw web/epimonitor_manual.pdf - | wc -c   # expect: 178

# Hashes for the artifact record
shasum -a 256 web/*.html web/*.pdf fda_* itunes_epimonitor.json
```

**Tooling required:** `curl`, `poppler` (`pdfinfo`, `pdffonts`, `pdftotext`), `qpdf`, `jq` or `python3`,
`shasum`. All were present except an OCR engine, which is why §8 G1 remains open. No proxy, emulator, `adb`
or device was used for this candidate — the Android traffic-inspection stack was deliberately not employed,
because there is no lawfully obtainable target.

## Related files

- `empatica_CLAIMED_FEATURES_AND_PURPOSE.md` — Phase 1 public-claims research and the evidence ledger
- `PROMPT_2.md` — candidate-specific second-stage research brief
- `empatica_COMPARISON.md` — the comparison against AnxietyWatch v1 and AnxietyWatch2
- `../PROCESS_STATE.md` — pipeline state, residual uncertainty and active-candidate line
- `../RESEARCH_METHOD.md` — evidence classes, inspection modes and standing sub-checks
