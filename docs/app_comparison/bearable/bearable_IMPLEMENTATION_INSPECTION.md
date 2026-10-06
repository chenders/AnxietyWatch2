# Bearable — authorized implementation inspection

**Slug:** `bearable`
**Inspection date:** 2026-10-06
**Target:** Bearable, `com.bearable` (Android) / `id1482581097` + `co.codeleap.bearable` (iOS)
**Status: NOT PERFORMED** — no lawfully authorized artifact was available to the researcher.

This report exists, as required by `../PROMPT1.md` Phase 2A, even though inspection could not be
performed. It records the blocker, the evidence gap it creates, and a validated toolchain that is
ready to use the moment authorization is granted.

---

## 1. Why inspection was not performed

Phase 2A permits inspection only of an artifact the researcher is authorized to inspect — an APK,
AAB-derived APK, IPA, source build, test build, or device installation. For Bearable:

| Potential route to an artifact | Lawful? | Outcome |
|---|---|---|
| **Vendor source release** | n/a | Bearable is **not** open source. No repository, no published source archive. |
| **Official vendor download** | n/a | Bearable is distributed only through the Apple App Store and Google Play. Bearable Ltd publishes no direct APK/IPA download. |
| **Third-party APK mirror** (APKMirror, APKPure, etc.) | **No** | These redistribute the app outside its distribution licence. Redistribution is not authorization to inspect, and the mirror cannot grant rights the vendor has not. **Not used.** |
| **Extraction from a device the researcher controls** | Possibly lawful | Would require a device or emulator signed into a Google/Apple account **belonging to the owner**, and — to exercise anything past the sign-up screen — creating a Bearable account. Both require owner credentials and an explicit authorization decision. **Not available to the researcher in this session.** |
| **Bearable Pro / research ePRO build** | n/a | Available only to research partners under an agreement. The researcher is not a partner. |

Two further Phase 2A constraints apply independently of artifact access:

1. Bearable **requires an account** before the app is usable at all (documented in
   `bearable_CLAIMED_FEATURES_AND_PURPOSE.md` §12). The brief forbids creating accounts or logging
   into production services, so even with an artifact the observable surface would stop at the
   sign-up screen.
2. Bearable's core function is cloud-backed. Meaningful runtime observation would require the app to
   talk to Bearable's production servers with a real account — also excluded by the brief.

**Conclusion.** This is an authorization/artifact-access blocker, not a tooling or skill blocker.
It is the single most consequential process finding of the pilot: **every commercial candidate on
the shortlist will hit this same wall.** Only `oscar`, `nightscout` and `homeassistant` (open source)
and `mindshift` (free, but still store-only) are inspectable on this basis, and only the open-source
three can be inspected lawfully with no owner decision at all.

## 2. Artifact record

| Field | Value |
|---|---|
| App name | Bearable (Symptom, Mood & Period Tracker) |
| Package identifier | `com.bearable` (Android); `co.codeleap.bearable` / App Store id `1482581097` (iOS) |
| Version / build | **Not acquired** — no version was pinned, because no artifact was obtained |
| Source | None. Store listings referenced for metadata only |
| Acquisition date | n/a |
| SHA-256 | **Not available** |
| Platform inspected | None |
| Device / emulator | None used for the target |
| Tools used against target | None |

## 3. Static findings

**None.** No APK, IPA, DEX, Mach-O, manifest, plist, resource table or bundled asset was obtained for
Bearable, so there are no static findings of any kind. Any statement about Bearable's internals
would be speculation and is deliberately absent from this report.

Note that the public claims in `bearable_CLAIMED_FEATURES_AND_PURPOSE.md` about missing capabilities
(no CPAP, no PDF export, no BLE) are **absence-of-evidence findings from public surfaces**, not
static-analysis findings. They must not be upgraded to "confirmed absent in the binary".

## 4. Authorized runtime observations

**None for the target app.** Nothing about Bearable was observed at runtime: no traffic, no
permissions behaviour, no local storage, no background work.

### 4.1 Toolchain validation (not a Bearable observation)

Because the pilot's purpose includes detecting tool failures before ten candidates are processed, the
free inspection stack in `../FREE-ANDROID-TRAFFIC-INSPECTION.md` was exercised end-to-end against a
**disposable emulator running only Android system components and no third-party app**. This validates
the mechanism; it says nothing about Bearable.

| Item | Observed value |
|---|---|
| Host proxy | `mitmdump 12.2.3` (Python 3.14.4, OpenSSL 3.5.5), listening on `127.0.0.1:8080`, verified with `lsof -nP -iTCP:8080 -sTCP:LISTEN` |
| Emulator | AVD `pixel_4_w_gapps`, Android 12 / SDK 31, `-no-window -no-audio -no-snapshot`, booted in ~14 s |
| adb | Android Debug Bridge 1.0.41 |
| Emulator binary | Android emulator 37.2.12.0 |
| Packet tooling available | TShark (Wireshark) 4.6.5 |
| Proxy set before app launch | `adb shell settings put global http_proxy 10.0.2.2:8080` → read back as `10.0.2.2:8080` |
| Flow capture | **Successful.** Flows recorded to `flows.mitm` and replayed with `mitmdump -nr`; e.g. `GET http://www.google.com/gen_204` → `204`, `GET http://connectivitycheck.gstatic.com/generate_204` → `204` |
| HTTPS behaviour | **As designed.** For `www.gstatic.com:443` the log records `Client TLS handshake failed. The client does not trust the proxy's certificate` — confirming that no proxy CA was installed and that HTTPS is opaque by default, exactly as the procedure intends |
| Cleanup | `adb shell settings delete global http_proxy` → read back `null`; emulator killed; `lsof` confirms port 8080 free afterwards |

**Findings from this validation**

- V1: The mitmproxy + emulator-proxy + `10.0.2.2:8080` path works on this workstation with no paid
  licence, on AVDs that include Google Play images.
- V2: HTTPS is opaque until a CA is installed, and the design intentionally leaves it that way. Any
  candidate that pins certificates will therefore be **uninspectable at the payload level** and must
  be reported as *not observable under the authorized test configuration*.
- V3: The proxy must be set before the app launches to capture first-run traffic; a missed ordering
  cannot be corrected retrospectively.
- V4: Emulator boot and proxy setup are cheap (~1 minute). Artifact acquisition is the expensive and
  blocked step, not the runtime capture.

## 5. Inferences (explicitly labelled as such)

- I1: Because Bearable requires an account and is cloud-backed, its first-run traffic almost certainly
  includes authentication and sync endpoints. **This is an inference from the documented account
  requirement, not an observation.**
- I2: The Play Store data-safety declaration ("may share ... Location, Personal info and 2 others")
  implies the release artifact contains at least one third-party SDK with data-sharing behaviour.
  Which SDK, and whether it is active, cannot be determined without static inspection.
- I3: The Free/Premium gate on correlation reports implies a server-side or remotely-configurable
  entitlement check, since entitlements must survive device changes. Unverified.

## 6. Claim-reconciliation table

Because no static or runtime evidence exists for the target, every row's evidence columns are empty
and the conclusion is necessarily "unresolved by inspection". The public-claim column reproduces the
vendor's position so a future authorized pass can fill in the rest.

| Feature | Public claim | Static evidence | Authorized runtime observation | Reachability / maturity | Conclusion |
|---|---|---|---|---|---|
| Mood/symptom/factor logging | Claimed and documented | *not obtained* | *not observed* | Publicly demonstrated at scale | **Unresolved by inspection.** Public evidence is strong; not independently verified. |
| Medication logging + reminders | Claimed and documented | *not obtained* | *not observed* | Publicly demonstrated | Unresolved by inspection |
| Correlation / Factor Effect reports | Claimed, documented, Premium-gated | *not obtained* | *not observed* | Publicly demonstrated | Unresolved by inspection |
| CSV export of all data, free tier | Documented with in-app path | *not obtained* | *not observed* | Vendor-documented | Unresolved by inspection |
| PDF export | **Not claimed**; listed as an outstanding community request | *not obtained* | *not observed* | n/a | Unresolved by inspection; public evidence suggests absent |
| Apple Health / Health Connect import | Claimed and documented | *not obtained* | *not observed* | Vendor-documented | Unresolved by inspection |
| Write-back to health platforms | Explicitly **not** supported | *not obtained* | *not observed* | Vendor-documented limitation | Unresolved by inspection |
| Weather sync | Explicitly **not** supported; manual factors only | *not obtained* | *not observed* | Vendor-documented limitation | Unresolved by inspection |
| CPAP / oximetry / BLE sensors | Not claimed anywhere examined | *not obtained* | *not observed* | n/a | Absent from public evidence; **not** confirmed absent in the binary |
| AI / LLM analysis | Not claimed | *not obtained* | *not observed* | n/a | Absent from public evidence |
| Account requirement | Confirmed in support docs and privacy policy | *not obtained* | *not observed* | Vendor-documented | Unresolved by inspection |
| Server-side encryption of health data | Claimed in privacy policy | *not obtained* | *not observed* | Legal-document claim | Unresolved by inspection; cannot be verified externally |
| No sale of personal data | Claimed repeatedly, including for HealthKit/Health Connect data | *not obtained* | *not observed* | Legal-document claim | Unresolved by inspection |
| Third-party data sharing (Android) | Play Store data-safety declaration names sharing | *not obtained* | *not observed* | Platform-declared | Unresolved by inspection |
| Telemetry / analytics | Conceded in privacy policy (device, event, IP) | *not obtained* | *not observed* | Legal-document claim | Unresolved by inspection |
| "Not medical advice" disclaimers | Present in store listing and medication article | *not obtained* | *not observed* | Public | Resolved by public evidence |
| Peer-reviewed citation | Linked to an oncology PRO systematic review | *not obtained* | *not observed* | Peer-reviewed, narrow scope | Resolved by public evidence |

## 7. Non-feature findings

No non-feature findings about the artifact could be produced (no artifact). The following are
observations about the **public surfaces**, kept deliberately separate from any implementation claim:

- The public website loads Google Tag Manager and Google Ads/DoubleClick endpoints. That is a
  website analytics/ads fact, **not** evidence about the app's telemetry, and the two must not be
  conflated.
- The Play Store data-safety declaration is inconsistent with a naive reading of "we don't share data
  with third parties": the vendor's privacy policy qualifies sharing as opt-in via integrations,
  while the store declaration names Location and Personal info as potentially shared. Reconciling
  these two statements is an unresolved question (U7 in the claims file).
- The vendor's privacy policy is unusually explicit and specific — naming the ICO registration
  (`ZB189459`), the company number, the DPO contact, and deliberately not collecting name/age/sex/
  gender. This is a positive signal about documentation quality, not a security audit.

## 8. What would unblock this inspection

In ascending order of owner involvement:

1. **Open-source candidates first.** Inspect `oscar`, `nightscout` and `homeassistant` from their
   published source with no owner decision required. This validates the *static* half of Phase 2A.
2. **Owner decides the store-install policy.** If the owner authorizes installing store apps on a
   disposable emulator under their own account, a Bearable pass becomes possible up to the sign-up
   gate, capturing first-run permissions, bundled SDKs, manifest surface and pre-account network
   destinations — without creating a Bearable account.
3. **Owner authorizes a throwaway account.** Only then can post-login sync behaviour be observed.
   This is the step the brief currently forbids, and it should not be taken without an explicit,
   recorded decision.
4. **Accept public-claims-only for commercial candidates.** Also legitimate — but then Phase 2A must
   be marked *not performed* for every commercial candidate, and the comparison reports must say so.

## 9. Evidence gaps created by not inspecting

- Cannot confirm which health metrics are actually requested from HealthKit/Health Connect versus
  merely listed, nor the read/write permission set.
- Cannot confirm the CSV export schema, timestamp format, timezone handling or unit conventions.
- Cannot confirm whether the correlation engine runs on-device or server-side, which determines
  whether the paywall is a UI gate or a genuine server entitlement.
- Cannot confirm the presence, identity or activity of third-party SDKs, despite the Play Store
  declaration implying at least one.
- Cannot confirm what happens to a day with a partially-covered imported metric (U3).
- Cannot confirm that the advertised feature set matches the reachable feature set — the whole point
  of Phase 2A.

## 10. Ethical and legal limits applied

- No third-party APK/IPA mirror was used, and no artifact was obtained outside its distribution
  licence.
- No protection was bypassed or attempted: no certificate pinning defeat, no anti-debugging defeat,
  no DRM/licence circumvention, no platform-protection circumvention.
- No proxy CA was installed, on any device.
- No account was created on Bearable or on any health platform.
- The emulator session used only Android system components; `emulator @pixel_4_w_gapps` was used as
  a disposable, non-personal device.
- The proxy setting was removed and verified removed; the emulator was terminated and the listening
  port confirmed free.
- No credentials, tokens, personal data or secret material are present in this report.
