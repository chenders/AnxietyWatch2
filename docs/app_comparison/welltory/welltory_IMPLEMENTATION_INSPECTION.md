# Welltory — implementation inspection

**Slug:** `welltory`
**Inspection date:** 2026-10-06
**Phase:** Phase 2A (authorized implementation inspection)
**Outcome:** **PARTIALLY PERFORMED** — the vendor's published research code and datasets were
inspected from public source. The shipped iOS/Android application was **not** inspected.

---

## 1. Summary of outcome

| Target | Inspected? | Basis |
|---|---|---|
| iOS application (`id1074364617`, bundle not confirmed) | **No** | Closed source, App Store distribution only |
| Android application (`com.welltory.client.android`) | **No** | Closed source, Play Store distribution only |
| Web application (`app.welltory.com`) | **No** | Closed source; account required to reach any authenticated surface |
| Published source code (GitHub org `Welltory`) | **Yes** | Public, openly licensed, no authentication required |
| Published research datasets (`hrv-covid19`, `welltory-ppg-dataset`) | **Yes** | Public, CC0-1.0 licensed |
| Peer-reviewed publications cited by the vendor | **Yes (metadata)** | Verified independently; full text not obtained here |

This is a materially better inspection position than candidate 1 (`bearable`), which published nothing.
Welltory publishes **no product source code** — the org contains no iOS or Android client code — but it
does publish the **research artifacts behind its headline accuracy claim**, and those are inspectable
and independently verifiable. That lets several of the vendor's public claims be reconciled against
primary evidence rather than left unresolved.

## 2. Authorization basis

| Requirement | Status |
|---|---|
| Artifact obtained from a publisher-authorised channel | **Yes** — `github.com/Welltory` and the GitHub API. Public repositories, public licences (CC0-1.0, MIT) |
| Publisher's terms permit inspection | **Yes** — CC0-1.0 and MIT are permissive licences expressly permitting use and redistribution |
| Circumvention of any technical protection measure | **None performed.** No DRM, no authentication, no pinning, no anti-debugging was bypassed at any point |
| Store-only application binary examined | **No** — not obtained, not decompiled |
| Account or credentials used | **None** |
| Personal or private data accessed | **None** — the datasets examined are deliberately anonymised and published for research; no subject-identifying fields were sought or retained |

The store applications remain authorised-for-inspection-in-principle but were not inspected, for the
same reasons recorded for `bearable`: Play-Store-only distribution, no lawful public binary source,
third-party mirrors unauthorised, and an account gate. **This means the Android and iOS implementations
were not examined and no statement about the binary is made anywhere in this report.**

## 3. Artifact record — inspected

| Field | Value |
|---|---|
| Artifact type | Public GitHub organisation (source + data) |
| Location | `https://github.com/Welltory` |
| Discovery method | Vendor website footer link "Open Source" → GitHub organisation |
| Retrieval method | GitHub REST API (`/orgs/welltory/repos`, `/repos/{owner}/{repo}`, `/repos/{owner}/{repo}/readme`) |
| Retrieval date | 2026-10-06 |
| Repository count | 13 |
| SHA-256 / commit pin | Not pinned. Findings below are tied to refs read at the dates given; repository state is mutable |

**Repository inventory (13), with the relevance of each:**

| Repository | Language | Licence | Relevance to product claims |
|---|---|---|---|
| `hrv-covid19` | n/a (data) | CC0-1.0 | **High** — open COVID-19 HRV dataset; vendor's own research basis |
| `welltory-ppg-dataset` | n/a (data) | CC0-1.0 | **High** — PPG-vs-Polar paired dataset behind the accuracy claim |
| `python-foobot` | Python | MIT | **Medium** — confirms indoor air-quality integration (Foobot) |
| `python-cardioqvark` | Python | MIT | **Medium** — confirms CardioQVARK HRV device integration |
| `python-baremetrics` | Python | — | Low — internal billing/analytics tooling |
| `Zoom2Youtube` | Python | — | None — unrelated internal utility |
| `stayinghomeclub` | — | — | None — COVID-era side project |
| `auth-without-play-services` | Kotlin | fork | **Low** — fork relevant to Huawei/Galaxy distribution |
| `Charts`, `SwiftGen`, `ConnectSDK-iOS`, `DeviceUtil`, `VisualEffectView` | Swift/Obj-C | forks | None — third-party forks, no original product code |

**Critical negative finding: the organisation contains no application source code.** There is no iOS
client, no Android client, no analysis engine, and no server code. The claim in §7 of the claims file —
that core functionality relies on the vendor's server-side analysis — is therefore **consistent with**
the absence of a published client, but is not proven by it. An app could exist and simply not be
published. This is recorded as an observation, not as proof of a closed architecture.

## 4. Static findings from published source

### 4.1 `welltory-ppg-dataset` — the PPG-vs-Polar accuracy basis

This repository is the primary evidence behind the vendor's public claim that phone-camera PPG is
equivalent to a Polar chest strap. Directly from the published README:

- **Design:** simultaneous recording — smartphone-camera PPG from the app, and RR intervals from a
  **Polar H10** chest strap worn at the same time.
- **Sample:** **21 recordings from 13 healthy volunteers aged 25–35**, each making 1 or 2
  measurements on their own Android phone.
- **Signal length:** **68 to 112 seconds** per record.
- **Protocol:** index finger on the smartphone camera; each frame's capture time in milliseconds and
  the mean red, green and blue channel values recorded; Polar RR intervals collected in milliseconds.
- **Per-record contents:** `PPG.csv` (frame time + R/G/B) and `RR.txt` (Polar RR in ms), in
  `subject_01` … `subject_21`.
- **Licence:** CC0-1.0 — dedicated to the public domain.
- **Created:** 2021-06-29; last push 2021-10-15 (i.e. **not updated in ~5 years**).
- **Correspondence:** a `welltory.com` address.
- **Cited publication:** Neshitov, A.; Tyapochkin, K.; Smorodnikova, E.; Pravdin, P. *Wavelet Analysis
  and Self-Similarity of Photoplethysmography Signals for HRV Estimation and Quality Assessment.*
  **Sensors 2021, 21, 6798.**

**Interpretation.** The published ground truth is the **Polar H10** — the same device AnxietyWatch uses.
This is the closest thing on the shortlist to a directly comparable measurement-validity question: a
PPG pipeline is being assessed against H10-derived RR intervals. The vendor therefore already treats
H10 RR as the reference standard, which is exactly AnxietyWatch's design assumption.

**Scope limits, stated plainly.** Twenty-one recordings from thirteen healthy adults aged 25–35,
at rest, finger on camera, 68–112 seconds each, do not establish equivalence for: other age groups or
clinical populations; motion; poor perfusion; cold hands; or durations relevant to overnight recording.
The dataset supports a claim about **short, resting, finger-on-camera PPG quality assessment**; it does
not support a general "as accurate as ECG" claim, and the store copy's broader phrasing exceeds it.

### 4.2 `hrv-covid19` — open dataset and research direction

From the published README:

- Created 2020-03-26 in response to COVID-19; data pushed through 2022-07-22; CC0-1.0.
- Collected from **users with positive COVID-19 status** who tracked symptoms, HRV, and connected-gadget
  data. HRV came from "any Bluetooth-enabled heart rate monitor or with a smartphone camera".
- Notably includes "**Clinically validated physical and mental health assessments**" — a feature
  "created specifically for this project", where participants added symptom and test-result information.
- Dataset released as CSV with a published `datatypes.md` schema.
- The vendor states the collection basis was **non-commercial**.
- **Citation asymmetry worth noting:** the vendor's own resulting paper (below) is a **preprint**,
  whereas the dataset was used in at least three **peer-reviewed** papers by external groups.

### 4.3 Peer-reviewed and preprint record — verified

All citation metadata below was verified independently against Crossref and Europe PMC on 2026-10-06,
not taken from the vendor's page.

| # | Work | Venue | Date | Authors | Verification |
|---|---|---|---|---|---|
| P1 | Wavelet Analysis and Self-Similarity of Photoplethysmography Signals for HRV Estimation and Quality Assessment | **Sensors** (MDPI), 21(20):6798 | 2021-10-13 | Neshitov, Tyapochkin, Smorodnikova, Pravdin | ✅ Crossref, `10.3390/s21206798` |
| P2 | Heart Rate Variability as a Prospective Predictor of Early COVID-19 Symptoms | **medRxiv preprint** | 2021-07-05 | (Welltory team) | ✅ Europe PMC, `10.1101/2021.07.02.21259891` |
| P3 | Pre-Emption of Affliction Severity Using HRV Measurements from a Smart Wearable; Case-Study on SARS-Cov-2 Symptoms | **Sensors** (MDPI), 20(24):7068 | 2020-12-10 | Varma, Chauhan, Singh, Singh | ✅ Crossref, `10.3390/s20247068` |
| P4 | When Patients Recover From COVID-19: Data-Driven Insights From Wearable Technologies | **Frontiers in Big Data** | 2022-04-28 | Guo, Nguyen, Du, Jin | ✅ Crossref, `10.3389/fdata.2022.801998` |
| P5 | Wearable Devices, Smartphones, and Interpretable Artificial Intelligence in Combating COVID-19 | **Sensors** (MDPI), 21(24):8424 | 2021-12-17 | Hijazi, Abu Talib, Hasasneh, Bou Nassif, Ahmed, Nasir | ✅ Crossref, `10.3390/s21248424` |

**What this establishes and what it does not.**
- **Establishes:** the vendor's research activity is real, published, and its datasets have been reused
  by independent groups (P3–P5). This is a genuine credibility signal that `bearable` could not match.
- **Does not establish product validity.** P3–P5 are third-party studies that *used the dataset*; they
  evaluate their own models, not the Welltory product. P1's authors are the Welltory team itself, so it
  is peer-reviewed but **not independent** of the vendor. P2 is a **preprint** — not peer-reviewed —
  and rests on **14 eligible subjects**.
- The load-bearing accuracy claim therefore rests on one small, vendor-authored, peer-reviewed study
  (n=13, resting, finger on camera) plus the vendor's own preprint (n=14). Both sample sizes are small.

### 4.4 Integration wrappers — corroborating environment claims

- **`python-foobot`** (MIT): a wrapper for the Foobot **indoor air-quality monitor** API, with a README
  stating "Welltory integrated with foobot.io". This independently corroborates, at source level, the
  claims-file finding that Welltory ingests indoor environmental hardware. It also **adds Foobot indoor
  air quality** to the environment list alongside Netatmo and Withings Body+ CO₂.
- **`python-cardioqvark`** (MIT): a wrapper for **CardioQVARK**, a CE-marked smartphone-ECG/HRV device.
  This is a further device family beyond the consumer straps enumerated in the support articles.
- Neither wrapper is part of the shipped app; both are integration tooling published by the vendor.

## 5. Runtime observation

**Not performed.** Rationale:

1. The Android application was not obtained (Play-Store-only; no authorised public binary).
2. The application requires an account for its primary surfaces, and creating one is outside the brief's
   authorisation.
3. There is no free, unauthenticated surface that would yield an honest runtime observation, and a
   logged-out splash screen would not produce comparable evidence.

The toolchain was validated against a disposable emulator on the `bearable` pass
(mitmdump 12.2.3 LISTEN on `127.0.0.1:8080`; AVD `pixel_4_w_gapps`, Android 12 / SDK 31; proxy
`10.0.2.2:8080` set and read back; HTTP flows captured and read back; `google.com/gen_204` and
`connectivitycheck.gstatic.com/generate_204` observed returning 204; HTTPS failing with "client does
not trust the proxy's certificate" as expected without a CA; proxy and emulator then removed and
verified gone). That validation stands, and is not re-run here because there is no target artifact to
exercise. See `../bearable/bearable_IMPLEMENTATION_INSPECTION.md` §4.1 for the full record.

**Any statement in `welltory_COMPARISON.md` about app runtime behaviour is therefore absent by design.**
Where the comparison describes how Welltory behaves, it is describing documented behaviour, and is
labelled as such.

## 6. Claim reconciliation

Claims from `welltory_CLAIMED_FEATURES_AND_PURPOSE.md`, reconciled against the evidence actually
obtained.

| # | Public claim | Source class | Inspection result | Status |
|---|---|---|---|---|
| 1 | Personal-baseline HRV algorithm instead of one-size-fits-all thresholds | Vendor claim (support article) | No analysis code published; algorithm not inspectable | **Unresolved by inspection** — consistent with the absence of published source, not verified |
| 2 | A fixed medical threshold overrides the baseline for chronic conditions, causing constant alerts | Vendor's own support article | Confirmed as a vendor *statement*; the mechanism is not inspectable | **Vendor-documented limitation** (not verifiable in code) |
| 3 | Phone-camera PPG equals Polar chest straps / "as accurate as ECGs" | Store copy + support article | **Partly corroborated.** A public CC0 dataset with **Polar H10** ground truth and a peer-reviewed method paper exist. Sample is n=13 healthy adults 25–35, resting, 68–112 s | **Narrower than claimed.** Supported for short resting finger-on-camera conditions; store wording generalises beyond the evidence |
| 4 | Research validated Welltory for HRV in elite endurance athletes (Apr 2022) | Vendor science page | **Not found.** No such paper located; science-page citation not retrieved | **Unverified** — vendor claim stands unsupported here |
| 5 | Results presented at the 41st IEEE EMBC, Berlin | Vendor science page | **Not verified** in this pass | **Unverified** |
| 6 | "Backed by over 25,000 studies on PubMed" | Vendor copy | Refers to HRV literature generally, not to Welltory | **Misleading framing** — not a product-validity claim |
| 7 | Indoor environment integration (air quality, CO₂ temperature/humidity) | Support articles | **Corroborated at source.** `python-foobot` (indoor air quality) and `python-cardioqvark` published by the vendor | **Supported** |
| 8 | Weather auto-integrated by geolocation | Support article | Not inspectable (client-side/server-side processing unpublished) | **Unresolved by inspection** |
| 9 | AI Coach is external on OpenAI ChatGPT, requires Premium **and** ChatGPT Plus, beta | Vendor support article | Not inspectable; no AI code published | **Unresolved by inspection** |
| 10 | No data sale; health data not shared for advertising | Privacy policy | Not inspectable. Advertising pixels (Meta, Google, TikTok) and analytics SDKs (Amplitude, AppsFlyer, Firebase, Crashlytics) are named in the vendor's own policy | **Unresolved** — vendor-disclosed ad/analytics footprint; SDK presence in the binary not verified |
| 11 | Core functionality relies on server-side health-data analysis | Privacy policy | **Consistent with** the absence of any published client or engine code. An unpublished closed client would also explain it | **Consistent, not proven** |
| 12 | Free tier deletes data older than 30 days | Support articles (two independent articles) | Documentation only; behaviour not observed | **Unresolved by inspection** |
| 13 | CPAP / AHI / respiratory-event support | Absence across all vendor surfaces | No code or data artifact references PAP therapy. `hrv-covid19` schema is the only published schema | **No evidence of the capability.** Absence of evidence — not proof of absence in the binary |
| 14 | Medication / dose tracking | Absence across all vendor surfaces | Nothing found in published data schemas or repos | **No evidence of the capability** |

**Rows 1, 2, 8, 9, 10, 12 remain genuinely unresolved**, and the comparison report must say so rather
than treating them as settled in either direction.

## 7. Evidence gaps

1. **No product binary inspected** — no iOS or Android artifact; all client behaviour is documented
   behaviour.
2. **No analysis code inspected** — the HRV scoring, baseline adaptation, episode detection and the
   fixed medical threshold have no published implementation.
3. **Full texts of P1–P5 not read** — only metadata (title, venue, date, authors, DOI) was verified.
   Effect sizes, quality-assessment methods and the actual PPG-vs-H10 agreement figures are unknown.
4. **The April 2022 athlete-validation study and the IEEE EMBC presentation were not located**, so two
   science-page claims stand unverified.
5. **`datatypes.md` schemas not parsed** — the published schemas would show which measurements the
   vendor actually models, and could narrow or close claims 13 and 14. Not done in this pass.
6. **Android data-safety "3 others"** (shared and collected) unresolved.
7. **Full pricing history and regional pricing** not established; only the plans page at the research
   date and the Play IAP range.
8. **App Store listing not retrieved** (guessed URL 404'd), so iOS-only figures are missing.
9. **Nothing was observed at runtime.** There is no packet capture, no log, no screenshot and no
   recorded interaction for Welltory. Any future claim about app behaviour needs that pass.

## 8. Ethical and legal limits applied

- No DRM, authentication, licence check, encryption, certificate pinning, anti-debugging or platform
  protection was bypassed — none was encountered, because no product binary was examined.
- No credentials, tokens or personal data were sought or extracted. The examined datasets are
  deliberately public, anonymised research data.
- No aspect of the vendor's service was accessed destructively or abusively; no production login was
  attempted.
- All GitHub access used documented public API endpoints under the publishers' permissive licences.
- Where the vendor's copy overstates what its own evidence supports, that is reported as a
  documentation discrepancy with the evidence cited — not as an accusation, and not as evidence that
  the product underperforms.

## 9. Reproduction

```sh
# Repository inventory
curl -sS "https://api.github.com/orgs/welltory/repos?per_page=100"

# Datasets and their READMEs
curl -sS -H "Accept: application/vnd.github.raw" \
  "https://api.github.com/repos/welltory/welltory-ppg-dataset/readme"
curl -sS -H "Accept: application/vnd.github.raw" \
  "https://api.github.com/repos/welltory/hrv-covid19/readme"

# Citation verification (independent of the vendor's pages)
for d in 10.3390/s21206798 10.3390/s20247068 10.3389/fdata.2022.801998 10.3390/s21248424; do
  curl -sS "https://api.crossref.org/works/$d" | python3 -m json.tool | head -30
done
curl -sS "https://www.ebi.ac.uk/europepmc/webservices/rest/search?query=DOI:%2210.1101/2021.07.02.21259891%22&format=json&resultType=core"
```

## 10. Related files

- Public claims: `welltory_CLAIMED_FEATURES_AND_PURPOSE.md`
- Candidate-specific prompt: `PROMPT_2.md`
- Comparison report: `welltory_COMPARISON.md`
- Method and evidence classes: `../RESEARCH_METHOD.md`
- Traffic-inspection stack (validated, unused here): `../FREE-ANDROID-TRAFFIC-INSPECTION.md`
