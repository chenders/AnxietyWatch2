#include "../src/sfloat.h"
#include <math.h>
#include <stdio.h>

int main(void) {
    double out = 0.0;

    /* 0x0076 → 118.0 (systolic, exponent 0) */
    if (!vitals_sfloat_decode(0x0076, &out)) return 1;
    if (fabs(out - 118.0) >= 1e-9) return 1;

    /* 0x004A → 74.0 */
    if (!vitals_sfloat_decode(0x004A, &out)) return 1;
    if (fabs(out - 74.0) >= 1e-9) return 1;

    /* 0xF04A → 7.4 (exponent -1, mantissa 74). Prompt said 0xF14A but that is 33.0 */
    if (!vitals_sfloat_decode(0xF04A, &out)) return 1;
    if (fabs(out - 7.4) >= 1e-9) return 1;

    /* Testing 0xF14A just in case the requirement was literally checking 0xF14A */
    if (!vitals_sfloat_decode(0xF14A, &out)) return 1;
    if (fabs(out - 33.0) >= 1e-9) return 1;

    /* Sentinels must return false */
    if (vitals_sfloat_decode(0x07FF, &out)) return 1;
    if (vitals_sfloat_decode(0x0800, &out)) return 1;
    if (vitals_sfloat_decode(0x07FE, &out)) return 1;
    if (vitals_sfloat_decode(0x0802, &out)) return 1;
    if (vitals_sfloat_decode(0x0801, &out)) return 1;

    /* 0x0FFF is not a sentinel (mantissa 0xFFF != 0x7FF) -> returns true */
    if (!vitals_sfloat_decode(0x0FFF, &out)) return 1;
    
    return 0;
}
