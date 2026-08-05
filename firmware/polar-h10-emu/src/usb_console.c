#include "usb_console.h"
#include "scenarios.h"

#include <zephyr/kernel.h>
#include <zephyr/device.h>
#include <zephyr/drivers/uart.h>
#include <zephyr/sys/ring_buffer.h>
#include <zephyr/logging/log.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

LOG_MODULE_REGISTER(usb_console, LOG_LEVEL_DBG);

#define CONSOLE_THREAD_STACK 1024
#define CONSOLE_THREAD_PRIO  7
#define CONSOLE_RINGBUF_SIZE 512
#define CONSOLE_LINE_MAX     128

static const struct device *uart_dev;

RING_BUF_DECLARE(console_rb, CONSOLE_RINGBUF_SIZE);
static struct k_thread console_thread_data;
static K_THREAD_STACK_DEFINE(console_thread_stack, CONSOLE_THREAD_STACK);
static struct k_mutex print_mutex;

/* ── Output ──────────────────────────────────────────────────────────── */

void polar_console_print(const char *line) {
    if (!uart_dev) return;

    k_mutex_lock(&print_mutex, K_FOREVER);
    for (size_t i = 0; line[i]; i++) {
        uart_poll_out(uart_dev, (unsigned char)line[i]);
    }
    uart_poll_out(uart_dev, '\r');
    uart_poll_out(uart_dev, '\n');
    k_mutex_unlock(&print_mutex);
}

void polar_console_printf(const char *fmt, ...) {
    if (!uart_dev) return;

    char buf[CONSOLE_LINE_MAX];
    va_list args;
    va_start(args, fmt);
    vsnprintf(buf, sizeof(buf), fmt, args);
    va_end(args);

    polar_console_print(buf);
}

void polar_console_print_banner(void) {
    polar_console_print("========================================");
    polar_console_print(" Polar H10 BLE Emulator (nRF52840)");
    polar_console_print(" Type 'help' for commands");
    polar_console_print("========================================");
    polar_console_printf("Active scenario: %s", polar_scenario_active()->name);
}

/* ── Commands ────────────────────────────────────────────────────────── */

static void cmd_help(void) {
    polar_console_print("Commands:");
    polar_console_print("  scenario <name>     Switch scenario (normal, sleep, exercise, afib)");
    polar_console_print("  hr min <v> max <v>  Set min/max HR in BPM");
    polar_console_print("  jitter <ms>         Set HRV jitter in ms");
    polar_console_print("  status              Show current configuration");
}

static void cmd_status(void) {
    const struct polar_scenario *s = polar_scenario_active();
    polar_console_print("--- Status ---");
    polar_console_printf("Scenario: %s", s->name);
    polar_console_printf("HR Range: %u - %u bpm", s->hr_min, s->hr_max);
    polar_console_printf("HRV Jitter: %d ms", s->hrv_jitter_ms);
}

static void handle_line(char *line) {
    if (line[0] == '\0') return;

    char *cmd = strtok(line, " \r\n");
    if (!cmd) return;

    if (strcmp(cmd, "help") == 0) {
        cmd_help();
    } else if (strcmp(cmd, "status") == 0) {
        cmd_status();
    } else if (strcmp(cmd, "scenario") == 0) {
        char *name = strtok(NULL, " \r\n");
        if (!name) {
            polar_console_print("Error: scenario requires a name.");
            return;
        }
        const struct polar_scenario *s = polar_scenario_by_name(name);
        if (!s) {
            polar_console_print("Error: unknown scenario.");
            return;
        }
        polar_scenario_activate(s);
        polar_console_printf("Activated scenario: %s", s->name);
    } else if (strcmp(cmd, "hr") == 0) {
        char *arg = strtok(NULL, " ");
        if (!arg || strcmp(arg, "min") != 0) goto hr_usage;
        arg = strtok(NULL, " ");
        if (!arg) goto hr_usage;
        int min_val = atoi(arg);

        arg = strtok(NULL, " ");
        if (!arg || strcmp(arg, "max") != 0) goto hr_usage;
        arg = strtok(NULL, " ");
        if (!arg) goto hr_usage;
        int max_val = atoi(arg);

        polar_scenario_set_hr((uint16_t)min_val, (uint16_t)max_val);
        polar_console_printf("HR range updated: %d - %d", min_val, max_val);
        return;
hr_usage:
        polar_console_print("Usage: hr min <v> max <v>");
    } else if (strcmp(cmd, "jitter") == 0) {
        char *arg = strtok(NULL, " ");
        if (!arg) {
            polar_console_print("Usage: jitter <ms>");
            return;
        }
        int j = atoi(arg);
        polar_scenario_set_jitter(j);
        polar_console_printf("HRV jitter set to %d ms", j);
    } else {
        polar_console_printf("Unknown command: %s", cmd);
    }
}

/* ── Read Loop ───────────────────────────────────────────────────────── */

static void console_thread(void *p1, void *p2, void *p3) {
    char line[CONSOLE_LINE_MAX];
    size_t pos = 0;

    polar_console_print_banner();
    polar_console_print("> ");

    while (1) {
        uint8_t ch;
        // Wait for a character (in a real blocking manner, depending on how uart_fifo_read handles it)
        // Since UART_INTERRUPT_DRIVEN is set, we can just use uart_poll_in if we want it simple,
        // but Zephyr poll is sometimes non-blocking. Let's do simple poll loop with sleep.
        if (uart_poll_in(uart_dev, &ch) == 0) {
            if (ch == '\r' || ch == '\n') {
                uart_poll_out(uart_dev, '\r');
                uart_poll_out(uart_dev, '\n');
                line[pos] = '\0';
                handle_line(line);
                pos = 0;
                polar_console_print("> ");
            } else if (ch == '\b' || ch == 127) {
                if (pos > 0) {
                    pos--;
                    uart_poll_out(uart_dev, '\b');
                    uart_poll_out(uart_dev, ' ');
                    uart_poll_out(uart_dev, '\b');
                }
            } else if (pos < sizeof(line) - 1) {
                uart_poll_out(uart_dev, ch);
                line[pos++] = (char)ch;
            }
        } else {
            k_msleep(10);
        }
    }
}

int polar_console_init(void) {
    k_mutex_init(&print_mutex);

    uart_dev = DEVICE_DT_GET(DT_CHOSEN(zephyr_console));
    if (!device_is_ready(uart_dev)) {
        LOG_ERR("UART device not ready");
        return -ENODEV;
    }

    LOG_INF("USB console UART ready");

    k_thread_create(&console_thread_data, console_thread_stack,
                    K_THREAD_STACK_SIZEOF(console_thread_stack),
                    console_thread, NULL, NULL, NULL,
                    CONSOLE_THREAD_PRIO, 0, K_NO_WAIT);

    return 0;
}
