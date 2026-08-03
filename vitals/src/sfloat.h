#ifndef VITALS_SFLOAT_H
#define VITALS_SFLOAT_H
#include <stdbool.h>
#include <stdint.h>
/* Decodes an IEEE-11073 16-bit SFLOAT.
 * Returns false for NaN/NRes/±INFINITY/Reserved — the value is ABSENT.
 * *out is untouched when false is returned. */
bool vitals_sfloat_decode(uint16_t raw, double *out);
#endif
