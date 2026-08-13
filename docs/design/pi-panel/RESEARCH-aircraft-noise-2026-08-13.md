# Aircraft Noise Prediction from ADS-B Without a Microphone: Data Sources, Model Architecture, and Honest Limits

## TL;DR
- **The single best backbone dataset is the Aircraft Noise and Performance (ANP) database's Noise-Power-Distance (NPD) curves, which directly give SEL and LAmax versus slant distance per aircraft and power setting — but ANP covers only civil airliner/large-GA fixed-wing types (EASA states "over 160 civil aircraft types"; broader NPD literature puts it near 300 fixed-wing plus ~26 helicopter types), excludes generic light pistons, is access-restricted from EASA, and must be joined to ICAO type designators via a substitution table. For your dominant helicopter and light-aircraft complaints you must fall back on FAA/Volpe measured reports and certification levels, which give relative ranking reliably but absolute LAmax only to roughly ±3–5 dB.**
- **Your two known failures are both real: certification/absolute source levels are the larger error source (your invented numbers were off by 10+ dB in places), and your thresholds are miscalibrated because outdoor ambient is ~40 dBA, so a defensible AUDIBLE floor is roughly ambient+5 dB (≈45 dBA), not 68. Helicopters also violate your phase-invariance assumption: certification data shows descending approach is +5 to +9 EPNdB LOUDER than level flyover due to blade-vortex interaction.**
- **Absolute received-level accuracy of about ±5 dB (fixed-wing with NPD) to ±6–10 dB (helicopters/pistons on defaults) is the realistic ceiling without a microphone; relative ranking between types is far more trustworthy. A ~$100–150 miniDSP UMIK-1 with its calibration file, or a class-2 SLM, would let you calibrate the absolute offset per type and cut error to ~±2–3 dB.**

## Key Findings

1. **Certification data is published in three incompatible unit/geometry systems.** Large jets and heavy props: EPNdB at three points (lateral/sideline 450 m, flyover 6.5 km from brake release, approach 2 km before threshold on a 3° glideslope = 120 m height). Light propeller aircraft: a single dBA number (flyover Part 36 Appendix F, or takeoff Appendix G / ICAO Chapter 10). Helicopters: either three EPNdB points (Chapter 8 / Appendix H) or a single flyover SEL in dB(A) for light helicopters (Chapter 11 / Appendix J), measured at 150 m.

2. **EPNdB is not convertible to LAmax without large error.** EPNL is duration- and tone-corrected perceived noise; it can be 10–15+ dB above LAmax and the offset is type-specific. Certification data is defensible for **relative ranking** and as a coarse absolute anchor, not for precise received level.

3. **NPD/ANP is the correct backbone where available**, because it was built precisely to give received SEL/LAmax vs distance. It is not directly downloadable from EASA (request form, Reg. 598/2014 eligibility); legacy versions are more widely circulated.

4. **Simple spherical spreading + linear absorption is adequate to about ±2–3 dB if the absorption coefficient is chosen correctly**, but a single frequency-independent term is the weak link at long range. A per-class effective A-weighted rate is defensible.

5. **Single-event audibility, not annoyance averages, is your metric.** Detectability research used by NPS/FAA finds audibility computed from a detectability index with 10·log(d')=7 dB, and field practice treats aircraft as audible when it exceeds ambient by ~3–4 dB in the relevant bands. Your 68 dBA AUDIBLE floor is ~25 dB too high for a 40 dBA ambient.

6. **Helicopters invert your phase model.** Approach is the loudest phase, not phase-invariant.

## Details

### (a) The certification data landscape — exactly what each source publishes

**ICAO Annex 16 Volume I / FAR Part 36 / EASA CS-36** define the certificated noise standards. The metric and geometry depend entirely on aircraft class:

- **Subsonic jets & heavy propeller transports (>8,618 kg): EPNdB at three reference points.** Per 14 CFR Part 36 Appendix B and ICAO Annex 16 Chapters 3/4/14:
  - **Lateral/sideline:** 450 m to the side of the runway centerline, at the point of max noise during takeoff, full takeoff thrust.
  - **Flyover:** 6,500 m from start of takeoff roll on centerline; thrust may be cut back.
  - **Approach:** 2,000 m before threshold, on a 3° glideslope (≈120 m / 394 ft height), landing configuration.
  - Units: **EPNdB** (Effective Perceived Noise Level), which integrates over the 10-dB-down window (PNLT) and adds tone/duration corrections, normalized to a 10-second reference.

- **Propeller aircraft ≤8,618 kg (light/commuter): a single dBA flyover/takeoff number.**
  - **Appendix F (tests before Dec 22 1988):** level flyover at 1,000 ft (305 m), **dBA** (LAmax).
  - **Appendix G (tests on/after Dec 22 1988):** takeoff climb reference, **dBA**.
  - **ICAO Annex 16 Chapter 10** is the current equivalent (dBA). This is what a TBM, PC-12, Caravan, C172, etc. carry. Reference conditions: sea level, 77°F, 70% RH.

- **Helicopters: two regimes.**
  - **Chapter 8 / FAR Part 36 Appendix H (Subpart H):** three points — takeoff, level flyover (150 m / 492 ft AGL), approach (6° glideslope) — all in **EPNdB**, with sideline stations 150 m either side. The reference level-flyover airspeed is the least of 0.9V_H, 0.9V_NE, 0.45V_H+65 kt, or 0.45V_NE+65 kt.
  - **Chapter 11 / Appendix J (light helicopters ≤3,175 kg / 7,000 lb, alternative):** a single **level-flyover SEL in dB(A)** at 150 m.

**Bulk, machine-readable access paths (with join keys):**
- **ICAO NoiseDB** (noisedb.stac.aviation-civile.gouv.fr), maintained by French DGAC for ICAO: **13,583 certified aircraft records**; browsable list with **XLSX and CSV export**; covers Annex 16 Chapters 2/3/4/14 and FAR Part 36 Stage 3/4/5. Join key: manufacturer + model + variant + MTOW + engine (not the ICAO type designator directly — you must map). Note NoiseDB's own warning that "determination of noise levels by mass interpolation is not an ICAO approved procedure."
- **EASA Certification Noise Levels database** (easa.europa.eu/en/domains/environment/easa-certification-noise-levels): four downloadable files — jet, heavy propeller, light propeller, rotorcraft — covering **over 35,000 aircraft variants** including acoustically significant STCs and transferred products. Best coverage of light aircraft and helicopters.
- **EASA TCDSN** individual PDFs (per type certificate, e.g., EASA.R.009 for EC135); authoritative per-type numbers. Errors/omissions reportable via EASA Forms 46/47.
- **FAA AC 36-1H** with appendices: Appendix 1 (turbojets, EPNdB), Appendix 6 (prop transports, EPNdB), Appendices 7/8 (small props, Appendix F dBA / Appendix G dBA), Appendix 10 (helicopters Appendix H, EPNdB), Appendix 11 (helicopters Appendix J, SEL dBA). Also **AC 36-3H** provides *estimated A-weighted* levels for jets — useful as a rough EPNdB→dBA anchor, but explicitly "intended to provide a consistent basis for comparison of noise levels of major aircraft models rather than establishing absolute levels."

**Categories NOT well covered / caveats:** Piston singles like the C172/C182 certified under old Appendix F have only a single flyover dBA. The push-pull Cessna 337 is a known outlier (see below). Light Sport/experimentals (RV-12) may have only *declared* (not EASA-validated) Part-21 Light data, or nothing.

### (b) Converting certification figures to received level at arbitrary geometry

This is where honesty matters most.

- **EPNdB → LAmax is not a fixed offset.** EPNL adds a duration correction plus tone corrections. Published aircraft-flyover relationships (FAA "Basics of Noise" appendices, multiple airport NEM reports): **SEL is typically 7–12 dB above LAmax**; EPNL and SEL differ again by a duration/tone term. The net EPNL−LAmax gap is commonly 10–15 dB and is type- and spectrum-specific. Any single global constant introduces several dB of error.
- **Certification geometry is fixed and unlike your cruise case.** Approach EPNdB is measured at 120 m directly below on a 3° path in landing config (gear/flaps down, partial power); flyover is at altitude with cutback thrust. Your "1.4 km slant in level cruise" matches none of these configurations. Using the flyover point as the anchor for level cruise is the least-wrong choice, but the configuration mismatch alone is worth several dB.
- **Verdict:** Treat certification EPNdB as a **relative-ranking backbone** and coarse absolute anchor only. This is genuinely useful — it will correctly tell you an A321 approach is louder than an E175 approach, and a Bell 407 approach is louder than its own level flyover.

- **The better path — ANP/NPD.** The Aircraft Noise and Performance database (ANP), underpinning ECAC Doc 29 / ICAO Doc 9911 / FAA AEDT, contains **NPD curves: tabulated noise as a function of slant distance for each aircraft at defined thrust/power settings (approach vs departure)**, plus spectral classes (24 one-third-octave bands) for atmospheric correction. From ANP v2.2 onward the NPD tables provide four metrics — **SEL, EPNL, LAmax and PNLTM**. This is exactly your prediction problem, pre-solved.
  - **Coverage:** EASA describes ANP as "over 160 civil aircraft types"; the broader NPD literature (which includes FAA/Volpe helicopter additions) reports NPD data for roughly **300 fixed-wing aircraft and 26 helicopter types**, derived "from either noise certification tests or controlled tests." Either way it is airliner- and large-GA-centric. **It excludes the generic light-GA INM/AEDT types (GASEPF, GASEPV, COMSEP, COMJET are generics, not in ANP).**
  - **Your types in ANP:** B738/B739, B38M/B39M, A319/A321/A21N, B763, B77L, E75L, CL35, and turboprops like the 1900D and DHC-8 are present or have close substitutes. **Not in the core ANP fixed-wing set:** all four priority helicopters, TBM8, C337, C172/C182, RV12 (PC12 and King Airs are represented via substitutes).
  - **Access:** the ANP dataset is available only to "any competent authority, aircraft operator, air navigation service provider and airport operator as described in Article 7(6) of Regulation (EU) No 598/2014," plus "other parties that can demonstrate a need for modelling purposes in accordance with Article 7(3)," via the EASA ANP data request form. Legacy versions (v1.0–2.3, collected by EUROCONTROL/FAA and verified by the U.S. DOT Volpe Center) are now published on the EASA site together with an **Aircraft Substitutions proxy table** — this is your join to ICAO type designators.
  - **Format:** relational tables (Aircraft, NPD data, spectral class, procedural steps). For a Pi, flatten the NPD table to a per-type array of (distance, LAmax, SEL) and interpolate in log-distance.

- **For helicopters and light GA (your actual dominant traffic), use measured NPD/LAmax from FAA/Volpe and certification levels.** The FAA's INM/AEDT rotorcraft data derive from Volpe measurements; **ACRP Project 02-44 ("Guidance for Helicopter Community Noise Prediction," Wyle/Volpe/NLR)** generated mode-specific NPDs (approach/level/departure) and spectral classes — confirming that, unlike fixed-wing, helicopter NPDs are separate per operational mode. AEDT's fixed-wing NPD tables also give **LAmax at centerline** for GA generics (from an AEDT-based North Slope study): **GASEPF (fixed-pitch single): 69.7 dBA @500 ft, 63.4 @1,000 ft; GASEPV (variable-pitch single): 75.3 @500 ft, 69.0 @1,000 ft; CNA208 Caravan: 85.4 @500 ft, 77.5 @1,000 ft, 70.8 @2,000 ft; CNA441 (light twin turboprop): 75.9 @500 ft, 69.5 @1,000 ft; 1900D: 80.4 @500 ft, 73.8 @1,000 ft, 67.1 @2,000 ft; BEC58P (light piston twin): 78.5 @500 ft, 72.2 @1,000 ft** (all LAmax at centerline). These are directly usable anchors.

### (c) Propagation model adequacy

Your model: RL = source − 20·log10(slant/300) − 0.002·slant.

- **Spherical spreading (20·log10) is correct** for a point source in the far field and holds across your 300 m–12 km range. It does not "break down" except very close to an extended source (irrelevant here) or under strong refraction/ducting.
- **The −0.002·slant linear absorption term is the weak point.** Real atmospheric absorption (SAE ARP 866A, "Standard Values of Atmospheric Absorption as a Function of Temperature and Humidity," 1975; conceptually superseded by SAE ARP 5534 (2013) and ANSI S1.26 / ISO 9613-1) is **strongly frequency-dependent** and depends on temperature and humidity. At mid-frequencies absorption is on the order of a few dB/km; at high frequencies it grows sharply — Part 36 caps validity where the 8 kHz one-third-octave attenuation would exceed 12 dB/100 m (=120 dB/km). Your 0.002 dB/m = 2 dB/km is a reasonable *A-weighted effective* value for the dominant mid-frequency content of prop/rotor noise but will **under-predict attenuation of high-frequency content and jets at long range**.
- **Magnitude of error from a frequency-independent term:** because A-weighted aircraft spectra shift their energy centroid downward as high frequencies are absorbed with distance, the *effective* dB/km is not constant — it is higher near the source and lower far away. Over 300 m–12 km a single fixed coefficient can misestimate absorption by a few dB at the far end. The Volpe "simplified procedure" (basis for replacing ARP 866A) was validated to within about **±0.5 dB up to 100 dB of attenuation** (±7% to 500 dB) against the exact method — i.e., proper frequency-resolved absorption is well understood; the error is entirely in the single-coefficient shortcut.
- **Ground effect and lateral attenuation (SAE AIR 5662):** at low elevation angles (aircraft near the horizon, long slant, low altitude) excess ground attenuation and refraction matter and can add several dB of attenuation or, under inversions, several dB of *enhancement*. For a hospital helipad 1.6 km away at 600 ft, the elevation angle is ~13° — low enough that lateral attenuation is non-trivial but not extreme. The NPS Grand Canyon field data illustrate how much real attenuation can exceed the ideal: Bell 407 noise dropped ~17 dBA between 100 ft and 400 ft (≈8.5 dBA per doubling, versus the 6 dB free-field ideal), because absorption and ground effects add to spreading.
- **Practical Pi recommendation:** Keep spherical spreading. Replace the single 0.002 term with **per-class effective A-weighted absorption coefficients**: use ~**1.5–2.5 dB/km for rotor/large-prop noise (low-frequency dominated)** and ~**3–5 dB/km for jets and high-RPM small props (more high-frequency content)**, at nominal 15°C/70% RH. Optionally implement a crude **two-band split** (a low band ~500 Hz at ~1 dB/km and a high band ~2–4 kHz at ~8–15 dB/km) combined post-hoc — this captures most of the spectral-shift effect at trivial cost. Do not attempt full ISO 9613-2 on the Pi; it buys little for a screening tool.

### (d) Single-event audibility thresholds for a ~40 dBA ambient

Your current buckets (VERY LOUD ≥88, LOUD ≥78, AUDIBLE ≥68, FAINT <68) put the audible floor ~28 dB above ambient — far too high. The single-event literature:

- **WHO guidance and long-term averages (DNL, Lnight 40 dB) are the wrong tool** for you; they are annual averages. But WHO's single-event work gives usable anchors. The WHO *Night Noise Guidelines for Europe* (2009, p.99) states: **"on the basis of the present available evidence we propose to assume that NOAEL_Amax = 42 dB(A)"** (the no-observed-adverse-effect indoor maximum level), adding that **"noise events exceeding 45 dBA should therefore be limited if possible."** The WHO *Guidelines for Community Noise* (Berglund & Lindvall) set an indoor single-event guideline of **45 dB LAmax** in bedrooms, noting that when noise is not continuous "sleep disturbance correlates best with LAmax and effects have been observed at 45 dB or less." With open-window insulation ~15 dB (closed ~24 dB), an indoor 42–45 dB corresponds to roughly **57–66 dBA outdoors** for sleep disturbance.
- **Detectability/audibility (NPS/FAA, INM/NMSim):** per the FAA report *Assessment of Tools for Modeling Aircraft Noise in the National Parks*, "detectability levels (d') calculated in support of this project are based on the signal-to-noise ratio within one-third octave-band spectra, using a **10·log(d') value of 7 dB**. Both INM and NMSim utilize d' to calculate audibility." NPS field toolkits (NPS-ActiveSpace) treat flight noise as audible when it is **~3–4 dB above the natural ambient** in the relevant bands. Part 36 itself requires the signal to exceed ambient by ≥3 dB per one-third-octave band to be a valid measurement.
- **Number-above-ambient practice:** FAA/community overflight metrics increasingly use "N events above ambient+10 dB" (NAA+10) as a nuisance threshold, with ambient estimated by community type (35 dB rural, 40 low-density suburban, 50 medium suburban, 60 urban).

**Proposed defensible buckets for a 40 dBA ambient (received LAmax, outdoor):**
- **FAINT / not reliably audible: < 45 dBA** (within ~5 dB of ambient; may be masked).
- **AUDIBLE: 45–55 dBA** (ambient+5 to +15; clearly detectable, onset of "noticeable").
- **LOUD / intrusive: 55–70 dBA** (interferes with conversation/TV; ambient+15 to +30; NAA+10 nuisance region and sleep-disturbance onset).
- **VERY LOUD: ≥ 70 dBA** (dominant single events; approaching the classic 65 dBA speech-interference / awakening region and above).

Tie each threshold to the ambient you actually assume: for a rural night (~35 dBA) shift all thresholds down ~5 dB. Report the ambient assumption alongside every score.

### (e) Lookup table for the listed types

Notation: **[EPNdB TO/FO/APP]** = certificated takeoff / flyover / approach in EPNdB (Chapter 8 / Part 36 Appendix H). **[dBA]** = single-number flyover/takeoff (Chapter 10 / Appendix F/G). **[SEL]** = Chapter 11 / Appendix J flyover SEL dB(A) at 150 m. **LAmax@X** = measured/AEDT LAmax at slant/height X. Helicopter EPNdB below are from EASA TCDSN (primary sources, per subagent extraction).

**HELICOPTERS (priority — reference geometry 150 m / 492 ft; approach on 6° path):**
| ICAO | Type | Certificated levels | Notes / measured |
|---|---|---|---|
| B407 | Bell 407 | EASA.IM.R.512 (Issue 6, 2026): **TO 88.3 / FO 85.4 / APP 90.8 EPNdB** (2,018 kg); Ch.11 **FO SEL 84.6 dBA** (2,268 kg). (Older NZ CAA TAR cites 88.4/85.2/90.7 — consistent.) | NPS Report GRCA-07-05 (Falzarano & Levy 2007), ANSI Type 1 meter: level overflight **77.5 dBA LAmax @400 ft AGL**; landing 78.5, takeoff 73.1, hover 82.0 (all @400 ft). Volpe/ACRP 02-44 NPDs exist. |
| EC35 | EC135/H135 | EASA.R.009 (Issue 12, 2022): P2+ **TO 88.6 / FO 84.0 / APP 92.7**; T2+ **TO 88.3 / FO 85.7 / APP 94.9**; P3H (newest) **TO 86.4 / FO 82.7 / APP 90.4 EPNdB**; Ch.11 FO SEL **80.2–81.4 dBA** (T1/P1) | No fixed-distance field LAmax located; use cert + Bell 407 analog. |
| EC45 | H145/EC145/BK117 | EASA.R.010: BK117 D-2 (=H145) **TO 87.1 / FO 84.7 / APP 90.3 EPNdB** (3,800 kg); D-2m **TO 86.5 / FO 84.7 / APP 90.3** | Approach loudest. |
| A119 | AW119 Koala | EASA.R.005 / Agusta rpts (via NZ CAA TAR): Ch.11 **FO SEL 85.9 dBA (FAA) / 86.5 (ICAO)**; AW119 MKII Ch.8 **FO 88.2 / TO 90.8 EPNdB** (approach EPNdB not found — gap) | |

**Key helicopter finding:** approach is **+5 to +9 EPNdB louder than level flyover** for all four (Bell 407 +5.4; EC135 T2+ +9.2; EC135 P2+ +8.7; H145 +5.6). This is BVI and **inverts your phase-invariant rotorcraft assumption.**

**TURBOPROPS (Chapter 10 / Appendix G dBA where available; AEDT LAmax anchors):**
| ICAO | Type | Data |
|---|---|---|
| TBM8 | TBM 850 | Certified Chapter 10 flyover dBA in AFM (single number). Was mis-binned as "light" — it is a 700-hp PT6 single, closer to PC-12/Caravan. Use CNA208-like NPD as proxy (≈77–85 dBA @500–1,000 ft). |
| PC12 | Pilatus PC-12 | Certified Chapter 10; AFM Section 4 "Noise Level" (e.g., a Chapter-10 flyover figure cited around 79.3 dBA class). Use CNA208/CNA441 NPD proxy. |
| BE99 | Beech 99 | Appendix F/G dBA; older commuter turboprop. |
| B190 | Beech 1900 | AEDT 1900D: **80.4 @500 ft, 73.8 @1,000 ft, 67.1 @2,000 ft**. |
| C208 | Cessna Caravan | AEDT CNA208: **85.4 @500 ft, 77.5 @1,000 ft, 70.8 @2,000 ft**. |
| B350 | King Air 350 | Appendix G dBA; AEDT CNA441 proxy (**75.9 @500, 69.5 @1,000**). |

**PISTON (Appendix F/G dBA; AEDT GASEPF/GASEPV/BEC58P):**
| ICAO | Type | Data |
|---|---|---|
| C337 | Cessna 337 Skymaster | **Known outlier — anomalously loud.** Documented mechanism (Wikipedia/type refs): the rear pusher prop "slic[es] through turbulent air from the front propeller and over the airframe," and per operator accounts "the turbulent air pushed by the front propeller meets the air from the rear propeller at higher speed, thus generating almost supersonic speeds of the prop blade tips." Treat as **+5 to +8 dB above a normal light twin (BEC58P)** as a LABELED engineering estimate — no clean certification dBA located. Your measured 70.8 @480 m is consistent with it being louder than GASEPV. |
| C172 | Cessna 172 | AEDT GASEPF: **69.7 @500 ft, 63.4 @1,000 ft**. Appendix F ~ high-60s to low-70s dBA. |
| C182 | Cessna 182 | GASEPV (variable pitch): **75.3 @500 ft, 69.0 @1,000 ft**. Volpe measured C182 in the NPS campaign. |
| PA32 | Piper PA-32 | GASEPV class (FAA substitution list groups PA-32 with GASEPV). |
| RV12 | Van's RV-12 | Light Sport; declared (not validated) EASA Part-21 Light data at best; treat as GASEPF-quiet. |

**JETS (EPNdB from AC 36-1H / NoiseDB; ANP NPD available):** representative values illustrate ranking (exact per-variant in NoiseDB export):
| ICAO | Type | Notes |
|---|---|---|
| A319 | A319-111 | AC 36-1H format ~ **80.5 (flyover) / 91.1 (lateral) / 93.2 (approach) EPNdB**. In ANP. |
| A321 / A21N | A321 / A321neo | In ANP; A21N notably quieter than A321ceo. |
| B738 / B739 | 737-800 / -900 | In ANP; validated spectral classes. |
| B38M / B39M | 737 MAX 8/9 | In ANP (recent additions). |
| E75L | E175 | In ANP. |
| B763 | 767-300 | In ANP. |
| B77L | 777-200LR/F | In ANP; heavy. |
| B78X | 787-10 | In ANP; among quietest widebodies per EPNdB margin. |

**BUSINESS JETS:**
| ICAO | Type | Notes |
|---|---|---|
| C560 | Citation V/Ultra | EPNdB in NoiseDB; small ANP presence. |
| CL35 | Challenger 350 | In ANP. |
| LJ60 | Learjet 60 | EPNdB in NoiseDB. |

**Correction procedure to arbitrary slant range (per row):**
1. If ANP/NPD exists: interpolate LAmax (or SEL) directly from the NPD (distance) curve at your slant range for the appropriate power mode (departure for climb, approach for descent). No propagation math needed — NPD already embeds spreading + absorption.
2. If only a certification number at reference distance d0 exists: RL(d) = L_ref − 20·log10(d/d0) − α_class·(d−d0)/1000, with d0 = 150 m (helicopter), 305 m (Appendix F prop), or the certification point; α_class per (c).
3. For helicopters, add a **phase term**: APP = FO + 6 dB (mean of measured spread), TO ≈ FO + 3 dB, level = FO. Use the type's own EPNdB spread where available.

**Honest error bars:** fixed-wing with NPD: **±3–5 dB** absolute LAmax. Certification-anchored (no NPD): **±5–8 dB**. Helicopters from cert + phase term: **±5 dB** relative between phases, **±5–7 dB** absolute. Light pistons on generic defaults: **±6–10 dB**. Relative ranking between types: much tighter, roughly **±2–3 dB**.

### (f) Flight-phase dependence

- **Jets:** a ~15 dB climb-vs-descent spread is **too large**. Certification approach EPNdB is often *higher* than flyover (cutback) EPNdB (e.g., the A319 numbers above show approach ≈ 93 vs flyover ≈ 91). In NPD terms, departure (high thrust) LAmax exceeds approach LAmax at a given distance by perhaps 3–8 dB, not 15. Use NPD departure vs approach curves rather than a fixed spread. Your heavy 105/95/92 climb/level/descent spread of 13 dB overstates it.
- **Helicopters:** your phase-invariance is wrong in the **opposite** direction — **approach is loudest** due to blade-vortex interaction (BVI), strongest in ~6–12° descents at ~60–80 kt, radiated forward and down (the "blade slap"). Certification data quantifies it: **approach exceeds level flyover by +5.4 (B407), +8.7–9.2 (EC135), +5.6 (H145) EPNdB.** Controlled experiments (Boeing-SMART active-flap rotor, AHS 2009) show ~2–7 dB of that is suppressible BVI. So for rotorcraft, set **APP > TO ≳ level**, not flat.

## Recommendations

**Stage 1 — fix thresholds and phase logic now (no new data needed).**
- Recalibrate buckets to ambient-relative: FAINT <45, AUDIBLE 45–55, LOUD 55–70, VERY LOUD ≥70 dBA (for 40 dBA ambient), and expose the ambient assumption as a parameter.
- Split helicopters by phase with APP = FO+6, TO = FO+3, level = FO (dB). Stop treating rotorcraft as phase-invariant.
- Reduce the jet climb/descent spread from ~15 dB to ~5–8 dB.
- Reclassify the TBM out of "light piston" into "turboprop single" (Caravan-class), which your split now allows.

**Stage 2 — replace invented source levels with sourced anchors.**
- Pull the **EASA Certification Noise Levels** four files and the **ICAO NoiseDB CSV/XLSX export**; build your lookup keyed by ICAO type via a manual map (NoiseDB gives manufacturer/model/variant/MTOW/engine).
- For helicopters, hard-code the EASA TCDSN EPNdB triplets above (B407, EC35, EC45; A119 partial).
- For light GA/turboprops, adopt the **AEDT NPD LAmax-vs-distance anchors** listed in (e).

**Stage 3 — get the NPD backbone for fixed-wing.**
- Request **ANP data from EASA** (or obtain a legacy version) and flatten NPD tables to per-type (distance, LAmax, SEL) arrays plus the substitution table. This upgrades all your airliner/bizjet predictions to ±3–5 dB and removes the need for propagation math on those types.

**Stage 4 — calibrate with a cheap microphone (optional but high-value).**
- A **miniDSP UMIK-1 (~$100–150) with its unique serial-numbered calibration file** — a driverless USB Audio Class 1.0 omni mic flat to 20 Hz–20 kHz ±1 dB with calibration loaded, with documented Raspberry Pi recording via SoX — or a **class-2 sound level meter with acoustic calibrator (~$150–300)**, would let you record LAmax for a few dozen known overflights and fit a **per-class absolute offset and effective absorption coefficient**. This is the single highest-leverage upgrade: it converts your ±5–10 dB absolute error into ±2–3 dB and directly validates the C337 outlier and the helicopter phase terms.

**Benchmarks that change the plan:**
- If ANP access is denied → stay on certification+AEDT anchors and widen fixed-wing error bars to ±5–8 dB; do not claim absolute accuracy.
- If a calibration mic shows systematic >5 dB bias for a class → adjust that class's source offset, not the propagation term.
- If helicopter complaints dominate and cert data proves insufficient → obtain ACRP 02-44 / Volpe Bell 407 NPD tables and the EASA **NORAH2** rotorcraft noise-hemisphere tool for approach directivity.

## Caveats

- **Absolute vs relative:** every number here is far more reliable as a *ranking* than as an *absolute* received level. State scores as ranges/buckets, never false-precision dB.
- **Configuration & weight are unknown to you** (gear/flap/thrust/mass), and these move certification-vs-actual by several dB. NPD partly handles this via power mode; certification anchors do not.
- **EPNdB→LAmax conversion carries ~±3–5 dB irreducible error**; treat EPNdB rows as ranking anchors.
- **Propagation:** a single absorption coefficient misestimates far-field high-frequency attenuation by a few dB; the two-band option mitigates but does not eliminate this.
- **Directivity is ignored:** helicopters especially radiate BVI forward-and-down; a receiver ahead of the aircraft on approach hears more than one behind. Slant range alone cannot capture this.
- **Meteorology (temperature inversions, wind) can add ±several dB** and occasionally *enhance* rather than attenuate; unmodeled here.
- **What only a microphone can fix:** the absolute per-type offset, the true C337 excess, actual atmospheric/ground conditions on the day, and real directivity. Without it, ~±5 dB absolute (fixed-wing/NPD) to ~±6–10 dB (pistons/helicopters on defaults) is the honest floor.
- **Explicit data gaps:** AW119 MKII approach EPNdB not located; no fixed-distance field LAmax for EC135/H145/AW119 (Volpe's NPS campaign measured only the Bell 407 among your four); C337 has no clean certification dBA — its "+5 to +8 dB" penalty is a labeled engineering estimate, not a measurement. Also note the subagent flagged that the widely reproduced NPS GRCA-07-05 figure of "takeoff 80.2 dBA" belongs to the *MD-900*, not the Bell 407 (Bell 407 takeoff was 73.1 dBA @400 ft); verify against the primary report before use.
- **"Unknown" is the right output when only a coarse ADS-B emitter category is available and it spans classes** — e.g., "small" covering both a C172 at ~64 dBA@1,000 ft and a Caravan at ~78 dBA@1,000 ft (a 14 dB spread). When the category default's plausible range crosses two of your buckets, report UNKNOWN rather than guess.