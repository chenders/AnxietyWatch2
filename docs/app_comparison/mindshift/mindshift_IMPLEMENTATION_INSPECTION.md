# MindShift CBT — implementation inspection (Phase 2A)

| Field | Value |
|---|---|
| **Candidate** | MindShift CBT — `mindshift` (shortlist #10) |
| **Vendor / operator** | **247 Labs Inc** (App Store `sellerName`, Google Play developer); Anxiety Canada is the *original* development partner, named in the past tense |
| **Research date** | 2026-10-06 |
| **Phase 2A outcome** | **Performed — published web client and public records (no app binary; no runtime observation).** |
| **Scratch root** | `/tmp/aw2research/mindshift/` |
| **Companion files** | `mindshift_CLAIMED_FEATURES_AND_PURPOSE.md`, `PROMPT_2.md`, `mindshift_COMPARISON.md` |

---

## Inspection-mode table

| Surface | Available? | Used? | What that means here |
|---|---|---|---|
| **Published web client** (served JavaScript) | yes | **yes** | The 741,896-byte `index-BogFfh2L.js` was downloaded and read as **text**. This is *static source reading of a published artefact*, not a runtime, not a binary, not decompilation. |
| **Vendor marketing pages** | yes | **yes** | Read as served documents (SEO shell + client-rendered copy). |
| **App Store / Google Play listing metadata** | yes | **yes** | Storefront pages plus the iTunes Lookup record; treated as a **third-party** surface, not a vendor claim. |
| **App Store "App Privacy" panel** | yes | **yes** | Read as a **developer-supplied** declaration that Apple labels unverified (§3.3). |
| **Independent clinical registry** (ClinicalTrials.gov API v2) | yes | **yes** | Retrieved as JSON; the registry record is a **third-party** artifact. |
| **Peer-reviewed publication + corrigendum** (Europe PMC / PMC) | yes | **yes** | Full text plus the journal record; a **third-party** artifact and the vendor's own authors' work — not independent validation of the product. |
| **App binary (iOS `.ipa` / Android `.apk`)** | **no** | **no** | Not obtainable lawfully for this candidate; Phase 2A binary inspection **not performed**. |
| **App runtime (launched app, proxy capture)** | **no** | **no** | No emulator run, no account, no traffic capture. |
| **Vendor API contract / OpenAPI specification** | **no** | n/a | None published; the site's own endpoints are undocumented (§4.2). |
| **Vendor source repository** | **no** | **no** | Closed source. |

**Consequence for evidence classes.** Every finding in this file is either a `[PUBLIC CLAIM]` by the vendor,
a `[THIRD PARTY]` record, or a static reading of the **published client**. **The package's
`[FOUND IN PUBLISHED CLIENT]` class is a static class.** Nothing in this file is a runtime observation, and
nothing in this file has been upgraded between classes.

---

## 1. Summary of outcome

**Phase 2A was performed, on one real surface, and stopped where the law and the artifacts stop.**

What was available and used was the vendor's **published web client** — the JavaScript bundle its own site
ships to every visitor — together with a set of **public records** (two storefront listings, the App Store
privacy panel, the ClinicalTrials.gov registration, and the peer-reviewed paper plus its corrigendum).

Five things came out of it that a claims-only reading would have missed:

1. **The storefront-declared privacy policy URL is dead on both storefronts.** Google Play and the App Store
   both point at `https://mindshiftcbt.com/privacy/`. That URL, and every plausible variant of it, returns a
   genuine HTTP 404 with a real Not-Found document. This was verified **five independent ways** — the direct
   probe, the sitemap, the SPA router table, a fixed-string sweep of the bundle, and a same-day re-fetch that
   returned byte-identical content (§3.3). This is the strongest *verified absence* in the whole package.
2. **The site ships its own visitor telemetry and publishes no notice for it** (§5.1). `POST
   /api/analytics/session` and `POST /api/analytics/events`, per-session UUID, four event names, page paths,
   `credentials:"include"`, silent failure — in a bundle that contains **no** cookie notice, **no** consent
   banner and **no** analytics disclosure, on a site that has no privacy page to disclose it on.
3. **The registration classifies the study differently from the way the study and the site describe it**
   (§5.2): `studyType: OBSERVATIONAL`, `observationalModel: ECOLOGIC_OR_COMMUNITY`, **no intervention
   module**, **one** registered primary outcome. The paper's title says "trial", the paper's text says "four
   primary outcomes", the site says "clinical trial".
4. **The vendor's own funded publication is the counter-surface to the vendor's marketing** (§5.3): the paper
   reports that improvement was *unrelated to app-use frequency*, that effects were *moderate*, and that
   "the lack of a comparison group … limits the causal inferences". The site renders "**Clinically proven**".
   The marketing sentence the site does quote — "all four primary outcomes" — is the one the registry does
   not support.
5. **The app that the marketing describes is not the app that the clinical record describes** (§5.4): the
   study's verified-install subset is **156 of 380**, so 224 participants never demonstrably installed the
   product, while the headline figure everywhere is 380.

Two structural positives are recorded as positives, not as consolation: the scope-boundary language
("not a medical device … not itself a crisis service") and the explicit refusal of a clinician portal in
favour of patient-controlled export.

**What was not done, and why.** The app binary was not inspected — it cannot lawfully be obtained for this
candidate. No app was run, no account was created, no endpoint was called, no proxy was configured.

---

## 2. Authorization basis

The inspection was confined to artifacts that the vendor itself publishes to unauthenticated visitors, plus
records that third parties publish about the vendor.

| Action | Basis | Limit observed |
|---|---|---|
| Retrieve the site's HTML and served JavaScript | public documents served to any visitor | no authentication, no account, no session |
| Retrieve the store listings and the iTunes Lookup record | public storefront pages and a public metadata API | no developer account, no store credentials |
| Retrieve `ClinicalTrials.gov` API v2 record `NCT04562948` | public U.S. government registry API | read-only GET |
| Retrieve the paper and corrigendum from Europe PMC / PMC | open-access published literature | read-only GET |
| Read the App Store "App Privacy" block | public storefront page | recorded as **developer-supplied and unverified by Apple**, per Apple's own label |
| Probe eight candidate privacy/terms paths | public URL probes | recorded as status codes in §3.3; **no attempt to discover a private document** |

**What was explicitly not done.** No app binary was acquired, decrypted or decompiled. No certificate
pinning, authentication, licensing, DRM or platform protection was bypassed. No vendor endpoint was called
with any payload. No credential, token, private datum or account was used. The vendor's own endpoints
`/api/analytics/*`, `/api/admin/*` and `/api/resources/*` were **read about** in the served JavaScript and
**never contacted**.

---

## 3. Artifact record

### 3.1 Primary artifact

**`web/index-BogFfh2L.js` — the published web client.**

| Property | Value |
|---|---|
| Bytes | **741,896** |
| SHA-256 | `1bb1681128467a90803d37652bc7eb8d65e209a8dbdd867a5a260e8cba1ef828` |
| Retrieved | 2026-10-06, Chrome user-agent, unauthenticated |
| Nature | A single bundled React SPA served to every visitor of `www.mindshiftcbt.com` |
| Why it is the primary artifact | It is the vendor's own published code. It establishes **what the site is written to do** — its routes, its telemetry, its third-party processors and its failure handling — which no marketing page states. |

**It is a client, not a binary and not a runtime.** All findings derived from it are classed
`[FOUND IN PUBLISHED CLIENT]` and are static. A string in this file is not a behaviour; a `fetch(` call is
not a request that was made.

**A second, distinct artefact matters and must not be confused with it.** `i.replit.com/script.js` appears
in the **served HTML shell of every document, including the 404 pages**, and appears **0 times** in the
bundle. Attributions about hosting or analytics must name the *shell* or the *client* explicitly; this
distinction was recorded as a standing rule during the MindShift pass.

### 3.2 Files retrieved

All retrieved 2026-10-06 with a Chrome user-agent. SHA-256, each computed from the file on disk:

**Vendor surfaces and the published client**

| Artifact | Bytes | SHA-256 |
|---|---|---|
| `web/home.html` (SEO shell) | 3,918 | `37687122e030377d2e72f62fc61f4388c3104c92b95556c4bca4fecbf2b379f5` |
| `web/index-BogFfh2L.js` (**published client**) | 741,896 | `1bb1681128467a90803d37652bc7eb8d65e209a8dbdd867a5a260e8cba1ef828` |
| `web/ms_about.html` | 3,861 | `ab58ecd4cf38e369a5bdfa2775e4003ee9e49b391a9c38c3b3d3e05614853b16` |
| `web/ms_app.html` | 4,339 | `36b5d99a8286a1042ad4f01242af943bdaa990c5ec87e3284dd47ba9cc12ebe8` |
| `web/ms_contact.html` | 3,876 | `71eab6dd36ee84d9b21d39b89433d9453bfc8d56069daf9cdf338613edf55dc7` |
| `web/ms_faq.html` | 6,828 | `dea8a01f057f3e4102676fcbf74f0d0922f20b6da9eab67b7f1265cc1fdb9c8c` |
| `web/ms_for-clinics.html` | 5,645 | `8e2cbe785e798aa9f92ae9c095e85191e09c0c111dff5c5f16463bf9a7c4e30c` |
| `web/ms_institutions.html` | 3,919 | `3c09ffdd1d23b7b04a9c9dfbecc9f70ab88ae01ae6e094675d7a63c26a1712a6` |
| `web/ms_methodology.html` | 3,889 | `ee5cff8e804c60908db3695f4e412ab3a3e6761b98bb183f12de1fcffbd50049` |
| `web/ms_resources.html` | 4,643 | `3e538958e617ab74eaa40ef1281833c61379b2220d03e0ab32cf818f42e54f4a` |
| `web/sitemap.xml` | 1,160 | `646c9740caa92c3166fea8afca74784bee07f2349d7bcfc22c878b2cfeca06c2` |
| `web/robots.txt` | 144 | `d119e9352b5f47b53876b2e9cf9d00d0de205cbc45143f12ea3dc47f8118a943` |
| `web/ms_privacy.html` (**404 document** for `/privacy/`) | 3,362 | `4a9ba979a9d4a5f0525e3f61f343fea0818a109cee05334ef237c952767fa3a7` |
| `web/ms_terms.html` (**404 document** for `/terms`) | 3,358 | `bc9ec2ddd69856c171265819821f84cb3380c518b5011ac36ed7aa78ac4fbb47` |
| `web/ms_privacy_20261006.html` (same-day independent re-fetch) | 3,362 | `4a9ba979a9d4a5f0525e3f61f343fea0818a109cee05334ef237c952767fa3a7` — **byte-identical to `web/ms_privacy.html`** |
| `web/probe_log.txt` (status codes for eleven probed paths) | 852 | `d2eae211983b770c1f2f9301f3350a18c288b666ee954cf86377feec8bc08957` |
| `web/bundle_strings.txt` (extracted string inventory) | 57,225 | `a15a83604b7dbcb1aabbb8ce95d5686bd1cf6e1ea89dc7d6b19e5d221bab7dd0` |
| `web/tx.py` (HTML→text extractor) | 344 | `97b9257115be5e6a31378bc8506c4099580b03c69c2a84ab4802f37c715afa50` |

**Anxiety Canada — a retrieval failure, recorded as such**

| Artifact | Bytes | SHA-256 |
|---|---|---|
| `web/ac_home.html` = `ac_mindshift.html` = `ac_ms1.html` = `ac_ms2.html` = `ac_privacy.html` = `ac_terms.html` (all six byte-identical) | 9,321 | `bacd434ff31aed1e42b609a2233d84b3b0d16a0a7b2b42f04b417acef179a8d6` |

Every `anxietycanada.com` path — including `/robots.txt` and `/sitemap.xml` — returned a **byte-identical
9,321-byte SPA shell** (`<title>Anxiety Canada</title>`). That is a **retrieval failure**, not an absence of
content, and it is why §8 lists Anxiety Canada's current statements about MindShift as `[UNKNOWN]`.

**Storefront records**

| Artifact | Bytes | SHA-256 |
|---|---|---|
| `web/appstore_ms.html` | 689,610 | `35a83c387d3f567eb99217742356d34359eb7d3c7153b2b26b780270fd47fd45` |
| `web/play_ms.html` | 1,206,296 | `8017dbc088c331449572be32adaf7b148bacc37420218b7d3516e5f141050a5c` |
| `api/itunes_com.bstro.mindshift.json` (iTunes Lookup, `trackId` 634684825) | 9,253 | `535edc20149c50737b1c8e4d1a6ee22f52bc31973b967255cbfe1365061d89af` |
| `api/itunes_ms.json` / `api/itunes_com.anxietybc.mindshift.json` / `api/itunes_ca.anxietycanada.mindshift.json` (all three identical, `resultCount 0`) | 42 | `7f5ce663b726607eae2fde2a6dc438052ac0d1681e636de7cc3377c0bb77e047` |
| `api/itunes_search.json` (24 results) | 250,197 | `5a2ed0783efc50abcf466e8461954d40a9da424a4d6a3f3e01c99520f20e50f6` |
| `api/itunes_ac.json` (18 results) | 185,910 | `0593422301d41fb9912e7db0ee4d942a8bf2e7e4b94e13be75ca1d08285c91db` |

A guessed-package-ID 404 is **not** absence of a listing: `com.bstro.mindshift` (lowercase) does not resolve,
while `com.bstro.MindShift` (capital S — the vendor's own bundle identifier case) does. The `resultCount 0`
records above are for *other* bundle IDs, and are retained to show what was tested.

**Independent and third-party records**

| Artifact | Bytes | SHA-256 |
|---|---|---|
| `api/ct_NCT04562948.json` (ClinicalTrials.gov API v2) | 20,342 | `55cb272ec50d5399801660e7fea173084fd90eb9df7c83c5c85ec317d9f402c4` |
| `api/epmc_mindshift.json` (Europe PMC search) | 164,775 | `07fc5916262506f2d68ddea99e42e76b89d2d82b1cb221344d277df9e74b82c0` |
| `api/epmc_40656972.json` (the paper's journal record) | 5,854 | `11a12ce0d4813a79ea65165d34b45eef02802ee68c4d80bc777c02a98b6b3b7d` |
| `api/epmc_40655914.json` (the corrigendum's journal record) | 3,950 | `901919c5e3ed61d278b005532f09b2cfa1fc20a1cf9551597801c4c1d2852778` |
| `api/fulltext_PMC12244177.xml` (paper full text) | 139,072 | `d39e93328326b88c5e77d70b443776f1f90116b4bc7beff00036de47e7976c7e` |
| `web/pmc_12244177.html` | 235,623 | `cdbfc0a90bae0750f84c737694ced9343ff77eb6709b973ef71b7fdf0a7eba83` |
| `web/pmc_12244218.html` (corrigendum) | 96,303 | `f541152e15c43be8efe890bb4182eceed99c88a4ae887baa7749b5f3c2f430d5` |

**Two failed retrievals, recorded as failures rather than as absent content.**
`api/fulltext_40656972.xml` and `api/fulltext_40655914.xml` each contain a **148-byte HTTP 500 body** from the
Europe PMC `/fullTextXML` endpoint. The fetch succeeded when re-issued against the **PMC id**
(`PMC12244177`) instead of the numeric PMID. **A 500 is a retrieval failure, not an absence**, and the
numeric-PMID failure is recorded here rather than quietly dropped. The corrigendum's full text was obtained
from its PMC page instead.

**An unverified hash is worse than no hash.** Every value in the three tables above was computed from the
file on disk. None was written from memory.

### 3.3 The privacy-URL verification chain

This finding carries the most weight in the file, so it was verified five independent ways.

**Link 1 — what the storefronts declare.**

- **Google Play** declares the app's privacy policy as `https://mindshiftcbt.com/privacy/` (retrieved from
  the rendered listing).
- The **App Store** page's "Privacy Policy" link is the **same URL**.

**Link 2 — the direct probe.** Eleven paths probed 2026-10-06 with a Chrome user-agent; full log at
`web/probe_log.txt`:

| Path | HTTP | Bytes | `<title>` |
|---|---|---|---|
| `/privacy` | **404** | 3,362 | `Page not found \| MindShift CBT` |
| `/privacy/` | **404** | 3,362 | same |
| `/terms` | **404** | 3,358 | same |
| `/terms/` | **404** | 3,358 | same |
| `/privacy-policy` | **404** | 3,376 | same |
| `/privacy-policy/` | **404** | 3,376 | same |
| `/legal` | **404** | 3,358 | same |
| `/legal/` | **404** | 3,358 | same |
| `/` (control) | **200** | 3,918 | `Free Evidence-Based CBT App for Anxiety \| MindShift CBT` |
| `/faq` (control) | **200** | 6,828 | `Anxiety App Frequently Asked Questions \| MindShift CBT` |
| `/resources` (control) | **200** | 4,643 | `Anxiety and CBT Guides and Resources \| MindShift CBT` |

The three controls matter: this is a **working site that returns 200 for real pages**, so the 404s are a
finding about those paths and not about the server.

**These are genuine Not-Found documents, not JS error shells and not WAF challenges.** The served HTML
carries `<title>Page not found | MindShift CBT</title>`, and the visible body reads, in full, "Page not
found" / "The requested MindShift CBT page could not be found." This distinction was made a package rule
during the Oura candidate, where a Next.js error shell was **not** a 404 and **not** evidence of absence.
(The bundle separately contains a localized SPA 404 component — "The page you're looking for moved or never
existed. Let's get you back to something helpful." — which is a *client* string and not part of the served
document.)

**Link 3 — the site's own inventory lists nothing.** `sitemap.xml` publishes nine `<loc>` entries — `/`,
`/methodology`, `/app`, `/institutions`, `/for-clinics`, `/about`, `/resources`, `/faq`, `/contact` — and the
SPA router table in the published client contains **thirteen** paths (those nine plus `/resources/:slug`,
`/sign-in/*?`, `/sign-up/*?`, `/admin/*?`). **No privacy path and no terms path in either.**

**Link 4 — a fixed-string sweep of the whole bundle.** In the 741,896-byte published client: `Privacy Policy`
→ **0**, `privacy policy` → **0**, `Terms of Use` → **0**, `Terms of Service` → **0**, `terms of use` → **0**,
`Terms` → **0**. Four `Privacy` strings survive, and all four are marketing or FAQ furniture (§3.4).

**Link 5 — a same-day independent re-fetch.** `/privacy` was fetched again and returned a **byte-identical**
3,362-byte document (SHA-256 `4a9ba979…`, unchanged). The finding is **stable within the research day**, not
a transient failure.

**What the vendor's own release notes say.** The App Store version history contains, for **v3.0.1**
(2021-12-17) and again for **v3.0.3** (2022-08-11): "We have integrated an online forum: MindShift CBT
Community… **The Terms of Use and Privacy Policy have also been updated to reflect this new feature.**" The
documents therefore existed in some form at those dates, and the URL the storefronts still declare is now
dead. **Whether they still exist inside the app is `[UNKNOWN]` — this inspection did not read the binary.**

**The finding is stated where it is:** this is a documentation-lifecycle and discoverability defect. It is
**not** a finding that the app is ungoverned, **not** a finding that data is mishandled, and **not** an
allegation of intent. §9 of the companion claims file draws the same line.

### 3.4 The four surviving `Privacy` strings

For completeness, because a zero count for `Privacy Policy` invites the wrong inference:

| String | Where |
|---|---|
| "**Privacy respected**" (feature-card title) with body "Your data is yours — securely stored, never sold, and exportable to your own clinician." | home / app surfaces |
| "**Privacy & data**" (FAQ category heading) | FAQ |
| "…and **how it protects your privacy**." | FAQ meta description |
| "Privacy respected" | feature-card title, second instance |

**Language about privacy is abundant; the document that language points at is absent.**

---

## 4. Static findings — architecture

Everything in this section is `[FOUND IN PUBLISHED CLIENT]` unless labelled otherwise. **None of it is a
runtime observation**, and none of it was confirmed by making a request.

### 4.1 The site is a single-page application with a server API

| Layer | What the bundle shows |
|---|---|
| Router | thirteen `path:` entries: `/`, `/about`, `/admin/*?`, `/app`, `/contact`, `/faq`, `/for-clinics`, `/institutions`, `/methodology`, `/resources`, `/resources/:slug`, `/sign-in/*?`, `/sign-up/*?` |
| Public data | `GET /api/resources` (query-key keyed), `GET /api/resources/:slug`, `POST /api/contact` |
| Admin data | `POST /api/admin/login`, `POST /api/admin/logout`, `GET /api/admin/session`, `GET /api/admin/analytics`, and resource CRUD at `POST` / `PUT` / `DELETE /api/admin/resources/…` |
| Identity | Clerk, with the frontend API proxied first-party through **`/api/__clerk`** (`VITE_CLERK_PROXY_URL`) |
| Content | resources are served as data from the API, with client-side failure copy "We couldn't load the resources right now. Please try again shortly." |
| Client state | `localStorage` for the language preference (`mindshift-lang`, `en`/`fr` only), `sessionStorage` for the analytics session id |

`robots.txt` disallows `/admin`, `/sign-in`, `/sign-up` and **`/api/`** — a real application surface exists
behind those paths, and the site asks crawlers not to look at it. That is a normal, unremarkable choice; it
is recorded because it corroborates that the endpoints above are functional rather than vestigial.

**The route count is stated from the table, not from memory.** Thirteen paths, counted above, enumerated in
`§4.1`; the same number appears in the claims file's §9.1.

### 4.2 The site ships its own visitor analytics, and there is no notice for it

This is the finding from the client that a claims-only reading cannot produce. Read from the bundle:

| Element | Detail |
|---|---|
| Session identity | `function Nh()` reads a UUID from `sessionStorage` under key `s0`, validates it against `/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i`, and mints one with `crypto.randomUUID()` when absent or invalid |
| Session registration | `fetch("/api/analytics/session", {method:"POST", headers:{"Content-Type":"application/json"}, credentials:"include", keepalive:true, body: JSON.stringify({sessionId})})` |
| Re-registration guard | a second `sessionStorage` key, **`mindshift.analytics.signed-in`**, stores `` `${state}:${sessionId}` `` so the call is skipped for a repeated state |
| Event transport | `fetch("/api/analytics/events", {method:"POST", credentials:"include", keepalive:true})` with `{category, name, path, sessionId}`; `path` is truncated to 240 characters and reduced to its first segment |
| Event taxonomy | **exactly four call sites**: `{category:"page_view", name:"page_view"}`, `{category:"client_error", name:<error>}`, `{category:"feature_use", name:"resource_opened"}`, `{category:"feature_use", name:"features_explored"}` |
| Failure handling | both calls end `.catch(()=>{})` — the site's own telemetry **fails silently** and can never surface an error to the visitor |
| Nothing else | `consent` → **0**; `cookie` → **6**, and **all six are inside the bundled Sentry SDK's own `dataCollection` configuration**; all 16 `Consent` occurrences belong to Clerk's `OAuthConsent` component; `GDPR` → 0; `banner` → 0; `doNotTrack` → 0 |

**Why it is a finding.** The same site says "**privacy-forward by design**" and titles a feature card
"**Privacy respected**". It measures page views, which resource pages were opened, which feature panels were
explored and what client errors occurred, keyed to a per-session identifier, posted to its own origin with
credentials included — and there is **no cookie notice, no consent banner and no analytics disclosure
anywhere in the client**, on a site whose privacy page 404s (§3.3). **A site that collects first-party
analytics and publishes no notice of it, because it has no privacy page to publish it on**, is the
combination recorded here.

**What this does not say.** It is a **static** finding about code: it establishes what the site is *written
to do*, not that any endpoint is live, that any event was stored, or that anything was shared. The session
id is a random per-session value, not a name or an account id. `credentials:"include"` plus the first-party
Clerk proxy is exactly why a notice would be expected in the ordinary case — that is an observation about the
**absence of a notice**, not an allegation about the vendor's handling of data. And it says nothing about
the **app**, which was not inspected.

### 4.3 Third-party processors visible in the client

| Processor | Evidence in the bundle | Count (match mode stated) |
|---|---|---|
| **Clerk** (identity, organizations, billing) | `clerk`, browser and dashboard references | `Clerk` capital → 103; `clerk` case-insensitive → **505**; `clerk.com` → **14**; `dashboard.clerk.com` → **4** |
| **Stripe** (payments, via Clerk billing) | `Stripe` and Stripe.js loader internals | `Stripe` → 3; `stripe` case-insensitive → 3; `PaymentElementContext` → 1; `StripeUtilsContext` → 1; `__internal_loadStripeJs` → 2 |
| **Sentry** (error telemetry) | Sentry SDK with redaction lists (`set-cookie`, `csrf`, `xsrf`, `authorization`, `bearer`, `sso`, `saml`, `credentials`, `sid`, `identity`) and `traceLifecycle: 'static'` guidance | `Sentry` capital → 20; `sentry` case-insensitive → **129** |

**A reference count is meaningless without its match mode.** `Sentry` (capital) is 20; `sentry`
case-insensitive is 129. `Clerk` (capital) is 44 in the earlier note; case-insensitive it is 505. Counts in
this file always carry the mode.

**Replit is not in the client.** `i.replit.com/script.js` appears in the served HTML shell of **every**
document — including the two 404 documents — and **0 times** in the 741,896-byte bundle. Any statement about
Replit here names the *shell*; attributing it to the client would be wrong.

**Two hypotheses were tested and refuted** and are therefore **not** findings: a Segment CDP integration, and
a Google `gtag` tag. Neither is present. A hypothesised integration that does not exist is not evidence of
good practice, and it is not recorded as a finding.

### 4.4 Failure and absence handling in the client

| Surface | Behaviour |
|---|---|
| Resource list fails to load | user-visible copy: "We couldn't load the resources right now. Please try again shortly." |
| Page-level retry | "Please refresh the page and try again." |
| Resource 404 (client-side route) | a localized component: "The page you're looking for moved or never existed. Let's get you back to something helpful." / "La page que vous cherchez a été déplacée ou n'a jamais existé…" |
| Analytics fails | swallowed (`.catch(()=>{})`) — no user-visible effect |

**A string is not a behaviour.** These strings establish that a failure *state is modelled in the UI
layer*; they do not establish that the state is reached correctly, that it is distinguishable from an empty
list, or that it is ever shown. Recorded as a modest positive — the client has failure copy rather than
assuming success — and no more.

---

## 5. Static findings — reconciliation points

These are the places where two artifacts that the vendor put into the world disagree with each other, or
where a published artifact undercuts a marketing claim. Each is stated with both sides quoted.

### 5.1 The declared privacy URL is dead, on both storefronts

Fully documented in §3.3. **Verified absence, five ways.** Recorded as a documentation-lifecycle defect,
not as a data-handling finding.

### 5.2 The registry classifies the study differently from the study and the site

| Surface | Says |
|---|---|
| **ClinicalTrials.gov** `NCT04562948` (third party) | `studyType: **OBSERVATIONAL**`, `observationalModel: ECOLOGIC_OR_COMMUNITY`, `timePerspective: PROSPECTIVE`, **no `armsInterventionsModule`**, `derivedSection.interventionBrowseModule` is `None`, `enrollment 380 (ACTUAL)` |
| **The paper** (the vendor's own authors) | title: "North American open-label 16-week **trial**…"; keywords include "Open-label **trial**"; text: "participated in an online open-label **trial**"; "**four primary outcomes**" |
| **The site** | "A 16-week **open-label clinical trial** of 380 adults showed significant improvement across **all four primary outcomes**." |

**Two distinct conflicts, both recorded unresolved:**

1. **Design class** — the registry classifies the study `OBSERVATIONAL`; the paper's noun is "trial" and the
   site's noun is "clinical trial". An observational registry record and a trial noun are not the same
   object.
2. **Primary outcomes** — the registry lists **one** primary outcome ("Linear and non-linear change in
   generalized anxiety severity over 16 weeks") and **three** secondary (depressive symptom severity,
   functional impairment, life satisfaction). The paper and the site both say **four primary outcomes.**

**This file does not adjudicate which is correct.** Both are quoted; neither is averaged; the discrepancy is
the finding.

**Also recorded, without interpretation:** the site's own outcome count is internally inconsistent — one
string says "anxiety, functioning, and quality of life" (three), while others say "all four primary
outcomes" (four); the French FAQ has the same three-versus-four split.

### 5.3 The vendor's own funded study is the counter-surface to its marketing

This is the sharpest surface conflict in the candidate, and it is a *within-listing* tension: both sides are
the vendor's own material.

| Marketing surface | The vendor's own publication |
|---|---|
| "**Clinically proven**" (footer badge) | "**Improvements were unrelated to the frequency with which participants used the MindShift app.**" |
| "**Proven, not promised.**" (meta) | "…**consistent with response to treatment but not remission**"; effects moderate (d = 0.61 / 0.50 / 0.55 / 0.31) |
| "**Most anxiety apps are guesswork.**" | "**the lack of a comparison group who abstained from using the MindShift app limits the causal inferences one can draw about the contribution of the MindShift app to participants' improved well-being over time.**" |

**The disclosure of who funded the study is in the source and not on the marketing surface.** The paper's
competing-interest statement reads "**Lance M. Rappaport reports financial support was provided by Anxiety
Canada**"; four of five authors "**volunteer on the Scientific Advisory Committee of Anxiety Canada**"; the
acknowledgements read "**This work was supported by a grant from Anxiety Canada to LMR.**" Anxiety Canada is
the app's original co-developer. **The disclosure exists in the peer-reviewed record — which is to the
authors' credit — and is absent from the marketing surface that cites the study.**

**Recorded as selective citation with the source quoted, and left standing. Not a misconduct allegation.**
The paper is unusually candid; the finding is about what the *marketing* keeps.

### 5.4 The trial's verified-install subset is 156 of 380

The paper's methods state that "**the sample was restricted to the 156 participants for whom entrance of
their study identification code into the MindShift smartphone app verified that they downloaded the
smartphone app**". **224 of 380 participants never demonstrably installed the product**, yet the headline
figure on the site, the store listing and the FAQ is **380**.

Recorded as a **claim-precision** finding. The paper reports the restriction plainly; the marketing carries
only the larger number.

### 5.5 One of the four headline outcomes required a published correction

The 2024 corrigendum reports "**an unfortunate error in the calculated total score for quality-of-life from
the Q-LES-Q-SF**"; the baseline quality-of-life mean was corrected from (M=30.76, SD=9.77) to (M=47.83,
SD=15.39), and the reliability item count from 16 to 14. The corrigendum states that "**the primary
conclusions are unchanged**" — and **does not restate the quality-of-life effect size**, so whether the
reported d = 0.31 changed is `[UNKNOWN]`.

### 5.6 Intra-vendor count and rating discrepancies, recorded without averaging

| Quantity | Surface A | Surface B | Handling |
|---|---|---|---|
| Downloads | site "**More than 2 million downloads across 180+ countries**" and badge "**2M+ downloads**" | Google Play "**500K+ Downloads**" | both quoted; **not averaged** |
| Rating | site "**4.8 rating**" rendered beside five filled star glyphs (no storefront named, no date) | App Store `averageUserRating` **4.32402 / 537 ratings**; Google Play **3.3★**, header "2.2K reviews", ratings block "2.08K" | all surfaces quoted with retrieval dates; **unresolved**; the site figure carries no qualifier |
| Age rating | App Store page displays **9+** | iTunes Lookup `contentAdvisoryRating` **4+** | both are Apple-side; recorded unresolved and **not attributed to the vendor** |
| App size | App Store page "88 MB" | iTunes Lookup `fileSizeBytes` **87,966,720** | consistent; recorded for completeness |

**An unresolved rating ambiguity is not recorded as a discrepancy.** Where the two storefront panels show
different review *counts* for the same app (2.2K in the header, 2.08K in the ratings block), both are
recorded and the ambiguity is marked **unresolved**.

### 5.7 A developer-declared accessibility field that contradicts the vendor's own claim

| Surface | Text |
|---|---|
| Site, UI/UX Designer role description | "…with a focus on accessibility — designing to **AODA and WCAG** standards so the app stays clear, calming, and usable for everyone." |
| App Store Accessibility section (developer-supplied) | "**The developer has not yet indicated which accessibility features this app supports.**" |

Recorded as a **per-language disagreement between vendor surfaces about the same claim.** A design standard
is not the same object as a declared accessibility feature, so this is registered as an
**inconsistency, not a refutation** — and the release notes for v4.1.0 do claim "Improved accessibility —
better screen reader support and clearer contrast throughout the app".

### 5.8 Per-language disagreement about supported languages

The site's FAQ says the app is "currently available in **English and French**"; the App Store declares
`languageCodesISO2A` **`['EN','FR','ES']`**. Unresolved, both quoted. A **v2.0.9** release note in the App
Store history reads: "Thank you for all your patience and support! We have improved our app's performance and
made it bilingual (French and English). Switch the language of your device to French if you want to use
MindShift in French." The bundle's language switch accepts only `en` and `fr`.

### 5.9 A moderated user-generated-content community with no published terms on this surface

The site describes "A safe, actively moderated discussion board", "Actively moderated for safety" and
"Anonymous and judgment-free". Google Play flags "**Users Interact**"; the App Store flags "**Contains
User-Generated Content**". A fixed-string sweep of the published client returns `guidelines` → **0**, and no
terms document exists on the site (§3.3). **The app's own release notes reference a guidelines surface** —
v4.1.0 says "the guidelines no longer interrupt you or send you back to the home screen" — so a moderation
document exists **inside the app** and is not published on the web surface. Recorded as a
**publication-placement** finding.

### 5.10 "Always Free" beside an unpriced B2B surface

Google Play: "**Always Free: No subscriptions, no hidden costs.**" The same published client ships **Clerk
organizations and Stripe billing**, and the site runs `/institutions` ("Talk to us about early access for
your organization") and `/for-clinics` ("bulk materials and organizational deployments"). **The consumer app
is free**; the operator is a for-profit firm with a commercial surface whose **prices are not published**
(`[UNKNOWN]`). Both recorded; no inference about intent.

---

## 6. Runtime observation

**There are none. Zero.**

| Runtime activity | Performed? |
|---|---|
| App installed on a device or emulator | no |
| App launched, any screen navigated | no |
| Account created or signed in | no |
| Proxy configured, traffic captured | no |
| Vendor endpoint called | no |
| Data export performed or CSV examined | no |

**Every behaviour attributed to MindShift in this package is either a vendor claim, a third-party record, or
a static reading of published code.** No statement in this file or its companions may be read as an
observation of the product running. Where the claims file says a behaviour is *claimed*, it is claimed; where
it says *documented*, it is documented; where it says `[FOUND IN PUBLISHED CLIENT]`, it was read as text.

---

## 7. Claim reconciliation

**Method.** Each checkable claim from `mindshift_CLAIMED_FEATURES_AND_PURPOSE.md` is matched against the
surface that could confirm or contradict it. Verdict vocabulary:

- **MATCH** — the claim is corroborated by an independent or third-party artifact.
- **DOCUMENTED ONLY** — the claim stands on the vendor's word (or a vendor-published document) with nothing
  external to check it; recorded as neither confirmed nor refuted.
- **DOES NOT MATCH** — a retrieved artifact contradicts the claim as stated.
- **PARTIAL** — the claim is accurate only under a qualification the surface does not carry.
- **OUTSIDE** — the claim is not a factual assertion that any retrieved artifact could settle.

| # | Claim | Where claimed | Where checked | Verdict |
|---|---|---|---|---|
| 1 | Privacy policy at `https://mindshiftcbt.com/privacy/` | Google Play listing | direct probe + sitemap + router + bundle sweep + re-fetch | **DOES NOT MATCH** |
| 2 | Same URL as the App Store "Privacy Policy" link | App Store page | same chain | **DOES NOT MATCH** |
| 3 | "The Terms of Use and Privacy Policy have also been updated…" (v3.0.1, v3.0.3) | App Store release notes | bundle sweep → 0 `Terms` occurrences | **PARTIAL** (documents once existed; declared URL now dead) |
| 4 | "**Privacy respected**" | site feature card | bundle string present; no policy to support it | **DOCUMENTED ONLY** |
| 5 | "**privacy-forward by design**" | `/for-clinics` | as above | **DOCUMENTED ONLY** |
| 6 | "**not a medical device, and not a replacement for therapy or crisis care**" | site FAQ | no device registry entry claimed; internally consistent | **MATCH** (a self-limiting claim, verifiable as a claim) |
| 7 | "**not itself a crisis service**" | site FAQ | crisis lines present in client (see #28) | **MATCH** (consistent with the client's own routing) |
| 8 | "**Peer-reviewed** 16-week open-label trial (N=380)" | site FAQ | PMID 40656972 exists; journal record verified | **MATCH** (as to existence and peer review) |
| 9 | "**open-label clinical trial**" | site | registry `studyType: OBSERVATIONAL` | **DOES NOT MATCH** |
| 10 | "16-week" | site, FAQ, registry | registry start 2020-11-11 → completion 2022-04-12; paper design | **MATCH** |
| 11 | "N=380 adults" | site, FAQ, paper | registry `enrollment 380 (ACTUAL)` | **MATCH** |
| 12 | "N=380" as the analysed sample | site, marketing | paper: verified-install subset **156 of 380** | **PARTIAL** |
| 13 | "**all four primary outcomes**" | site, FAQ, paper | registry: **one** primary + three secondary | **DOES NOT MATCH** |
| 14 | "anxiety, functioning, and quality of life" (three outcomes) | one site string | the site's other strings say four | **DOES NOT MATCH** (internally) |
| 15 | "**Clinically proven**" | site footer badge | paper: no comparison group; effects moderate; unrelated to app use | **DOES NOT MATCH** |
| 16 | "**Proven, not promised.**" | site meta | as #15 | **DOES NOT MATCH** |
| 17 | "**Most anxiety apps are guesswork.**" | site | no retrieved artifact could settle a comparative claim about unnamed products | **OUTSIDE** |
| 18 | "making MindShift one of the few anxiety apps with **published, peer-reviewed evidence**" | site | PMID 40656972 and its preprint lineage exist | **MATCH** (narrowed form; the broader "most/guesswork" claim is #17) |
| 19 | "**More than 2 million downloads across 180+ countries and all 13 Canadian provinces**" | site | Play says "500K+ Downloads" | **PARTIAL** |
| 20 | "**2M+ downloads**" badge | site | as #19 | **PARTIAL** |
| 21 | "**4.8 rating**" | site | App Store 4.32402; Play 3.3 | **DOES NOT MATCH** |
| 22 | "**500K+ Downloads**" | Play | site says 2M+ | **PARTIAL** |
| 23 | "**Always Free: No subscriptions, no hidden costs.**" | Play description | App Store `formattedPrice: Free`; the B2B surface (§5.10) is a separate, unpriced offering | **MATCH** |
| 24 | "MindShift is designed, built, and cared for by the team at **247 Labs**…" | site About | App Store `sellerName: 247 Labs Inc`; Play developer `247 Labs Inc` | **MATCH** |
| 25 | "**Originally developed in partnership with Anxiety Canada**" (past tense) | site About/footer | registry `collaborator: Anxiety Canada`; paper methods name Anxiety Canada as app owner | **MATCH** |
| 26 | "**recommended by clinicians at academic medical centres in Canada and the US, and used in research in outpatient and inpatient psychiatric settings**" | site FAQ | no citation given on the surface | **DOCUMENTED ONLY** |
| 27 | "**There's no clinician portal — each person owns and controls their export.**" | `/for-clinics` | nothing contradicts; no contract exists | **DOCUMENTED ONLY** |
| 28 | Crisis lines 988 / Kids Help Phone / Talk Suicide Canada | published client | three named lines with tel: links and Canadian qualifiers | **MATCH** (static; the strings exist as claimed) |
| 29 | "**A structured CSV of check-ins and the data collected across guided tools**" | `/for-clinics` | no schema published; no export performed | **DOCUMENTED ONLY** |
| 30 | "you can **request deletion from the app settings, which permanently removes your data from our servers**" | site FAQ | Play: "You can request that data be deleted" | **DOCUMENTED ONLY** |
| 31 | "**Your Thought Records, check-ins, and personal data are stored securely**" | site FAQ | "stored securely" is undefined; no encryption-at-rest statement | **DOCUMENTED ONLY** |
| 32 | "**we don't sell your personal data to third parties**" | site FAQ | Play data-safety: "No data shared with third parties" | **PARTIAL** (storefront self-declaration is not an audit) |
| 33 | "**Data is encrypted in transit**" | Play data safety | nothing to check without runtime | **DOCUMENTED ONLY** |
| 34 | "**Adolescents (13+) and adults with mild-to-moderate anxiety**" | site FAQ | App Store age rating 9+; registry `minAge: 18 Year` for the study | **PARTIAL** (different objects: product audience vs study eligibility) |
| 35 | "**currently available in English and French**" | site FAQ | App Store `languageCodesISO2A` `['EN','FR','ES']` | **DOES NOT MATCH** |
| 36 | "designing to **AODA and WCAG** standards" | site team surface | App Store: developer has not indicated accessibility features | **DOES NOT MATCH** |
| 37 | "**Improved accessibility — better screen reader support and clearer contrast**" (v4.1.0) | App Store release notes | — | **DOCUMENTED ONLY** |
| 38 | "**A safe, actively moderated discussion board**" / "**Actively moderated for safety**" | site | Play "Users Interact"; App Store "Contains User-Generated Content"; no published guidelines | **PARTIAL** |
| 39 | "**Anonymous and judgment-free**" | site | an optional account (Name, email, User ID) is declared in App Privacy | **PARTIAL** |
| 40 | "**This study was registered on ClinicalTrials.gov (NCT04562948).**" | paper methods | registry record exists; last update 2022-11-02 | **MATCH** |
| 41 | "**the lack of a comparison group … limits the causal inferences**" | paper limitations | registry has no interventions module and no control arm | **MATCH** (the source and the registry agree) |
| 42 | "**Improvements were unrelated to the frequency with which participants used the MindShift app.**" | paper abstract/results | no independent replication retrieved | **DOCUMENTED ONLY** (the vendor's own funded paper) |
| 43 | "**unfortunate error in the calculated total score for quality-of-life**" | corrigendum | corrigendum exists, PMID 40655914, PMC12244218 | **MATCH** |
| 44 | QoL effect size **d = 0.31** | 2023 paper | corrigendum does not restate it | **OUTSIDE / `[UNKNOWN]`** |
| 45 | "**Lance M. Rappaport reports financial support was provided by Anxiety Canada**" | paper competing-interest | registry IPD-sharing rationale also names Anxiety Canada as funder | **MATCH** |
| 46 | Site presents the trial as "**published, peer-reviewed evidence**" with no funding disclosure | site | paper's own competing-interest statement | **DOES NOT MATCH** (a disclosure present in the source is absent from the surface citing it) |
| 47 | Store panels' declared data categories | App Store App Privacy / Play data safety | Apple's own label: "This information has not been verified by Apple." | **DOCUMENTED ONLY** |
| 48 | Site ships `POST /api/analytics/session` and `/api/analytics/events` | published client | read as text | **MATCH** (static: the code is as claimed; **no endpoint was called**) |
| 49 | No cookie/consent/analytics notice in the client | published client | `consent` → 0; `cookie` hits all inside Sentry SDK; `Consent` hits all Clerk `OAuthConsent` | **MATCH** (fixed-string sweep) |
| 50 | No privacy or terms path in sitemap or router | site | 9 `<loc>` entries; 13 router paths | **MATCH** |
| 51 | `robots.txt` disallows `/admin`, `/sign-in`, `/sign-up`, `/api/` | site | file read directly | **MATCH** |
| 52 | Clerk / Stripe / Sentry are loaded by the client | published client | string counts recorded with match mode | **MATCH** (static) |
| 53 | Replit is loaded by the site | shell vs bundle | 0 occurrences in the bundle; present in every served HTML shell | **PARTIAL** (attributable to the shell, not the client) |
| 54 | Canadian crisis resources are reachable from the app | published client | `tel:` links for all three lines | **MATCH** (static) |
| 55 | "**Everything you might want to know about MindShift CBT, how it works, and how it protects your privacy.**" | FAQ meta description | the FAQ does not link to a privacy document | **DOES NOT MATCH** |
| 56 | Export is patient-initiated | `/for-clinics` | no runtime performed; no contract | **DOCUMENTED ONLY** |

**Verdict tally** — machine-counted from the table above, not written from impression:

| Verdict | Count |
|---|---|
| MATCH | 20 |
| DOCUMENTED ONLY | 12 |
| DOES NOT MATCH | 12 |
| PARTIAL | 10 |
| OUTSIDE | 2 |
| **Total** | **56** |

*Machine-counted from the 56 rows of the table above by a script that reads the verdict column, not written from
impression. `MATCH` is the largest class and `DOES NOT MATCH` is tied with `DOCUMENTED ONLY`; no verdict was
assigned to a row before the surface that could settle it had been retrieved.*

### 7.1 Positive comparators found while reconciling

Three findings are recorded **as positives**, and are not softened by the conflicts above:

1. **`PublicSleepType`-equivalent provenance does not exist here, but narrower discipline does.** The vendor
   states its own limits in its own words: "not a medical device", "not itself a crisis service",
   "mild-to-moderate", "Adolescents (13+)". This is the three-state honesty rule applied to **scope**, in
   plain language, unprompted.
2. **Patient-controlled export with the clinician portal explicitly refused.** "There's **no clinician
   portal** — each person owns and controls their export." Whatever else is unresolved, the vendor names its
   boundary rather than implying a capability it does not have.
3. **The publication diagnoses the app's own weakness.** A vendor whose published paper says "improvements
   were unrelated to the frequency with which participants used the MindShift app" has done the harder thing
   that the marketing then undoes. The paper is a genuine contribution; the marketing is the defect.

---

## 8. Evidence gaps

| # | Gap | Why it remains open | Consequence |
|---|---|---|---|
| G-1 | The app binary was never inspected | not lawfully obtainable for this candidate | whether the privacy policy / terms exist **inside** the app is `[UNKNOWN]`; release notes say they were updated |
| G-2 | No runtime observation of any kind | no app run, no proxy, no account | every behaviour claim about the app is a vendor claim or a client string |
| G-3 | `anxietycanada.com` returned a byte-identical SPA shell for every path | JS-rendered site; retrieval failure | Anxiety Canada's **current** statements about MindShift, and its own privacy policy, are `[UNKNOWN]` |
| G-4 | The FAQ's stated language set conflicts with the App Store | no binary to read the shipped UI | which surface reflects the real UI is `[UNKNOWN]` |
| G-5 | Apple's App Privacy panel is internally ambiguous | Contact Info appears under both "Data Linked to You" (Name) and "Data Not Linked to You" (Email Address); the collapsed summary lists it under "Linked" | whether the store panels' "Health & Fitness" data is linked to the account is `[UNKNOWN]` |
| G-6 | B2B pricing not published | `/institutions` says "Talk to us about early access" | the commercial model is `[UNKNOWN]`; the consumer app is free |
| G-7 | Community moderation policy not published on the web | release notes imply a guidelines surface exists inside the app | whether a moderation policy / terms exists inside the app is `[UNKNOWN]` |
| G-8 | `CANDIDATE_SHORTLIST.md` carried the superseded operator wording | a reader of the shortlist alone would have got the wrong operator | **closed 2026-10-06** — entry 10 corrected in place; see §14.4 of the claims file |
| G-9 | The site's own analytics endpoints were never called | explicit authorization decision | whether they are live, and what they store, is `[UNKNOWN]` |
| G-10 | Sentry/SDK configuration read as strings only | no runtime | whether telemetry actually leaves the page is `[UNKNOWN]`; the code is written to |
| G-11 | The QoL effect size after the corrigendum was never restated | corrigendum does not include it | whether d = 0.31 changed is `[UNKNOWN]` |
| G-12 | Only targeted literature searching was performed | one Europe PMC sweep | "the only study" is **not** a claim this package makes; the paper itself cites "[35], [36]" as prior work on the app |

---

## 9. Ethical and legal limits applied

| Limit | Application here |
|---|---|
| **No DRM, licence or protection circumvention** | The app binary was not acquired or decompiled. Only artifacts served to a browser were read. |
| **No certificate pinning defeated** | No proxy was configured at all; no TLS interception of any kind. |
| **No credentials or private data** | No account, token or credential was used. The only identifiers discussed are the vendor's own published ones (track IDs, bundle IDs, registry numbers). |
| **No vendor endpoint called** | Endpoints appear in this file **only** as strings read from published JavaScript. |
| **No personal data reproduced** | Named individuals in this file are the vendor's own published clinical contributors and authors, cited as the vendor publishes them. Third-party developer machine paths found in public source are **not** reproduced (a standing rule recorded during the OSCAR pass). |
| **No PHI/PII** | None encountered; none recorded. |
| **One-shot, dated retrieval** | All artifacts are dated 2026-10-06 and hashed. The vendor's site may change; the hashes are what was read. |
| **A hidden identifier is not reproduced** | An error-telemetry ingest identifier found in the bundle is **not** reproduced in any committed file. |

---

## 10. Reproduction

```sh
# 0. Working directory for this candidate
mkdir -p /tmp/aw2research/mindshift/{web,api} && cd /tmp/aw2research/mindshift
UA='Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36'

# 1. The site: shell, then the published client
curl -s -A "$UA" https://www.mindshiftcbt.com/ -o web/home.html
# extract the bundle URL from the shell, then fetch it as web/index-BogFfh2L.js
grep -oE '/assets/index-[A-Za-z0-9_-]+\.js' web/home.html | head -1

# 2. Site inventory — the sitemap and the crawler policy
curl -s -A "$UA" https://www.mindshiftcbt.com/sitemap.xml -o web/sitemap.xml
curl -s -A "$UA" https://www.mindshiftcbt.com/robots.txt  -o web/robots.txt

# 3. Content pages quoted in the claims file
for p in about app contact faq for-clinics institutions methodology resources; do
  curl -s -A "$UA" "https://www.mindshiftcbt.com/$p" -o "web/ms_$p.html"
done

# 4. THE 404 CHAIN — status codes and the genuine Not-Found document
: > web/probe_log.txt
for u in /privacy /privacy/ /terms /terms/ /privacy-policy /privacy-policy/ /legal /legal/ \
         / /faq /resources; do
  code=$(curl -s -o /tmp/body -w '%{http_code}' -A "$UA" "https://www.mindshiftcbt.com$u")
  size=$(wc -c </tmp/body | tr -d ' ')
  title=$(grep -o '<title>[^<]*</title>' /tmp/body | head -1 | sed 's/<[^>]*>//g')
  printf '%-22s %s  %8s B  %s\n' "$u" "$code" "$size" "$title" | tee -a web/probe_log.txt
done
curl -s -A "$UA" https://www.mindshiftcbt.com/privacy -o web/ms_privacy_20261006.html

# 5. The string sweep — run BEFORE drawing any conclusion about absence
B=web/index-BogFfh2L.js
for s in "Privacy Policy" "privacy policy" "Terms of Use" "Terms of Service" "terms of use" "Terms" \
         "guidelines" "consent" "cookie" "Clerk" "clerk.com" "dashboard.clerk.com" "Stripe" "Sentry"; do
  printf '%-20s %s\n' "$s" "$(grep -o -F "$s" "$B" | wc -l | tr -d ' ')"
done
# match mode matters: capital-S Sentry is 20, case-insensitive is 129
printf 'sentry (i)  %s\n' "$(grep -o -i -F sentry "$B" | wc -l | tr -d ' ')"
printf 'clerk (i)   %s\n' "$(grep -o -i -F clerk  "$B" | wc -l | tr -d ' ')"

# 6. The routes and the telemetry — read, never called
grep -o -E 'path:"[^"]*"' "$B" | sort -u
grep -o -E '.{80}/api/[A-Za-z0-9_/:{}.-]*' "$B" | sort -u
grep -o -E 'Zl\(\{[^}]{0,200}' "$B" | head -20

# 7. Storefront metadata — match on bundleId, not on position
curl -s 'https://itunes.apple.com/lookup?bundleId=com.bstro.mindshift' -o api/itunes_com.bstro.mindshift.json
curl -s 'https://itunes.apple.com/lookup?bundleId=com.bstro.mindshift&country=ca' -o api/itunes_ca.json

# 8. The independent registry, and the literature
curl -s 'https://clinicaltrials.gov/api/v2/studies/NCT04562948' -o api/ct_NCT04562948.json
curl -s 'https://www.ebi.ac.uk/europepmc/webservices/rest/search?query=mindshift&format=json' \
  -o api/epmc_mindshift.json
# NOTE: the numeric-PMID form of /fullTextXML returns a 148-byte HTTP 500 body.
#       Re-issue against the PMC id:
curl -s 'https://www.ebi.ac.uk/europepmc/webservices/rest/PMC12244177/fullTextXML' \
  -o api/fulltext_PMC12244177.xml

# 9. Quote verification — a required, separate pre-commit step
#    (run from the repository root; the checker is described in ../RESEARCH_METHOD.md §1,
#     and the scratch copy under /tmp did not survive the reboot)
AW2_ROOT=/tmp/aw2research/mindshift python3 qc2.py \
  docs/app_comparison/mindshift/mindshift_CLAIMED_FEATURES_AND_PURPOSE.md

# 10. Hashes for the artifact record
find web api -type f -print0 | sort -z | xargs -0 shasum -a 256
```

**Reproduction caveats.** (a) The bundle filename is content-hashed and will change on the vendor's next
deploy; the record here is the 2026-10-06 revision, `index-BogFfh2L.js`, SHA-256 `1bb16811…`. (b) The
storefront HTML and the App Store privacy panel are rendered and may drift; use the hashes in §3.2 as the
authority for what was read. (c) `anxietycanada.com` returned a JS shell for every path on this date, so
that gap cannot be closed by re-running this script. (d) **Steps 6 and 9 read and verify; no step in this
script calls a vendor endpoint.** (e) The quote checker normalises to lowercase alphanumerics, strips
HTML/XML tags, decodes JavaScript `\uXXXX` escapes and splits elided quotes on `…`/`...`; a naive string
match produces both false negatives (tag names pollute) and false positives (a paraphrase can pass).

---

## Related files

- `mindshift_CLAIMED_FEATURES_AND_PURPOSE.md` — the public-claims dossier and the full claim ledger
  (`M-1`–`M-58`), artifact record and evidence gaps (`G-1`–`G-12`).
- `PROMPT_2.md` — the candidate-specific second-stage brief.
- `mindshift_COMPARISON.md` — the v1/v2 comparison.
- `../CANDIDATE_SHORTLIST.md` — shortlist entry #10 — **corrected in place 2026-10-06** (operator and
  commercial model); see `mindshift_CLAIMED_FEATURES_AND_PURPOSE.md` §14.4.
- `../PROCESS_STATE.md` — candidate dossier and residual uncertainty.
- `../RESEARCH_METHOD.md` — inspection modes and the standing sub-checks added by this pass.
