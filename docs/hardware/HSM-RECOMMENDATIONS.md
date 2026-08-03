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

---

## 5. If you are reading this to decide

- **Already own a TPM, or running on x86?** Use it. The probe will find it.
- **Buying one thing for a Pi?** TPM 2.0 module, ~$25, for the `systemd-cryptenroll` path.
- **Want tamper response?** ZYMKEY 5, alone or alongside a TPM.
- **Want the cheap real improvement?** ATECC608B, ~$5.
- **Want to run it today with nothing extra?** Do that. The system will tell you what you have.
