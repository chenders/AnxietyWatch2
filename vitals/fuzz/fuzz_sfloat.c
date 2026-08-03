#include "../src/sfloat.h"
#include <stddef.h>
#include <stdint.h>

int LLVMFuzzerTestOneInput(const uint8_t *d, size_t n) {
    if (n < 2) return 0;
    double v;
    uint16_t raw = (uint16_t)(d[0] | (d[1] << 8));
    if (vitals_sfloat_decode(raw, &v)) {
        /* A decode that succeeds must produce a finite number. */
        if (!(v == v) || v > 1e300 || v < -1e300) __builtin_trap();
    }
    return 0;
}
