# Proposal: Make spool identity stable across service restart

The first post-merge activation passed preflight but rolled back because restarting `vitalsd`
created a new empty spool segment. The segment's bytes and pending data were unchanged, but the
fingerprint included the time-derived filename. That made the deployment gate reject a safe
append-only restart boundary.

This change keeps the gate strict about pending spool contents while making its identity independent
of segment filenames. It does not permit changed bytes, changed total size, or an additional
undrained segment. The existing backup, restore, off-Pi copy, rollback, and operator gates remain
required.
