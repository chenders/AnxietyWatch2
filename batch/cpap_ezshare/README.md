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

Files stream into exclusive same-directory temporary files, are SHA-256 hashed and fsynced, then linked
to immutable `content/<sha256>` paths without overwriting existing evidence. The containing directory is
fsynced. Handled failures remove partial files.

## CLI

From the repository root:

```sh
python3 -m batch.cpap_ezshare --origin http://pi-host \
  --mirror-dir /var/lib/anxietywatch2/cpap-mirror
```

Run `python3 -m batch.cpap_ezshare --help` for named conservative defaults for connect/read timeout,
listing bytes, file bytes, file count, and traversal depth. Success prints structured JSON effects:
`discovered`, `downloaded`, `replayed`, `rejected`, and `bytes`. Any listing, download, or validation
failure exits nonzero and names the rejected value and reason; origins containing credentials are
rejected without printing those credentials.

## Manifest

`manifest.json` is atomic, compact JSON:

```json
{"entries":[{"card_path":"DATALOG/night.edf","content_path":"content/<sha256>","first_seen":1786017600,"length":1234,"sha256":"<sha256>"}],"generation":1,"version":1}
```

Each append increments `generation`. A valid newer `.manifest.json.pending` left between fsync and rename
is recovered on startup. An unchanged card name and listing size is replayed without another file HTTP
request. A changed size is fetched and preserved as another entry/content object; fetched bytes, not the
name and size, determine immutable content identity. `first_seen` is UTC epoch seconds.
