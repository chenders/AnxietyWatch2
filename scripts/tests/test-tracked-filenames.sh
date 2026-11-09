#!/usr/bin/env bash
# No tracked path may contain a character that is ambiguous to a shell.
#
# Two zero-byte files named `[^` and `]*` were committed in 79201d3. They are
# the two halves of the character class `[^ ]*`, split on the space by shell
# word-splitting and then materialised as files:
#
#     PATTERN='[^ ]*'
#     touch $PATTERN        # unquoted -> two words -> two empty files
#
# Quoted it is one argument. Unquoted the shell splits it before anything sees
# it. That failure is silent in every direction: zero-byte files produce no
# error, no output and nothing to notice, and `git add -A` then sweeps them in
# and the commit looks clean. AGENTS.md already forbids `git add -A` for this
# reason; this gate is the backstop for when that rule is broken.
#
# This is an ALLOWLIST, not a list of banned characters. A denylist has to
# anticipate every hazard, and the first draft of this file proved why: it
# declared `~` and `#` forbidden in a variable the check never consulted, so
# `tilde~` passed the gate that was written to stop it. An allowlist fails
# closed on anything nobody thought of.
#
# Calibration, measured rather than guessed: of 904 tracked paths at the time
# this was written, exactly two fell outside the safe set and both were
# accidents. Fourteen contain spaces -- design artifacts like
# `A - Clinical.dc.html` -- which are deliberate, so space is in the set. The
# gate rejects the pathology and nothing in use.
set -Eeuo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"

# The only characters a tracked path may contain. Widen this deliberately, with
# a measurement showing nothing legitimate is being excluded.
readonly SAFE_SET='A-Za-z0-9._/ -'
failures=0

report() {
    printf 'tracked-filenames FAIL: %s\n' "$1" >&2
    failures=$((failures + 1))
}

while IFS= read -r -d '' path; do
    # Anything left after removing every safe character is a violation. This
    # also catches control characters, since they are not in the set.
    unsafe=${path//[$SAFE_SET]/}
    if [[ -n $unsafe ]]; then
        report "unsafe character(s) ${unsafe@Q} in tracked path: ${path@Q}"
        continue
    fi
    # A leading dash on any component reads as an option to most tools.
    case "/$path" in
        */-*) report "path component starts with '-', which reads as an option: ${path@Q}" ;;
    esac
    # A trailing space or dot is invisible, and illegal on Windows.
    case "$path" in
        *' ' | *. | *' /'* | *./*)
            report "path component ends in a space or dot: ${path@Q}"
            ;;
    esac
done < <(git ls-files -z)

if (( failures )); then
    cat >&2 <<'HINT'

These names are almost always an unquoted expansion, not a deliberate choice.
Quote the variable:   touch "$PATTERN"   not   touch $PATTERN
Then remove the file: git rm -- '<name>'
Stage by name rather than with `git add -A`, per AGENTS.md.
HINT
    printf 'tracked-filenames: %d violation(s)\n' "$failures" >&2
    exit 1
fi

printf 'tracked-filenames: all %d tracked paths are shell-safe\n' \
    "$(git ls-files | wc -l | tr -d ' ')"
