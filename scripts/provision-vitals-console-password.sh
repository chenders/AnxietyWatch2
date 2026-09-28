#!/usr/bin/env bash
# Provision the vitals-console Argon2id verifier on the Pi.
set -euo pipefail

readonly DEFAULT_ENV_FILE=/etc/vitals/vitals-console.env
readonly HASH_KEY=VITALS_CONSOLE_ADMIN_PASSWORD_HASH

mode=dry-run
restart=false
env_file=$DEFAULT_ENV_FILE

usage() {
  cat >&2 <<'EOF'
Usage: provision-vitals-console-password.sh [--apply] [--restart] [--env-file PATH]

Default: validate the target and print a dry-run plan.
--apply:  read a new password twice, replace only the Argon2id verifier,
          and leave the service stopped/restarted state unchanged.
--restart: with --apply, restart vitals-console and require it to be active.
--env-file PATH: override the target environment file (for tests only).
EOF
}

while (($# > 0)); do
  case "$1" in
    --apply)
      mode=apply
      ;;
    --restart)
      restart=true
      ;;
    --env-file)
      (($# >= 2)) || { echo 'reset blocked: --env-file needs a path' >&2; exit 2; }
      env_file=$2
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "reset blocked: unknown option $1" >&2
      usage
      exit 2
      ;;
  esac
  shift
done

if [[ "$restart" == true && "$mode" != apply ]]; then
  echo 'reset blocked: --restart requires explicit --apply' >&2
  exit 2
fi

[[ -x /usr/bin/argon2 ]] || { echo 'reset blocked: /usr/bin/argon2 is missing' >&2; exit 1; }
[[ -x /usr/bin/openssl ]] || { echo 'reset blocked: /usr/bin/openssl is missing' >&2; exit 1; }
[[ -f "$env_file" ]] || { echo "reset blocked: env file is missing: $env_file" >&2; exit 1; }

grep -q "^${HASH_KEY}=" "$env_file" || {
  echo "reset blocked: $HASH_KEY is absent from $env_file" >&2
  exit 1
}

if [[ "$mode" == dry-run ]]; then
  printf 'dry-run: would replace %s in %s; service restart=%s\n' "$HASH_KEY" "$env_file" "$restart"
  exit 0
fi

if [[ "$EUID" -ne 0 ]]; then
  echo 'reset blocked: --apply must run as root on the target Pi' >&2
  exit 1
fi

umask 077
hash_file=$(mktemp)
env_tmp=$(mktemp)
cleanup() {
  unset new_password confirmation
  rm -f "$hash_file" "$env_tmp"
}
trap cleanup EXIT INT TERM

new_password=''
confirmation=''
read -r -s -p 'New console admin password: ' new_password
printf '\n' >&2
read -r -s -p 'Repeat console admin password: ' confirmation
printf '\n' >&2

if [[ -z "$new_password" ]]; then
  echo 'reset blocked: password is empty' >&2
  exit 1
fi
if [[ "$new_password" != "$confirmation" ]]; then
  echo 'reset blocked: password confirmation did not match' >&2
  exit 1
fi

salt=$(/usr/bin/openssl rand -base64 16)
printf '%s\n' "$new_password" | /usr/bin/argon2 "$salt" -id -e >"$hash_file"
unset new_password confirmation

grep -Eq '^\$argon2id\$' "$hash_file" || {
  echo 'reset blocked: argon2 did not produce an Argon2id verifier' >&2
  exit 1
}

awk -v hash="$(<"$hash_file")" -v key="$HASH_KEY" '
  index($0, key "=") == 1 {
    print key "=" hash
    found = 1
    next
  }
  { print }
  END {
    if (!found) exit 1
  }
' "$env_file" >"$env_tmp" || {
  echo "reset blocked: failed to rewrite $HASH_KEY in $env_file" >&2
  exit 1
}

install -o root -g root -m 0600 "$env_tmp" "$env_file"
rm -f "$hash_file"

if [[ "$restart" == true ]]; then
  systemctl restart vitals-console.service
  systemctl is-active --quiet vitals-console.service || {
    echo 'reset blocked: vitals-console.service did not become active after restart' >&2
    exit 1
  }
  echo 'reset applied: vitals-console.service is active'
else
  echo 'reset applied: restart vitals-console.service separately before testing login'
fi
