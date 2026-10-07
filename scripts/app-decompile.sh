#!/usr/bin/env bash
# app-decompile.sh — obtain a candidate's artifact and read it, writing an inventory for the researcher.
#
#   app-decompile.sh android <slug> [--apk PATH | --package PKG]
#   app-decompile.sh ios     <slug> [--ipa PATH | --bundle-id ID]
#
# Everything lands in $AW2_RESEARCH_DIR/<slug>/ (default ~/aw2research/<slug>/), which is scratch and is
# never committed. The researcher reads the inventory and writes conclusions into the candidate's
# IMPLEMENTATION_INSPECTION.md, with the digests and tool versions copied from here.
#
# A missing tool is reported and skipped, not fatal: partial coverage with the mechanism named beats no
# coverage (PROMPT1.md section 2). Failing to obtain the artifact at all is fatal, with the reason.
set -Eeuo pipefail

RESEARCH_DIR="${AW2_RESEARCH_DIR:-$HOME/aw2research}"
PLATFORM=${1:-}
SLUG=${2:-}
[[ -n "$PLATFORM" && -n "$SLUG" ]] || { sed -n '2,10p' "$0" >&2; exit 2; }
shift 2

APK=""; PKG=""; IPA=""; BUNDLE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --apk)       APK=${2:-};    shift 2 ;;
    --package)   PKG=${2:-};    shift 2 ;;
    --ipa)       IPA=${2:-};    shift 2 ;;
    --bundle-id) BUNDLE=${2:-}; shift 2 ;;
    *) echo "app-decompile: unknown argument '$1'" >&2; exit 2 ;;
  esac
done

DIR="$RESEARCH_DIR/$SLUG"
ART="$DIR/artifact"
INV="$DIR/INVENTORY.md"
mkdir -p "$ART"

have() { command -v "$1" >/dev/null 2>&1; }
ver()  { local t=$1; if have "$t"; then printf '%s' "$("$t" --version 2>&1 | head -1)"; else printf 'NOT INSTALLED'; fi; }
say()  { printf '%s\n' "$*" | tee -a "$INV" >/dev/null; }

# Grep targets that identify capability, endpoints, third parties and gates. Extend freely.
PATTERNS=(
  'https?://[^"'"'"' )>]+'
  'api[_-]?key|client[_-]?secret|Bearer [A-Za-z0-9._-]+'
  '\b(sqlite|SharedPreferences|EncryptedSharedPreferences|Keystore|Realm|coredata)\b'
  '\b(WorkManager|AlarmManager|foregroundService|BOOT_COMPLETED|BODY_SENSORS|HealthConnect|GATT)\b'
  '\b(Firebase|GraphQL|Sentry|Amplitude|Mixpanel|Segment|Braze|AppsFlyer|Adjust|Meta|Facebook|GAID|ADID)\b'
  '\b(subscription|entitlement|sku|purchase|trial|paywall|premium)\b'
  '\b(HKQuantityTypeIdentifier[A-Za-z]+|CBUUID|0000[0-9A-Fa-f]{4}-0000-1000-8000-00805F9B34FB)\b'
)

inventory_header() {
  {
    printf '# %s — artifact inventory\n\n' "$SLUG"
    printf 'Generated: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'Scratch dir: %s (not committed)\n\n' "$DIR"
    printf '## Tool versions\n\n'
    printf -- '- bash: %s\n' "${BASH_VERSION}"
  } > "$INV"
}

scan_text() {  # scan_text <label> <file>
  local label=$1 file=$2
  [[ -f "$file" ]] || return 0
  say "### Grep: $label"
  say ''
  for p in "${PATTERNS[@]}"; do
    local hits
    hits=$(grep -Eoh "$p" "$file" 2>/dev/null | sort | uniq -c | sort -rn | head -25 || true)
    [[ -n "$hits" ]] || continue
    say "Pattern: \`$p\`"
    say '```'
    printf '%s\n' "$hits" | tee -a "$INV" >/dev/null
    say '```'
  done
  say ''
}

# ------------------------------------------------------------------ android
do_android() {
  local selected=""
  inventory_header
  say '## Platform: Android'

  if [[ -n "$APK" ]]; then
    [[ -f "$APK" ]] || { echo "app-decompile: APK not found: $APK" >&2; exit 1; }
    cp -f "$APK" "$ART/" && selected="$ART/$(basename "$APK")"
  elif [[ -n "$PKG" ]] && have adb && adb get-state >/dev/null 2>&1; then
    local remote
    remote=$(adb shell pm path "$PKG" 2>/dev/null | tr -d '\r' | sed -n 's/^package://p')
    [[ -n "$remote" ]] || { echo "app-decompile: '$PKG' is not installed on the attached device" >&2; exit 1; }
    while IFS= read -r r; do
      [[ -n "$r" ]] && adb pull "$r" "$ART/" >/dev/null
    done <<< "$remote"
    selected=$(ls -1 "$ART"/base.apk 2>/dev/null | head -1)
    [[ -n "$selected" ]] || selected=$(ls -1 "$ART"/*.apk 2>/dev/null | head -1)
    adb shell dumpsys package "$PKG" > "$DIR/dumpsys.txt" 2>/dev/null || true
  elif [[ -n "$PKG" ]]; then
    echo "app-decompile: no --apk given and no usable adb device for '$PKG'." >&2
    echo "  Provide the published artifact: app-decompile.sh android $SLUG --apk /path/to/base.apk" >&2
    exit 1
  else
    selected=$(ls -1 "$ART"/*.apk 2>/dev/null | head -1)
    [[ -n "$selected" ]] || { echo "app-decompile: no APK. Pass --apk PATH or --package PKG (with a device)." >&2; exit 1; }
  fi

  say ''
  say '## Artifact'
  say ''
  for f in "$ART"/*.apk; do
    [[ -f "$f" ]] || continue
    say "- \`$(basename "$f")\` — $(wc -c < "$f" | tr -d ' ') bytes"
    say "  - sha256 \`$(shasum -a 256 "$f" | awk '{print $1}')\`"
  done

  say ''
  say '## Tool versions'
  say ''
  for t in jadx apktool apkanalyzer aapt2 apksigner java; do say "- $t: $(ver "$t")"; done

  if have apksigner && [[ -f "$selected" ]]; then
    say ''
    say '## Signing'
    say '```'
    apksigner verify --print-certs "$selected" 2>&1 | tee -a "$INV" >/dev/null || true
    say '```'
  fi

  local dec="$DIR/decompiled" apt="$DIR/apktool"
  if have jadx; then
    say ''
    say "## jadx -> $dec"
    jadx --no-inline-anonymous -d "$dec" "$selected" >"$DIR/jadx.log" 2>&1 \
      && say "ok: $(find "$dec" -name '*.java' 2>/dev/null | wc -l | tr -d ' ') java files" \
      || say "jadx failed (see $DIR/jadx.log)"
  else
    say ''
    say '## jadx: NOT INSTALLED — install with `brew install jadx` for the Java/Kotlin layer'
  fi
  if have apktool; then
    say ''
    say "## apktool -> $apt"
    apktool d -f -o "$apt" "$selected" >"$DIR/apktool.log" 2>&1 \
      && say "ok: manifest + resources extracted" \
      || say "apktool failed (see $DIR/apktool.log)"
  else
    say ''
    say '## apktool: NOT INSTALLED — install with `brew install apktool` for manifest/resources'
  fi

  # Flatten the readable text of the decompiled tree for grepping without loading it all into context.
  local flat="$DIR/text.txt"
  : > "$flat"
  for d in "$dec" "$apt"; do
    [[ -d "$d" ]] || continue
    find "$d" -type f \( -name '*.java' -o -name '*.kt' -o -name '*.xml' -o -name '*.json' \
      -o -name '*.properties' -o -name '*.smali' -o -name '*.txt' -o -name '*.html' -o -name '*.js' \) \
      -print0 2>/dev/null | xargs -0 cat >> "$flat" 2>/dev/null || true
  done
  say ''
  say "## Flattened text: \`$flat\` ($(wc -c < "$flat" | tr -d ' ') bytes)"
  scan_text "decompiled tree" "$flat"
  [[ -f "$DIR/dumpsys.txt" ]] && scan_text "dumpsys package" "$DIR/dumpsys.txt"

  say ''
  say '## Next'
  say ''
  say '1. Read the manifest/permissions sections above out of `apktool/AndroidManifest.xml`.'
  say '2. Read the package and SDK inventory from `decompiled/` and `apktool/res/`.'
  say '3. Mark every conclusion `[STATIC]` unless it was also observed running.'
  say '4. Write the findings into the candidate IMPLEMENTATION_INSPECTION.md, copying the digests and'
  say '   tool versions from this file.'
  echo "app-decompile: inventory written to $INV"
}

# ---------------------------------------------------------------------- ios
do_ios() {
  inventory_header
  say '## Platform: iOS'

  if [[ -z "$IPA" ]]; then
    IPA=$(ls -1 "$ART"/*.ipa 2>/dev/null | head -1)
  fi
  if [[ -z "$IPA" && -n "$BUNDLE" ]]; then
    if have ipatool; then
      echo "app-decompile: fetching $BUNDLE with ipatool (uses your own Apple ID)"
      ( cd "$ART" && ipatool download --bundle-identifier "$BUNDLE" --output "$ART/" ) \
        || { echo "app-decompile: ipatool download failed for '$BUNDLE'" >&2; exit 1; }
      IPA=$(ls -1 "$ART"/*.ipa 2>/dev/null | head -1)
    else
      echo "app-decompile: ipatool not installed; cannot fetch '$BUNDLE'." >&2
      echo "  brew install ipatool   (or pass --ipa /path/to/app.ipa)" >&2
      exit 1
    fi
  fi
  [[ -n "$IPA" && -f "$IPA" ]] || { echo "app-decompile: no .ipa. Pass --ipa PATH or --bundle-id ID (with ipatool)." >&2; exit 1; }

  say ''
  say '## Artifact'
  say ''
  say "- \`$(basename "$IPA")\` — $(wc -c < "$IPA" | tr -d ' ') bytes"
  say "  - sha256 \`$(shasum -a 256 "$IPA" | awk '{print $1}')\`"

  local ex="$DIR/ipa"
  rm -rf "$ex"; mkdir -p "$ex"
  unzip -q "$IPA" -d "$ex" || { echo "app-decompile: unzip failed" >&2; exit 1; }
  local app
  app=$(ls -1d "$ex"/Payload/*.app 2>/dev/null | head -1)
  [[ -n "$app" ]] || { echo "app-decompile: no Payload/*.app inside $IPA" >&2; exit 1; }
  local bin; bin=$(find "$app" -maxdepth 1 -type f -perm -u+x 2>/dev/null | head -1)

  say ''
  say "## App bundle: \`$(basename "$app")\`"
  say ''
  say "- files: $(find "$app" -type f 2>/dev/null | wc -l | tr -d ' ')"
  say "- size: $(du -sh "$app" 2>/dev/null | awk '{print $1}')"

  say ''
  say '## Tool versions'
  say ''
  for t in plutil codesign otool nm strings class-dump frida; do say "- $t: $(ver "$t")"; done

  local info="$DIR/info.plist.txt"
  if have plutil; then
    plutil -p "$app/Info.plist" > "$info" 2>/dev/null || true
    say ''
    say "## Info.plist -> \`$info\`"
    say ''
    say '```'
    cat "$info" | tee -a "$INV" >/dev/null
    say '```'
  fi

  if have codesign; then
    say ''
    say '## Entitlements'
    say '```'
    codesign -d --entitlements - "$app" 2>&1 | tee -a "$INV" >/dev/null || true
    say '```'
  fi

  if [[ -n "$bin" ]]; then
    if have otool; then
      say ''
      say '## Linked frameworks'
      say '```'
      otool -L "$bin" 2>&1 | tee -a "$INV" >/dev/null || true
      say '```'
    fi
    if have strings; then
      strings -a "$bin" 2>/dev/null | sort -u > "$DIR/strings.txt" || true
      say ''
      say "## Strings: \`$DIR/strings.txt\` ($(wc -l < "$DIR/strings.txt" 2>/dev/null | tr -d ' ') unique)"
      say ''
      say 'Strings are thin on an App Store binary: code sections are FairPlay-encrypted unless it is'
      say 'decrypted on a capable device. That is a *technical limit* — name it, and list what was read'
      say 'instead (bundles, resources, plists, entitlements, framework names, served web assets).'
      scan_text "binary strings" "$DIR/strings.txt"
    fi
  fi

  # Flatten everything texty inside the bundle for grepping.
  local flat="$DIR/text.txt"
  : > "$flat"
  find "$app" -type f \( -name '*.plist' -o -name '*.json' -o -name '*.strings' -o -name '*.stringsdict' \
    -o -name '*.js' -o -name '*.html' -o -name '*.css' -o -name '*.txt' -o -name '*.xml' \) \
    -print0 2>/dev/null | xargs -0 cat >> "$flat" 2>/dev/null || true
  cmp -s "$flat" "$DIR/strings.txt" 2>/dev/null || true
  say ''
  say "## Flattened bundle text: \`$flat\` ($(wc -c < "$flat" | tr -d ' ') bytes)"
  scan_text "app bundle resources" "$flat"

  say ''
  say '## Next'
  say ''
  say '1. Read Info.plist URL schemes, background modes, ATS exceptions and the entitlement list.'
  say '2. Read every bundled resource you can (config, localisation, assets, storyboards, JSON).'
  say '3. Record the FairPlay limit as a technical limit with what was read instead, if it applies.'
  say '4. Write the findings into the candidate IMPLEMENTATION_INSPECTION.md with the digests above.'
  echo "app-decompile: inventory written to $INV"
}

case "$PLATFORM" in
  android) do_android ;;
  ios)     do_ios ;;
  *) echo "app-decompile: platform must be 'android' or 'ios', got '$PLATFORM'" >&2; exit 2 ;;
esac
