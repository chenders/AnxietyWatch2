# ez Share safe mirror

This standard-library Python batch package mirrors bytes from the documented ez Share SD-card HTTP
interface. It **does not parse EDF, infer physiology, edit the registry, or write physiological data or
SQL**.

## Trust boundary

Everything received from the card is untrusted: HTML, links, names, sizes, redirects, headers, and file
bytes. The mirror accepts one fixed `http://` host and port, encodes query parameters, parses HTML as
data, confines normalized POSIX paths, and applies finite timeout/size/count/depth limits. Same-origin
redirects are bounded; cross-origin redirects fail. Plain HTTP provides no confidentiality or server
authentication and must later be confined to the dedicated card network (outside T-CP1).

Two properties do most of the work, and it is worth being precise about which is which:

- **No card-supplied string ever becomes a filesystem path.** Every write goes to a `mkstemp` name or
  to `content/<sha256>` — 64 lowercase hex characters. Encoded, double-encoded, overlong-UTF-8,
  unicode-normalising and case-folding traversal attempts are therefore inert *by construction*, not
  by validation. Card paths are still validated, but only so the evidence record is meaningful.
- **Individual limits are not run limits.** Bounding one file, one listing and the file count still
  permits an unbounded walk: a card offering ten legal subdirectories per listing at the depth limit
  describes about 10^8 legal directories. So the run itself is bounded as well — see below.

Files stream into exclusive same-directory temporary files, are SHA-256 hashed and fsynced, then linked
to immutable `content/<sha256>` paths without overwriting existing evidence. The containing directory is
fsynced. Handled failures remove partial files. Pre-existing content at a target path is accepted only
after its size *and* hash are re-verified — including when the link loses a race — so a file planted at
a content path can never be mistaken for downloaded evidence.

The mirror directory and `content/` are forced to mode `0700` even when they already existed. This
store holds PHI.

## Run bounds

`--max-file-bytes`, `--max-listing-bytes` and `--max-files` bound individual objects. These bound the
run:

| Flag | Default | What it stops |
|---|---|---|
| `--max-directories` | 512 | a wide or cyclic tree of individually-legal listings |
| `--max-requests` | 2048 | a request storm that exhausts local ephemeral ports |
| `--max-total-bytes` | 2 GiB | `max-files × max-file-bytes` is 31.25 GiB, which is not conservative on a Pi that also runs the alarm |
| `--run-timeout` | 900 s | a card trickling bytes just inside the per-read timeout, which bounds one `recv` and not the body |
| `--min-free-bytes` | 1 GiB | filling the disk; content is immutable and never pruned, so this is checked per download |

An over-depth directory is **pruned and reported**, not fatal: aborting the run let one hostile deep
name deny mirroring for every other file on the card. Exhausting a run budget *is* fatal.

Only one run per mirror directory at a time, enforced by an exclusive `.mirror.lock`. Concurrent runs
previously took a lost update on the manifest — both read the same generation, both appended, and one
run's recorded evidence disappeared while the generation did not advance, so nothing downstream could
detect the loss. If a run is killed, the lock must be removed by the operator; that is deliberate,
because a killed run is exactly the case where the manifest state wants a human look.

## CLI

From the repository root:

```sh
python3 -m batch.cpap_ezshare --origin "http://$EZSHARE_CARD_IP" \
  --mirror-dir /var/lib/anxietywatch2/cpap-mirror
```

`CARD-IP` is the card's access-point address, recorded once in
`docs/integration/hms-cpap-ezshare-integration.md`. It is not repeated here: the public-information
gate rejects a private-network URL in tracked source, and it is right to — the address belongs in
the one integration document, not scattered through code and examples.

Run `python3 -m batch.cpap_ezshare --help` for every limit and its named conservative default.

Output is structured JSON effects — `discovered`, `downloaded`, `replayed`, `rejected`, `bytes`, and
`blockers`, where each rejection names itself. Those counts are always what was *measured*: a run that
committed four files and then hit a budget reports `downloaded: 4`, never zero.

Exit status:

- `0` — the run completed and rejected nothing.
- `1` — a listing, download or validation failure, **or** any rejection. A partially mirrored card is
  not a success.

Origins containing credentials are rejected without printing them, and query *values* are redacted from
all error text (keys are kept, which is enough to name the rejected parameter). The ez Share convention
passes its password as a query parameter, so this matters before T-CP5 puts one there.

A `Content-Encoding` response is rejected rather than decoded: stored evidence must be the file's own
bytes, not a transport encoding of them.

## Manifest

`manifest.json` is atomic, compact JSON:

```json
{"entries":[{"card_path":"DATALOG/night.edf","content_path":"content/<sha256>","first_seen":1786017600,"length":1234,"sha256":"<sha256>"}],"generation":1,"version":1}
```

Each append increments `generation`. A valid newer `.manifest.json.pending` left between fsync and
rename is recovered on startup; every entry is shape-validated on load, so a malformed manifest names
its blocker instead of failing somewhere deeper. `first_seen` is UTC epoch seconds.

### Replay, and one residual risk worth stating plainly

An unchanged card name and listing size is replayed without another file HTTP request. A changed size
is fetched and preserved as an additional entry and content object; old evidence is never replaced,
truncated or deleted.

The task requires both "a replay of unchanged name+size must make no HTTP file download" and "do not
trust name+size as the ultimate identity once bytes are fetched". Those two pull against each other,
and the resolution here is deliberate:

- Once bytes **are** fetched, identity is the SHA-256 and nothing else. Content is addressed by hash,
  never deduplicated or overwritten by name and size.
- Skipping the fetch when name and size are unchanged is a bandwidth decision, and it carries a real
  consequence: **a file mutated in place without changing its length is not detected.** The mirror
  cannot know what it did not fetch.

`--verify-replayed` is the opt-in resolution. It re-fetches entries already recorded under the same
name and size, compares hashes, and reports a changed body as a named rejection while retaining both
evidences. It is off by default because it re-downloads the whole card on every run. For AirSense data
the default is reasonable — sessions arrive as new files and `STR.edf` changes length — but a
correctness-critical import should schedule a periodic `--verify-replayed` pass rather than assume
name and size are sufficient.
