# Traceability — Pi prototype storage degraded mode

| Requirement | Design location | Task | Evidence |
|---|---|---|---|
| Preserve production invariants before relaxation | proposal hard gates 1–3 | T-P0, T-P1 | baseline and verified recovery manifest |
| Backup PostgreSQL off Pi | design §2 | T-P1 | encrypted backup, checksum, restore/readability result |
| Preserve pending spool data | design §2 | T-P1 | byte-for-byte spool manifest and checksum comparison |
| Preserve planes/noise and CNS data in recovery | design §2.1 | T-P1, T-P5a | unfiltered backup and inventory |
| Remove only proven project-exclusive data | design §2.1 | T-P5a, T-P5 | classified cleanup manifest and operator decision |
| Never delete/update `samples` | design §2.1 | T-P5a | inventory proves no samples deletion attempt; tombstone path |
| Reduce load in least-data-affecting order | design §3 | T-P2, T-P3 | active profile and load-control evidence |
| No delete/update/offset manipulation | proposal prototype restrictions | T-P1–T-P3 | review plus command evidence; any mutation fails task |
| Keep safety-relevant sources lossless | design §4 | T-P5 | operator decision artifact |
| Make source-specific reduction explicit | proposal hard gate 3 | T-P5 | named affected source/type and rollback command |
| Measure rather than assume second NVMe helps | design §5 | T-P4 | topology decision with contention and power/thermal values |
| Re-enter production only after all gates | design §6 | T-P6 | production acceptance artifact |
| Do not resolve schema drift by workaround | proposal non-goals; design §6 | T-P0, T-P6 | schema state remains explicit and reconciled deliberately |

## Decision record

The second-NVMe question remains open until T-P4. The intended production design permits a second
NVMe when measured storage contention, endurance, capacity, and failure-domain benefits outweigh
purchase, power, thermal, PCIe, and maintenance costs. A second device is not a substitute for
bounded queries, connection management, spool indexing, or safe backpressure.
