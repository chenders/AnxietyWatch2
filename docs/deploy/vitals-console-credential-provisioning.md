# Vitals console admin credential provisioning

This document records what is actually available for the AnxietyWatch2 console admin password and what is still an operational decision.

## Current facts

- The console uses one operator password verified with Argon2id.
- The Pi stores only `VITALS_CONSOLE_ADMIN_PASSWORD_HASH` in the root-only file `/etc/vitals/vitals-console.env`.
- The plaintext password is not recoverable from the Pi, the database, the service journal, or this repository.
- There is no separate phone-enrollment password. Phone enrollment reuses the console admin password and requires recent reauthentication.
- The Pi currently has `/usr/bin/argon2` and `openssl` installed.
- The Pi does not currently have the 1Password `op` CLI installed.
- The repository contains no password-generation or password-capture helper.

Therefore, the existing password must be retrieved from the authorized password manager or from the authorized operator who provisioned it. If that entry is unavailable, the password must be rotated; it cannot be found by inspecting the Argon2id verifier.

## What is and is not approved

There is currently **no documented, project-approved password-reset command or capture tool**. `/usr/bin/argon2` is an available hashing utility, not automatically an approved operational workflow.

The reset procedure below is the proposed procedure to approve before use. It has not been run by this change and it must be performed only by an authorized operator from a trusted terminal.

## Proposed authorized reset procedure

The repository helper `scripts/provision-vitals-console-password.sh` packages the procedure below. It is dry-run by default and has two explicit safety gates: `--apply` is required to change the verifier, and `--restart` is separately required to restart the service.

After owner approval, copy the helper to the target Pi and inspect the dry-run first:

```sh
scp scripts/provision-vitals-console-password.sh <pi-alias>:/tmp/vitals-console-password-reset.sh
ssh <pi-alias> 'sudo install -o root -g root -m 0755 /tmp/vitals-console-password-reset.sh /usr/local/sbin/vitals-console-password-reset && rm -f /tmp/vitals-console-password-reset.sh'
ssh <pi-alias> 'sudo /usr/local/sbin/vitals-console-password-reset'
```

Only after the workflow and password-manager destination are approved should the authorized operator run:

```sh
ssh -t <pi-alias> 'sudo /usr/local/sbin/vitals-console-password-reset --apply --restart'
```

The script was syntax-checked and its dry-run was tested with a fake environment file. Neither `--apply` nor `--restart` has been run against the live Pi.

### 1. Prepare a password-manager entry first

Create or open the dedicated console-admin entry in the approved password manager. Generate a strong unique password there, or prepare a strong value for entry. Do not paste it into chat or a ticket.

### 2. Generate an Argon2id verifier interactively

The script performs this step interactively when invoked with `--apply`. The manual mechanics are shown here to make the security properties auditable:

Run this from a trusted terminal on the Pi or an approved administration workstation with access to the Pi. Do not put the password in the command line, shell history, environment, or a regular file.

```sh
set -eu
umask 077
hash_file=$(mktemp)
new_password=''
confirmation=''
cleanup() {
  unset new_password confirmation
  rm -f "$hash_file"
}
trap cleanup EXIT INT TERM

read -r -s -p 'New console admin password: ' new_password
printf '\n' >&2
read -r -s -p 'Repeat console admin password: ' confirmation
printf '\n' >&2

if [ "$new_password" != "$confirmation" ]; then
  echo 'reset blocked: password confirmation did not match' >&2
  exit 1
fi

salt=$(openssl rand -base64 16)
printf '%s\n' "$new_password" | /usr/bin/argon2 "$salt" -id -e >"$hash_file"
unset new_password confirmation

grep -Eq '^\$argon2id\$' "$hash_file"
```

The resulting file contains a verifier, not the plaintext password. Keep it temporary and remove it after the configuration update. The exact Argon2 memory/time parameters must be approved by the operator responsible for the host; the command above uses the installed utility's defaults.

### 3. Replace only the verifier atomically

The script performs this step after successful interactive confirmation. The manual mechanics are shown here to make the target and file-mode requirements auditable:

Review the target path and preserve every unrelated environment setting. The following example updates only the `VITALS_CONSOLE_ADMIN_PASSWORD_HASH` line and installs the result root-owned with mode `0600`:

```sh
set -eu
source_env=/etc/vitals/vitals-console.env
replacement=$(mktemp)
trap 'rm -f "$replacement"' EXIT INT TERM

sudo awk -v hash="$(cat "$hash_file")" '
  /^VITALS_CONSOLE_ADMIN_PASSWORD_HASH=/ {
    print "VITALS_CONSOLE_ADMIN_PASSWORD_HASH=" hash
    found = 1
    next
  }
  { print }
  END {
    if (!found) print "VITALS_CONSOLE_ADMIN_PASSWORD_HASH=" hash
  }
' "$source_env" >"$replacement"

sudo install -o root -g root -m 0600 "$replacement" "$source_env"
rm -f "$hash_file"
```

The `hash_file` reference above must point to the temporary verifier created in step 2. Do not substitute the plaintext password for it.

### 4. Restart and verify without exposing the password

The script performs the restart only when `--restart` is explicitly supplied with `--apply`. For a manual verification, use:

```sh
sudo systemctl restart vitals-console.service
sudo systemctl is-active --quiet vitals-console.service
sudo journalctl -u vitals-console.service -n 30 --no-pager
```

The journal check is for startup errors only. Do not log in with the password in a scripted request and do not place it in a command argument.

### 5. Store and test the new password

Save the new password in the approved password manager, then test it manually at the console login page. After successful login, enroll the phone using [the iPhone enrollment runbook](iphone-enrollment-runbook.md).

A service restart invalidates in-memory console sessions. The phone enrollment artifact must still be newly generated after the restart.

## Required approval before this becomes standard

Before treating this as the project’s approved method, an owner must explicitly approve:

1. `/usr/bin/argon2` as the hashing utility;
2. the Argon2id memory/time/parallelism parameters;
3. the interactive capture and temporary-verifier handling above;
4. the approved password-manager destination;
5. the operator and recovery procedure if the restart or login verification fails.

Until that approval exists, the only established method is retrieving the existing password from the approved password manager or asking the authorized provisioning operator.
