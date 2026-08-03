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

| Board | Chip | Verdict |
|---|---|---|
| **ANAVI TPM 2.0 (vertical)** | SLB9672 | **Buy this.** $28 delivered, in stock, open hardware, 10 pins only |
| LetsTrust TPM (pi3g) | SLB9672 | Equally good board; European/APAC shipping |
| Infineon Iridium eval board | SLB9670 | DigiKey / Mouser fallback, ~$64 |
| GeeekPi TPM9670 | SLB9670 *(allegedly)* | **Avoid** — see the marketplace warning below |
| Infineon SLB9673 boards | SLB9673 | **I2C, not SPI.** Kernel ≥ 6.0 supports it but may need enabling at compile time. Skip unless you specifically want SPI free |

**Search terms that work:** `ANAVI TPM 2.0 Raspberry Pi` · `LetsTrust TPM Raspberry Pi` ·
`Infineon OPTIGA TPM Raspberry Pi SPI`

| Route | Part / link | Notes |
|---|---|---|
| **Crowd Supply** | [ANAVI TPM 2.0 **Vertical**](https://www.crowdsupply.com/anavi-technology/anavi-tpm-2-0-for-raspberry-pi) — $20 + $8 US shipping | **Buy this.** In stock, ships in 3 business days. SLB**9672**, OSHWA-certified open hardware, TRNG per NIST SP 800-90A/B. **Uses only 10 header pins (2×5), leaving the rest of GPIO free**, and the vertical variant is stated not to interfere with the Pi 5 Active Cooler. Crowd Supply ships from Portland |
| **DigiKey** | `IRIDIUMSLM9670TPM20TOBO1` (~$64) · `IRIDIUMSLI9670TPM20TOBO1` | Fallback if ANAVI goes out of stock. Infineon's own eval board, older SLB9670, 2× the price |
| **Mouser** | `IRID9670TPM20LINUXTOBO1` | Same board as DigiKey's |
| **LetsTrust direct** | [pi3g](https://pi3g.com/products/industrial/letstrust-tpm/) · [The Pi Hut](https://thepihut.com/products/letstrust-tpm-for-raspberry-pi) (UK) · [Cytron](https://www.cytron.io/p-letstrust-tpm-for-raspberry-pi) (MY) · [BerryBase](https://www.berrybase.de/en/letstrust-tpm-cryptography-module-for-raspberry-pi) (DE) · Core Electronics `CE10600` (AU) | Also SLB9672. European/APAC shipping |
| ~~Amazon marketplace TPM boards~~ | — | **Avoid — see below** |

**Bare chips**, if building your own board: `SLB9670VQ20FW785XTMA1` (Mouser),
`SLB9670VQ20FW760XUMA1` (DigiKey). The Infineon variant letters — **SLB** standard, **SLM** mobile,
**SLI** industrial/automotive — are all fine here.

#### Horizontal or vertical?

ANAVI ships two variants at the same price. The header is 2×5 (2.54 mm) female either way; the
vertical one mounts it at 90°.

**Take the vertical.** Crowd Supply states it does not interfere with the Raspberry Pi 5 Active
Cooler, which is the exact clearance problem a top-mounted M.2 HAT creates. The horizontal variant
is the right pick only if your enclosure is short and wide rather than tall.

It occupies **physical pins 17–26 exactly** (verified on the product page), so the amendment 03
MOSFET board on physical 29/31/33/36 is clear of it. No stacking riser needed — that was an open
item in §4 and is now closed.

#### Do not buy a TPM from an Amazon marketplace seller

The GeeekPi TPM9670 listing (ASIN `B09G2BZQT5`) sits at **3.5 stars with roughly 20% one-star
reviews**, and the complaints are specifically about the one attribute that matters:

- *"FALSE ADVERTISING"*
- *"tpm 1.2 shipped as 2.0"* — with a reviewer reporting *"the first bunch I got all work; the last 4
  I've received were all tpm 1.2 only"*
- Multiple complaints about documentation not specifying which CS pin is used

**TPM 1.2 is not a lesser TPM 2.0, it is a different specification.** No SHA-256 PCR bank, different
command API, and `systemd-cryptenroll --tpm2-device` will not work with it at all. Neither will the
monotonic-counter design in amendment 02 §5.2. Everything this module was being bought for depends on
it genuinely being 2.0.

The batch-to-batch inconsistency is worse than a uniformly bad product: mixed reviews stay mixed, and
you cannot tell from the listing which version you will receive.

**For a security component, provenance is the product.** A part whose entire value is "this key
cannot be extracted" is not something to buy from a marketplace where the chip on the board is not
guaranteed. Buy from Infineon through DigiKey or Mouser, from Crowd Supply, or direct from pi3g —
the ~$40 difference buys a supply chain, which is the thing being paid for.

#### A spare module is not a key backup

Keys are non-extractable *per module* — that is the whole point of the part. A second TPM does not
hold copies of the first one's keys and cannot be made to.

**Consequences, both of which must be designed for:**

**LUKS — always enroll a recovery passphrase alongside the TPM.** If the volume is sealed to the TPM
only and the module fails, the chip desolders, or a firmware update shifts the PCR values, the data
is unrecoverable. Not difficult — gone.

```sh
sudo systemd-cryptenroll /dev/nvme0n1p2 --recovery-key         # FIRST. Store it offline.
sudo systemd-cryptenroll /dev/nvme0n1p2 --tpm2-device=auto --tpm2-pcrs=7+11
```

PCR sealing is *designed* to break when the measured boot state changes, so a kernel or firmware
update will legitimately invalidate the seal. Expect to use the recovery key during ordinary
maintenance, not only in a disaster. `systemd-cryptenroll --wipe-slot=tpm2` then re-enrolling is the
normal post-update flow.

**Signing keys — the spare enrolls as a new producer, and nothing is lost.** Past batches stay
verifiable because the public key is stored in `vitals_producer`. A replacement module generates a
fresh key, gets a new producer row with its own `added_at`, and the old row gets `revoked_at`.
History remains checkable against the key that actually signed it, and new batches carry the new one.
This is exactly why that table is a log rather than a single current key.

#### Verify what you actually received

Regardless of source, confirm the spec version before designing around it:

```sh
sudo tpm2_getcap properties-fixed | grep -A2 TPM2_PT_FAMILY_INDICATOR   # expect "2.0"
sudo tpm2_getcap properties-fixed | grep -i manufacturer                # expect IFX (Infineon)
sudo tpm2_pcrread sha256:0,7,11        # a TPM 1.2 has no SHA-256 bank and this fails
```

If `tpm2_getcap` errors out entirely, or the SHA-256 PCR read fails, you have a 1.2 part. Return it.

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
