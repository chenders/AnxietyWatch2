#include "sfloat.h"
#include <math.h>

bool vitals_sfloat_decode(uint16_t raw, double *out) {
    int16_t mantissa = (int16_t)(raw & 0x0FFF);
    int8_t  exponent = (int8_t)(raw >> 12);

    /* Reserved mantissa values are ABSENT, never numbers. */
    switch (mantissa) {
        case 0x07FF:  /* NaN        */
        case 0x0800:  /* NRes       */
        case 0x07FE:  /* +INFINITY  */
        case 0x0802:  /* -INFINITY  */
        case 0x0801:  /* Reserved   */
            return false;
        default: break;
    }
    if (exponent >= 0x08) exponent -= 0x10;      /* sign-extend 4-bit  */
    if (mantissa >= 0x0800) mantissa -= 0x1000;  /* sign-extend 12-bit */
    *out = (double)mantissa * pow(10.0, (double)exponent);
    return true;
}
