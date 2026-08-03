# Key custody options

**Date:** 2026-08-03
**Context:** `docs/specs/2026-08-03-amendment-02-integrity-and-encryption.md` §5

**None of this is required.** The system runs with software-held keys and tells you so. A security
module raises the guarantee; it is not a prerequisite, and the software supports several so you can
use whatever you already own.

Two jobs a module can do here:

1. **Custody of the ingest signing key** so it cannot be extracted, plus a monotonic counter so
   existing history cannot be altered or deleted undetectably.
2. **Unattended LUKS unlock** without a keyfile on the boot SD card.

---

## 1. The system adapts to what is present

`vitalsd` probes for a custody backend at startup, in this order, and uses the first that answers:

```
TPM 2.0 (/dev/tpmrm0)  →  ZYMKEY (I2C 0x30)  →  ATECC608 (I2C 0x60)  →  software keys
```

Whatever it finds, the **health endpoint and the admin console state the level achieved** — because
"unknown is not OK" applies to the security posture too. A system running on software keys says so
plainly rather than implying a guarantee it does not have:

```json
{
  "key_custody": "software",
  "key_extractable": true,
  "rollback_detection": false,
  "boot_integrity": false,
  "tamper_response": false,
  "note": "Signing key is on disk. Provenance is recorded but not tamper-resistant."
}
```

Override the probe with `VITALS_KEY_CUSTODY=tpm2|zymkey|atecc|software` when you want to force a
backend or prove the fallback path in a test.

---

## 2. What each option buys

| | Software | ATECC608B | TPM 2.0 | ZYMKEY 5 |
|---|---|---|---|---|
| Typical cost | — | ~$5 | ~$25 | ~$100 |
| Interface | — | I2C | GPIO SPI | GPIO I2C |
| Key non-extractable | ✗ | ✓ | ✓ | ✓ |
| Monotonic counter → rollback detection | ✗ | ✓ | ✓ | ✓ |
| PCR sealing / boot integrity | ✗ | ✗ | ✓ | vendor equivalent |
| LUKS unattended unlock | keyfile | keyfile | **`systemd-cryptenroll`** | vendor tooling |
| Physical tamper response | ✗ | ✗ | ✗ | **✓ key destruction on breach** |
| Tooling on the boot path | n/a | n/a | standard | proprietary |

### Software keys — the honest baseline

Ed25519 key on disk via systemd `LoadCredentialEncrypted=`, which encrypts against a host-bound
secret readable only by root. You still get: provenance per batch, the hash-chained audit trail, and
a permanent answer to "did this come from the BP collector or from a script."

You do not get: protection from anyone who can read the key. Which for a single-user box you own,
sitting in your bedroom, may be exactly the right trade.

### ATECC608B — the cheap real upgrade

Non-extractable ECDSA keys and monotonic counters for the price of a coffee. Closes the two threats
that matter most for a medical record — offline forgery and silent history rewriting — and does
nothing for disk encryption. A genuinely reasonable stopping point, not a token.

### TPM 2.0 — the best default if you are buying something

The decisive property is not features, it is that **systemd supports it natively**:

```sh
systemd-cryptenroll /dev/nvme0n1p2 --tpm2-device=auto --tpm2-pcrs=7+11
```

A documented, widely-deployed path to PCR-sealed unlock. For the one component sitting between the
machine booting and the alarm existing, boring and well-tested beats capable. Also brings
anti-hammering (locks after 32 auth failures, forgets one per 10 minutes) and TPM command auditing.

Note that many x86 machines already have a TPM on the motherboard — if you are running this on
something other than a Pi, you may already have this and not know it. The probe will find it.

#### Which TPM board, specifically, for a Raspberry Pi 5

All of these are Infineon OPTIGA parts on SPI and all use the same kernel driver (`tpm_tis_spi`) and
the same device-tree overlay. The differences are packaging, price and availability.

| Board | Chip | Where |
|---|---|---|
| **LetsTrust TPM** (pi3g) | SLB9672 | Best board, mostly **European** stock — see §US below |
| **ANAVI TPM 2.0** | SLB9672 | [Crowd Supply](https://www.crowdsupply.com/anavi-technology/anavi-tpm-2-0-for-raspberry-pi) — US-friendly, open hardware |
| **GeeekPi TPM9670** | SLB9670 | Amazon, ASIN **`B09G2BZQT5`** — cheapest, older chip, in US stock |
| Infineon Iridium eval board | SLB9670 | DigiKey / Mouser, US stock, but $57–64 |
| Infineon SLB9673 boards | SLB9673 | **I2C, not SPI.** Kernel ≥ 6.0 supports it but may need enabling at compile time. Skip unless you specifically want SPI free |

#### Buying from the US

LetsTrust is the best board but is sold mainly through European and APAC retailers, so shipping and
lead time are the deciding factor rather than the hardware.

**Search terms that work:** `LetsTrust TPM Raspberry Pi` · `TPM 2.0 module Raspberry Pi SLB9670` ·
`Infineon OPTIGA TPM Raspberry Pi SPI`

| Route | Part / link | Notes |
|---|---|---|
| **Amazon (fastest)** | GeeekPi TPM9670, ASIN [`B09G2BZQT5`](https://www.amazon.com/dp/B09G2BZQT5) | SLB9670. Ships now, cheap, works with the standard overlay. **This is the pragmatic pick if you want it this week.** |
| **Crowd Supply** | [ANAVI TPM 2.0](https://www.crowdsupply.com/anavi-technology/anavi-tpm-2-0-for-raspberry-pi) | SLB9672, open hardware, US fulfilment. Check whether it is in stock or in a campaign window |
| **DigiKey** | `IRIDIUMSLM9670TPM20TOBO1` (~$64) · `IRIDIUMSLI9670TPM20TOBO1` | Infineon's own eval board. Guaranteed genuine, guaranteed US stock, roughly 2.5× the price of the Amazon board for the same chip |
| **Mouser** | `IRID9670TPM20LINUXTOBO1` | Same board, "Iridium 9670 TPM2.0 Linux" |
| **LetsTrust direct** | [pi3g](https://pi3g.com/products/industrial/letstrust-tpm/) · [The Pi Hut](https://thepihut.com/products/letstrust-tpm-for-raspberry-pi) (UK) · [Cytron](https://www.cytron.io/p-letstrust-tpm-for-raspberry-pi) (MY) · [BerryBase](https://www.berrybase.de/en/letstrust-tpm-cryptography-module-for-raspberry-pi) (DE) · Core Electronics `CE10600` (AU) | The SLB9672 board. Worth the wait if you are not in a hurry |

**Bare chips**, if you are building your own board: `SLB9670VQ20FW785XTMA1` (Mouser),
`SLB9670VQ20FW760XUMA1` (DigiKey). Note the Infineon variant letters — **SLB** is the standard part;
**SLM** and **SLI** are mobile and industrial/automotive grades of the same silicon and are equally
fine here.

**Recommendation for a US buyer: the GeeekPi board from Amazon.** It is the SLB9670 rather than the
9672, which changes nothing that matters — same `tpm_tis_spi` driver, same `dtoverlay=tpm-slb9670`,
same TPM 2.0 feature set including PCR sealing and monotonic counters. Spend the difference on the
UPS HAT instead.

Prices and stock move; verify before ordering.

#### Setup

```sh
# /boot/firmware/config.txt
dtparam=spi=on
dtoverlay=tpm-slb9670        # correct for 9670 AND 9672 — same SPI TIS interface
```

```sh
sudo apt install tpm2-tools
sudo reboot

ls -l /dev/tpm0 /dev/tpmrm0          # both should exist
sudo tpm2_getcap properties-fixed | grep -i manufacturer
sudo tpm2_pcrread sha256:7,11        # PCRs populated
```

#### Verify before relying on it — Pi 5 caveat

There is a
[reported Pi 5 issue](https://github.com/raspberrypi/linux/issues/6217) where IMA logged
*"No TPM chip found, activating TPM-bypass!"* at boot. That is **early-boot** detection, and
userspace access via `tpm2-tools` was unaffected — but PCR-sealed LUKS unlock happens in the
initramfs, which is exactly the window in question.

So confirm the whole path before trusting it, on a **scratch volume**, not on `PGDATA`:

```sh
sudo truncate -s 64M /tmp/tpmtest.img
sudo cryptsetup luksFormat /tmp/tpmtest.img
sudo systemd-cryptenroll /tmp/tpmtest.img --tpm2-device=auto --tpm2-pcrs=7+11
sudo reboot
# after reboot, confirm it unlocks with no passphrase prompt
sudo /usr/lib/systemd/systemd-cryptsetup attach tpmtest /tmp/tpmtest.img - tpm2-device=auto
```

If unlock works unattended across a reboot, the initramfs path is good and T-C4 can seal the real
volume. **If it does not**, the TPM is still fully useful for §5.1 and §5.2 — non-extractable signing
keys and the monotonic counter, which is the larger half of the value — and disk unlock falls back to
the keyfile approach. Confirm which before committing the design either way.

One more Pi 5 note: PCR 7 measures secure-boot state, which the Pi handles differently from a PC.
If sealing to `7+11` proves flaky, `--tpm2-pcrs=11` alone still binds to the kernel and initramfs
and is a meaningful control.

### ZYMKEY 5 — the one with physical tamper response

Two independent perimeter circuits with configurable policy including **key destruction on breach**,
plus 640 key slots, secure key generation, filesystem encryption, and Pi-specific integration.

Worth it if the device will leave the house, be serviced by someone else, or you want the integrity
story to survive physical access. The cost is proprietary tooling on the boot path.

**These are not exclusive.** A TPM for boot integrity and unlock, plus a ZYMKEY for tamper response,
is a perfectly coherent pairing — the abstraction allows a different backend per job.

---

## 3. Ordering constraint — the one thing that cannot be deferred

**Keys must be born inside the module.** Generating software keys now and "migrating them to the HSM
later" is not possible: the key would have existed in software once, which forfeits non-extraction
permanently and retroactively. Every batch signed before the module arrives is signed by a key that
could have been copied.

This does not mean you must decide now. It means:

- Batches signed under software custody are **recorded as software-custody** in `ingest_batch`, so
  the history is honest about which guarantee applied when.
- Adding a module later starts a new key with a new producer record. Old batches stay verifiable
  against the old key; new ones carry the stronger property. Nothing is lost, nothing is
  retroactively overclaimed.

That is the whole reason `vitals_producer` has `added_at` and `revoked_at` rather than a single
current key.

---

## 4. Physical fit — check before ordering

The Pi 5 will be carrying an M.2 HAT on PCIe and a 7" panel on DSI, inside a case with a tall
heatsink and fan.

- A GPIO module stacks **on top of** the M.2 HAT and needs a stacking/booster header or pass-through.
  Measure clearance first.
- **Bus allocation:** TPM uses SPI (CE0/CE1); ZYMKEY and ATECC use I2C. Neither collides with DSI or
  PCIe, but confirm against the specific M.2 HAT — some route signals through the GPIO header.
- Multi-modal alerting (amendment 03) also wants GPIO for the piezo siren and bed-shaker relay. Plan
  the header allocation once rather than discovering a conflict twice.

**Concrete allocation for this build**, so the three things do not collide:

| Consumer | Pins |
|---|---|
| TPM (SPI0) | 19 MOSI, 21 MISO, 23 SCLK, 24 CE0, 22 reset, plus 3V3 + GND |
| Relay board (siren, shaker, strobe) | any free GPIO — 5, 6, 13, 16 are clear of SPI0 |
| Free | I2C (2, 3) stays open for a future ATECC or ZYMKEY |

The LetsTrust board sits over roughly the first ten pins, so a relay HAT wanting the far end of the
header can coexist. Check it against the M.2 HAT's own stacking height before ordering both.

---

## 5. If you are reading this to decide

- **Already own a TPM, or running on x86?** Use it. The probe will find it.
- **Buying one thing for a Pi 5?** A **LetsTrust TPM** or **ANAVI TPM 2.0** — both Infineon
  SLB9672 on SPI, ~$25. Set `dtparam=spi=on` + `dtoverlay=tpm-slb9670`, then run the scratch-volume
  unlock test in §2 before sealing anything real.
- **Want tamper response?** ZYMKEY 5, alone or alongside a TPM.
- **Want the cheap real improvement?** ATECC608B, ~$5.
- **Want to run it today with nothing extra?** Do that. The system will tell you what you have.
