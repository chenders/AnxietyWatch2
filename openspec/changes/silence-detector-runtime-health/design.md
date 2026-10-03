# Design

## Database connection

The service uses a SQLx PostgreSQL URL with the local PostgreSQL Unix socket. The binary rejects
libpq keyword strings before attempting a connection, so a malformed deployment fails closed with a
configuration error rather than entering an unbounded restart storm.

## Service evidence

`vitals-deploy snapshot` obtains `ActiveState`, `SubState`, `Result`, `ExecMainStatus`, and
`NRestarts` from `systemctl show`. A long-running target is healthy only when it is `active/running`,
has a successful result and zero main-process exit status, and has not increased its restart count
during activation. The completed `vitals-bp-autoconnect` oneshot may be `inactive/dead` only with a
successful result and zero exit status.

Activation waits five seconds after restarting targets and observes transient activating states for
up to five additional seconds. It then records named blockers, including the service's structured
state, rather than treating a successful `systemctl restart` command as proof of process health.

## Rollback

Any non-empty health blocker follows the existing rollback path. The spool and backup gates remain
unchanged. No service is enabled as part of the code change; the operator enables the corrected
silence detector only after a direct Pi start, several 30-second cycles, and status/notification
persistence checks succeed.
