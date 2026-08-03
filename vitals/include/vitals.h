/* vitals — the narrow waist.
 *
 * One append-only table of (source, type, timestamp, value) plus an HLC stamp,
 * and everything else in the system is a view over it. See
 * docs/specs/2026-08-03-pi5-vitals-core-design.md §2.
 *
 * Public surface lands here as T-D3 and T-D6 complete.
 */
#ifndef VITALS_H
#define VITALS_H

#define VITALS_VERSION_MAJOR 0
#define VITALS_VERSION_MINOR 1
#define VITALS_VERSION_PATCH 0

/* Returns "MAJOR.MINOR.PATCH". Never NULL. */
const char *vitals_version(void);

#endif /* VITALS_H */
