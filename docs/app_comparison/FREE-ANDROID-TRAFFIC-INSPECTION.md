# Free Android and iOS inspection toolchain

Everything here is free software or free of charge. The workflow is **decompile first, observe second**:
the decompiled artifact explains more about what an app sends, stores and gates than any captured session,
and it works when interception does not.

```sh
scripts/compare-one-app.sh lab <slug>     # brings the lab up, runs it, and tears it down
scripts/app-decompile.sh android <slug>   # pull or fetch the APK, decompile it, write an inventory
scripts/app-decompile.sh ios     <slug>   # unpack the .ipa, read everything readable, write an inventory
```

---

## 1. Tools

| Tool | Role | Install |
|---|---|---|
| `adb` + Android Emulator | Controlled device, package extraction, traffic capture point | Android SDK command-line tools + `emulator` |
| `jadx` / `jadx-gui` | Java/Kotlin decompilation, the fastest route to endpoints, SDKs, keys and gates | `brew install jadx` |
| `apktool` | Manifest, resources, `res/`, smali round-trip | `brew install apktool` |
| `baksmali` / `apkanalyzer` | Smali disassembly; APK composition and method counts | Android SDK / `brew` |
| `mitmproxy` / `mitmdump` | HTTPS interception, flow export, addon scripting | `brew install mitmproxy` |
| `tcpdump` / `tshark` | Packet-capture fallback when HTTPS cannot be intercepted | `brew install wireshark` |
| `ipatool` | Fetch an `.ipa` from the App Store with your own Apple ID | `brew install ipatool` |
| `otool`, `nm`, `strings`, `plutil`, `class-dump` | Mach-O metadata, symbols, plists, Objective-C classes | Xcode command-line tools |
| `frida` | Runtime instrumentation, when a device plus a readable target allows it | `brew install frida` |

Record every version actually used in the candidate's inspection file. A tool version is part of the
method.

---

## 2. Android: obtain the artifact

Two routes; do both when they disagree, because a store build and a device build can differ.

```sh
# Route A — the installed package on a device or emulator you control
adb shell pm list packages | grep <name-fragment>
adb shell pm path <package.id>                 # base.apk plus any split APKs
adb pull /data/app/.../base.apk ./<slug>/artifact/
adb shell dumpsys package <package.id> > ./<slug>/artifact/dumpsys.txt   # version, installer, perms

# Route B — the published artifact
# fetch the APK from a public mirror (APKMirror/APKPure) or, for a direct-distribution app, from the
# vendor's own download page; record the URL, byte size and SHA-256 of what you got
shasum -a 256 ./<slug>/artifact/*.apk
```

Record: package id (case-sensitive), versionName/versionCode, signing certificate fingerprint
(`apksigner verify --print-certs`), install source, and the SHA-256 of each artifact byte.

## 3. Android: read it

```sh
jadx --no-inline-anonymous -d ./<slug>/decompiled  ./<slug>/artifact/base.apk
apktool d -f -o ./<slug>/apktool ./<slug>/artifact/base.apk
```

Then walk the inventory in `PROMPT1.md` §2: `AndroidManifest.xml` (permissions, exported components,
background modes, deep links, `queries`), `res/xml/*` (network security config, data-extraction rules,
backup rules, accessibility), bundled assets and JSON config, feature-flag defaults, `BuildConfig` and
resource strings, third-party SDKs (analytics, advertising, crash, attribution, payments), endpoint
constants, auth scheme, storage keys, and every place a value is written to disk.

**Grep targets that have paid off repeatedly:** `https?://`, `api[_-]?key`, `client_secret`,
`Bearer `, `Firebase`, `GraphQL`, `sqlite`, `.db`, `SharedPreferences`, `EncryptedSharedPreferences`,
`Keystore`, `WorkManager`, `AlarmManager`, `foregroundService`, `BOOT_COMPLETED`, `POST_NOTIFICATIONS`,
`BODY_SENSORS`, `ACCESS_FINE_LOCATION`, `HealthConnect`, `GATT` UUIDs, `experiment`/`flag` tables,
`subscription`/`sku`/`entitlement`, and any string that looks like a clinical or scoring label.

**Play Integrity / attestation.** A modified or emulated client may be rejected. That is a technical
limit: the flows behind it are read statically, and the finding is recorded as `[STATIC]`.

## 4. iOS: obtain and read

```sh
ipatool auth login                 # your own Apple ID
ipatool download --bundle-identifier <bundle.id> --output ./<slug>/artifact/
unzip -q ./<slug>/artifact/*.ipa -d ./<slug>/ipa
plutil -p ./<slug>/ipa/Payload/*.app/Info.plist
codesign -d --entitlements - ./<slug>/ipa/Payload/*.app
otool -L ./<slug>/ipa/Payload/*.app/<binary>       # linked frameworks
strings -a ./<slug>/ipa/Payload/*.app/<binary> | sort -u > ./<slug>/strings.txt
```

Read what is readable: `Info.plist` (URL schemes, background modes, ATS exceptions, entitlements),
`Frameworks/` (which SDKs ship), asset catalogs, storyboards and XIBs, localization tables, bundled JSON
and plists, the app's own configuration, and anything the app serves to its web view.

**Encryption is the one common technical limit.** App Store binaries have FairPlay-encrypted code
sections, so the `strings` output and symbol tables are thin unless the binary is decrypted on a device
that can do it. When that is the case, say so with the mechanism named and list everything that *was*
read — bundles, resources, plists, entitlements, framework names and the served web assets are
frequently enough to answer the question. Do not write "could not inspect" and stop.

## 5. Runtime observation

Only if a device or emulator path exists. The lab is disposable: one emulator, one snapshot, torn down
afterwards, no other data on it.

1. **Start the emulator and the proxy before the app ever launches.** A first launch that reaches the
   network without a proxy in place will not repeat its first-run requests under capture.

   ```sh
   mitmweb --listen-host 0.0.0.0 --listen-port 8080 --set confdir=./<slug>/mitm
   emulator -avd <avd> -no-snapshot-save -writable-system
   adb shell settings put global http_proxy 10.0.2.2:8080
   adb shell settings get global http_proxy        # verify before installing anything
   ```

2. **Install the proxy CA in the disposable emulator only** (Android 14+ treats user CAs as untrusted for
   app traffic, so a `--writable-system` emulator with the CA in the system store is usually required).
   Never install a proxy CA on a personal or production device; never disable certificate pinning on one.
3. **Install the target, then launch it.** Capture flows (`mitmdump -w flows.mitm`), and keep a parallel
   `tshark` capture for anything the proxy cannot see.
4. **Use synthetic data only.** No real user data, no other person's account, no production credentials.
5. **Tear down:** kill the emulator, remove the AVD or revert its snapshot, and record what was captured.

**Certificate pinning.** If the client pins, interception fails and that is a technical limit. Do not
patch the app to defeat it on a device; take the endpoint contract from the decompiled code instead, mark
it `[STATIC]`, and record the pinning as the reason runtime observation is partial.

**When the proxy cannot see the traffic,** fall back to destination-level evidence: `tshark` for hostnames
and SNI, the decompiled endpoint list, and the vendor's own documentation. Destination-level evidence
answers "what does it talk to", which is usually the question that matters.

---

## 6. Reproducibility record

Every inspection file states: artifact identifier and SHA-256, tool versions, device/emulator image,
the exact commands run (or the script and its arguments), retrieval dates and source URLs, and the
coverage limits with their mechanisms. A reader with the same artifact and the same tools must be able to
reach the same findings.
