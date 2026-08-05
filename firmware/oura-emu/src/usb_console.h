/*
 * usb_console.h — USB CDC ACM serial console command parser
 *
 * Provides a line-oriented command interface over the USB virtual serial
 * port (/dev/ttyACM0 or /dev/cu.usbmodem*).
 *
 * Commands (case-insensitive):
 *   scenario normal|desat|sleep|exercise|idle
 *   hr min <bpm> max <bpm>
 *   spo2 min <%> max <%>
 *   ibi mean <ms> jitter <ms>
 *   accel on|off
 *   temp deviation <°C>
 *   sleep <stage>
 *   battery <pct>
 *   key <hex>
 *   status
 *   help
 */

#ifndef USB_CONSOLE_H
#define USB_CONSOLE_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Initialize the USB CDC ACM UART and start the console read loop.
 *
 * This function returns immediately; the console runs in a dedicated
 * thread that reads lines and dispatches commands.
 *
 * @return 0 on success, negative errno on failure.
 */
int oura_console_init(void);

/**
 * Print a line to the USB console.  Thread-safe.
 */
void oura_console_print(const char *line);

/**
 * Print a formatted line to the USB console.  Thread-safe.
 */
void oura_console_printf(const char *fmt, ...);

/**
 * Banner shown at boot.  Call after USB is enumerated.
 */
void oura_console_print_banner(void);

#ifdef __cplusplus
}
#endif

#endif /* USB_CONSOLE_H */
