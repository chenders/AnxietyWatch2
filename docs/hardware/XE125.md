# XE125 / XM125: respiratory-rate assessment

## Bottom line

The photos identify this as an **Acconeer XE125 evaluation board, revision PB41 V1.3**, with an **XM125 module** fitted. The XM125 contains an **A121 60 GHz pulsed-coherent radar** and an STM32L4 microcontroller.

**Yes: for a prototype, this hardware alone can estimate respiratory rate.** The minimum setup is:

- the XE125;
- a USB-C **data** cable and a host computer or Raspberry Pi; and
- compatible Acconeer module firmware plus the Exploration Tool/software.

No respiratory belt, ECG, camera, microphone, or pulse oximeter is required merely to produce a breathing-rate estimate. Extra sensing becomes important for validation and for any medical or safety claim.

## What the board is

| Part | Purpose |
|---|---|
| **XE125** | Evaluation/carrier board. It supplies power, USB-to-serial connectivity, debug access, reset, and GPIO test points. |
| **XM125** | The soldered radar module. It exposes the radar and MCU through serial/I2C and can also run a custom embedded application. |
| **A121** | 57–64 GHz radar IC with integrated RF front end, baseband, and antenna-in-package. |
| **STM32L431CBY6** | 80 MHz Cortex-M4 on the XM125, with 128 kB flash and 64 kB RAM. |

The XE125 documentation says that USB connection creates two virtual serial ports, **Enhanced** and **Standard**. The Enhanced port is the one intended for the Exploration Tool. The board's USB1 connector is the useful connector; USB2 is marked as having no mount on the evaluation board.

This is radar, not an optical sensor. It measures reflections at selected distances and returns complex **I/Q** samples. The A121's integrated antenna means that an external radar antenna is not needed.

## How the breathing application works

The official A121 breathing reference application does approximately this:

1. Run the presence processor to find a person and estimate the person's distance.
2. Select a small group of range points around the person.
3. Track the phase of the complex radar return. Chest motion from inhalation and exhalation changes that phase.
4. Build a time series, band-pass it over the anticipated breathing-rate range, and find the dominant peak in its power spectrum.
5. Report the peak as breaths per minute.

At 60 GHz the free-space wavelength is about 5 mm. For a monostatic radar, a full phase rotation corresponds to approximately half that wavelength in target displacement, about 2.5 mm. Consequently, phase tracking can reveal much smaller relative motion than the radar's ordinary range resolution. This is why a radar that cannot spatially resolve every part of a chest can still detect breathing motion.

The reference algorithm is an estimator of **breathing frequency and chest displacement**. It does not measure tidal volume, airflow, blood oxygen, carbon dioxide, or blood gases.

### Timing and defaults

The current reference application's defaults include:

- measurement range: approximately 0.3–1.5 m;
- 5 s for initial distance determination;
- 20 Hz frame rate;
- a 20 s breathing time series; and
- an anticipated range of 6–60 breaths/min.

The time-series window must fill before the first reliable spectral estimate. In practice, expect a startup delay on the order of tens of seconds, with a new stable estimate roughly every half-window rather than a new clinically meaningful value on every radar frame.

Acconeer's documentation includes example tests at 0.5, 1, and 2 m, but those are demonstrations, not clinical validation or a guarantee for every room, body position, or sensor unit.

## What is required beyond software

### For a first experiment

Only modest practical hardware is needed:

- a data-capable USB-C cable;
- a stable host (a laptop or the Raspberry Pi 5 in this project); and
- a rigid mount that keeps the sensor from moving.

Start at roughly 0.5–1 m with one stationary person, aim the antenna face toward the chest or upper abdomen, and keep other moving objects out of the measured range. The official calibration guidance recommends a lens for distances beyond about 1 m; start without one at shorter range.

The Python Exploration Tool is the quickest route. Flash a module firmware version compatible with the Exploration Tool, launch the tool, choose the **Enhanced** serial port, and select the A121 Breathing reference application. The repository also contains command-line examples under `examples/a121/algo/breathing/`.

This repository now includes a bench-only JSONL wrapper around that reference application. From the repository root, run:

```sh
uv run --with 'acconeer-exptool[app]' tools/xe125_breathing.py \\
  --serial-port /dev/ttyUSB0 \\
  --raw-record xe125-session.h5
```

Replace `/dev/ttyUSB0` with the XE125 **Enhanced** port. The wrapper emits one flushed observation per frame, preserves no-presence/motion/initialization states, and never writes the vitals spool. A raw H5 recording is optional but recommended for replay.

For an eventual overnight installation, an enclosure/radome must be chosen so it does not introduce troublesome 60 GHz reflections. A lens may improve longer-distance performance. Neither is required to establish that the bare evaluation board can detect breathing.

### For confidence in the result

A second sensor is not required to run the algorithm, but it is required to evaluate accuracy responsibly. Use at least manual breath counts for an initial check; a respiratory-inductance belt is much better, and airflow/capnography is a stronger reference when testing apnea or ventilation. An ECG chest strap such as the Polar H10 is useful for cardiac reference, but it is not by itself a direct gold-standard measurement of airflow or respiratory effort.

### For clinical-style monitoring

The XE125 should not be treated as sufficient by itself. Radar chest motion can be absent because of sensor dropout, a changed position, occlusion, or movement—not necessarily apnea. Conversely, chest effort can continue during obstructed breathing without effective ventilation. Radar also cannot report oxygen desaturation.

A system intended to detect respiratory compromise would need independent evidence such as pulse oximetry and, depending on the claim, an airflow/effort measurement, plus extensive validation. The project disclaimer applies: a prototype respiratory-rate number is not a medical measurement.

## Important limitations to test

The reference application is designed for a **stationary person**. It pauses or reinitializes when large/intra-presence motion is detected. Expect trouble from:

- turning over, sitting up, limb motion, or another person in the beam;
- a poor angle to the chest or a changed range;
- nearby curtains, fans, pets, furniture, or metal objects;
- weak or changing reflections from bedding and clothing; and
- sensor movement or an electrically troublesome enclosure.

A missing or invalid result should be treated as **not measured**, not as zero breathing and not as proof that breathing is normal.

## Other things this radar can do

The A121/XM125 platform is general-purpose. Depending on the firmware, service, and processing software, the same raw radar capability can support presence and micro-motion detection, distance and velocity, phase/vibration tracking, gestures, touchless controls, parking/occupancy, level sensing, and breathing. The linked Exploration Tool repository has examples and reference applications for many of these. A121 datasheets also mention pulse-rate research, but that should not be confused with a ready, clinically validated pulse algorithm in this repository.

## Recommended first milestone

1. Connect the XE125 to the Pi or laptop over USB1.
2. Confirm that the Enhanced serial port appears.
3. Flash matching XM125/A121 module software if necessary.
4. Run the stock breathing reference application before changing parameters.
5. Record the raw session and manually count breaths at the same time.
6. Repeat seated and lying down, then deliberately test movement and loss of presence.
7. Only after those tests should we write a collector/decoder for AnxietyWatch2 and decide whether radar rate is suitable as a corroborating signal.

## References

- [A121 breathing reference application](https://docs.acconeer.com/en/latest/ref_apps/a121/breathing.html)
- [A121/XM125/XE125 setup](https://docs.acconeer.com/en/latest/getting_started/evk_setup/xm125.html)
- [Breathing example scripts](https://github.com/acconeer/acconeer-python-exploration/tree/master/examples/a121/algo/breathing)
- [Exploration Tool](https://github.com/acconeer/acconeer-python-exploration)
- [A121 datasheet](https://developer.acconeer.com/download/A121-datasheet)
- [XM125 datasheet](https://developer.acconeer.com/download/XM125-datasheet)
