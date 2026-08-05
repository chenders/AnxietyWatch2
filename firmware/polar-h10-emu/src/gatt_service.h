#pragma once

#include <stdint.h>
#include <stdbool.h>

void gatt_service_init(void);

// Notifies the connected client with a Heart Rate Measurement frame.
// data: pointer to the frame bytes
// len:  number of bytes in the frame
void gatt_notify_hrm(const uint8_t *data, uint16_t len);

// Returns true if a client is connected and has enabled notifications
// on the Heart Rate Measurement characteristic.
bool gatt_is_subscribed(void);
