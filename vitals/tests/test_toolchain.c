/* Proves the toolchain end to end before any real task runs: C11 compiles,
 * warnings-as-errors passes, sanitizers link, the static lib links, and CTest
 * reports an exit code. If this fails, no T-D* task can be trusted. */
#include "vitals.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    const char *v = vitals_version();
    assert(v != NULL);
    assert(strcmp(v, "0.1.0") == 0);

    /* The store holds UTC epoch seconds as double. Assert the representation
     * carries sub-millisecond resolution at present-day magnitudes — sample
     * timestamps depend on it. */
    double now = 1785748180.0;
    assert((now + 0.001) != now);

    printf("vitals %s · toolchain ok\n", v);
    return 0;
}
