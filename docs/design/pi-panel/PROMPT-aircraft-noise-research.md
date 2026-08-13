# Research prompt — aircraft noise levels for a domestic overflight monitor

Paste the section under **"The prompt"** into Claude.ai research mode. Written
2026-08-13 against `deploy/stopgap/panel-adsb-status`.

Everything the model currently uses for loudness is invented. This is the brief
for replacing it with published data.

---

## The prompt

I am building a small system that predicts, from ADS-B data, whether an aircraft
about to pass over my house will be loud. I need help replacing the invented
numbers in it with real published data. Please research thoroughly and cite
sources; I would rather have a well-sourced partial answer than a confident
complete one.

### What the system does

A Raspberry Pi with an RTL-SDR receives ADS-B, and a separate feed from the
Flightradar24 API supplements it. For each aircraft I know, at 1 Hz:

- ICAO 24-bit address, and often the exact ICAO type designator (`B739`, `EC35`,
  `TBM8`, `C337`); sometimes only the coarse ADS-B emitter category
- position, so slant range from a fixed point on the ground, in metres
- barometric altitude
- vertical rate, so climb / level / descent
- ground speed and track

From that I compute a predicted closest approach and score how loud it will be.
There is no microphone anywhere in the system, and no measurement of any kind.

### The current model, and why it is inadequate

Aircraft are binned into five classes, each with an invented "source level" in
dB at 300 m, varying by flight phase:

```
heavy       climb 105  level 95  descent 92
jet         climb 100  level 88  descent 85
turboprop   climb  94  level 86  descent 83
light       climb  85  level 78  descent 75
rotorcraft  climb  92  level 92  descent 90
```

Received level is then `source − 20·log10(slant/300) − 0.002·slant`, i.e.
spherical spreading plus a crude atmospheric absorption term. The result is
bucketed:

```
VERY LOUD ≥ 88    LOUD ≥ 78    AUDIBLE ≥ 68    else FAINT
```

Every one of those fourteen numbers was guessed. Two independent errors have
already shown up against my own ears:

1. A Daher TBM 850 (`TBM8`, 700 hp PT6 turboprop) passing 1.07 km away at
   2,075 ft was rated FAINT. I heard it clearly. It had been binned as "light"
   alongside Cessna 172s; I have since split turboprops out, but the new number
   is another guess.
2. A Cessna M337B Skymaster (`C337`, push-pull piston twin) predicted to pass
   480 m away at 1,382 ft scored 70.8, which the thresholds call AUDIBLE. That
   felt roughly right — but it means anything scoring 55–67 is called FAINT
   while being plainly hearable, since outdoor ambient here is maybe 40 dB.

So I suspect both the source levels and the thresholds are wrong, and I cannot
tell which is contributing more.

### What I actually need

**1. Published noise data per aircraft type.**
ICAO Annex 16 Volume I, EASA Type Certificate Data Sheets for Noise, and FAA
AC 36-1H all publish certificated noise levels. I would like to understand:

- What exactly do these publish — EPNdB? dBA? At what reference points
  (takeoff / flyover / sideline / approach), at what distances, and under what
  aircraft configuration and power setting?
- Where can the underlying datasets be obtained in bulk, ideally machine-readable
  and keyed by something I can join to an ICAO type designator?
- Which aircraft categories are *not* covered. My understanding is that light
  piston aircraft fall under Annex 16 Chapter 10 / FAR Part 36 Appendix G with a
  single flyover number, and helicopters under Chapter 8 or 11, but I would like
  this confirmed and explained.

**2. How to convert certification figures into a received level at arbitrary
geometry.** This is the part I expect to be hardest and where I most want honesty
about the limits. Certification measures a specific profile at a specific point.
I need a level at, say, 1.4 km slant range from an aircraft in level cruise. Is
that conversion defensible at all? If it is only valid for relative ranking
between types rather than absolute levels, please say so plainly — relative
ranking is genuinely useful to me and I would rather know its limits than
overstate it.

**3. Whether my propagation term is right.** Is `20·log10(d/d₀)` plus a linear
absorption coefficient adequate for 300 m–12 km slant ranges, or should I be
using ISO 9613-2, SAE ARP 866, or something else? What does atmospheric
absorption actually depend on — frequency, humidity, temperature — and how much
error does ignoring that introduce over these distances?

**4. Audibility thresholds.** What received level outdoors is actually
noticeable, disruptive, or sleep-disturbing in a quiet residential setting? WHO
Environmental Noise Guidelines and FAA/FICON day-night level criteria seem
relevant but are long-term averages, whereas I care about single events. Is
there a better basis for a single-event threshold — SEL, L_max, or something
else?

**5. Specific types I most need numbers for**, being what actually flies here:

- Helicopters, the priority: `B407` (Bell 407), `A119` (Leonardo AW119),
  `EC35` (H135), `EC45` (H145). These are air ambulances serving a hospital
  helipad 1.6 km away and are the dominant complaint. Departures pass at under
  1 km, at 600 ft, climbing at over 1,000 fpm.
- Turboprops: `TBM8`, `PC12`, `BE99`, `B190`, `C208`, `B350`
- Piston: `C337` (the push-pull twin — is it anomalous?), `C172`, `C182`,
  `PA32`, `RV12`
- Jets, mostly at altitude but on approach here: `E75L`, `B739`, `B738`,
  `B38M`, `B39M`, `A321`, `A319`, `A21N`, `B763`, `B77L`, `B78X`
- Business jets: `C560`, `CL35`, `LJ60`

**6. Does flight phase matter as much as I assume?** I currently apply roughly a
15 dB spread between climb and descent for jets, on the reasoning that takeoff
thrust dominates. Is that supported? For helicopters I apply almost no phase
variation, assuming rotor noise dominates over engine power — is that right, and
does blade-vortex interaction during descent make them *louder* on approach,
which would invert my assumption?

### What would be most useful to receive

A table I can turn into a lookup, keyed by ICAO type designator where possible
and by category where not, with a source level and its reference conditions, plus
a clear statement of what geometry it is valid for. Alongside that, whatever
correction I need to get from those reference conditions to an arbitrary slant
range — and an honest assessment of the error bars.

If the answer to part of this is "you cannot get there without a microphone",
that is genuinely useful too, and I would rather hear it than get a plausible
number that turns out to be fiction. In that case, I would want to know what the
cheapest defensible measurement setup looks like and what it would let me
calibrate.

### Constraints worth knowing

- Everything runs offline on a Raspberry Pi; a static lookup table is ideal, and
  anything requiring a network call at prediction time is not.
- I only reliably know: ICAO type, slant range, altitude, vertical rate, ground
  speed. No engine setting, no configuration, no weight.
- Roughly 40% of aircraft the receiver hears broadcast no type at all, so I also
  need sensible per-category defaults for when the type is unknown — and I would
  rather report "unknown" than guess, so I need to know when a default is too
  weak to be worth using.
