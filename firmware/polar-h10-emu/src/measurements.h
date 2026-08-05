#pragma once

#include <stdint.h>

void measurements_init(void);

// Begin the 1Hz heartbeat timer that calculates and sends RR intervals
void measurements_start(void);

// Stop the timer
void measurements_stop(void);
