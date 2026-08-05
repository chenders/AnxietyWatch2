/*
 * usb_console.c — USB CDC ACM serial console command parser
 *
 * A dedicated thread reads lines from the USB CDC ACM UART and dispatches
 * them to command handlers.  This module is the runtime control surface
 * for the dongle: scenario selection, parameter tweaks, key changes, etc.
 *
 * The UART device is `CDC_ACM_0` (default from the overlay's cdc_acm_uart0).
 * The console thread waits for USB enumeration, then enters the read loop.
 */

#include "usb_console.h"
#include "scenarios.h"
#include "auth.h"
#include "measurements.h"

#include <stdio.h>
#include <string.h>
#include <ctype.h>
#include <stdlib.h>

#include <zephyr/kernel.h>
#include <zephyr/device.h>
#include <zephyr/drivers/uart.h>
#include <zephyr/logging/log.h>
#include <zephyr/sys/ring_buffer.h>

LOG_MODULE_REGISTER(console, LOG_LEVEL_DBG);

/* ── Ring buffer and UART ────────────────────────────────────────────── */

#define CONSOLE_RINGBUF_SIZE 1024
#define CONSOLE_LINE_MAX     128

static const struct device *uart_dev;

RING_BUF_DECLARE(console_rb, CONSOLE_RINGBUF_SIZE);

static struct k_thread console_thread_data;
static K_THREAD_STACK_DEFINE(console_stack, 2048);

/* ── Mutex for print thread safety ───────────────────────────────────── */

static struct k_mutex print_mutex;

/* ── Forward ─────────────────────────────────────────────────────────── */

static void console_thread_fn(void *arg1, void *arg2, void *arg3);

/* ── UART callback (ISR → ring buffer) ───────────────────────────────── */

static void uart_cb(const struct device *dev,
    struct uart_event *evt, void *user_data)
{
    (void)user_data;

    switch (evt->type) {
    case UART_RX_RDY:
        /* Write received bytes into ring buffer */
        ring_buf_put(&console_rb, evt->data.rx.buf + evt->data.rx.offset,
            evt->data.rx.len);
        break;

    case UART_RX_BUF_REQUEST:
        /* Provide a buffer for the next chunk */
        /* Handled automatically by async UART */
        break;

    case UART_RX_DISABLED:
        /* Re-enable reception */
        uart_rx_enable(dev, evt->data.rx.buf, evt->data.rx.len, SYS_FOREVER_MS);
        break;

    default:
        break;
    }
}

/* ── Public: init ─────────────────────────────────────────────────────── */

int oura_console_init(void)
{
    k_mutex_init(&print_mutex);

    uart_dev = DEVICE_DT_GET(DT_CHOSEN(zephyr_console));
    if (!device_is_ready(uart_dev)) {
        LOG_ERR("UART device not ready — is USB enumerated?");
        return -ENODEV;
    }

    LOG_INF("USB console UART ready: %s", uart_dev->name);

    /* Start console thread */
    k_thread_create(&console_thread_data, console_stack,
        K_THREAD_STACK_SIZEOF(console_stack),
        console_thread_fn,
        NULL, NULL, NULL,
        K_PRIO_PREEMPT(7), 0, K_NO_WAIT);

    return 0;
}

/* ── Public: print ────────────────────────────────────────────────────── */

void oura_console_print(const char *line)
{
    k_mutex_lock(&print_mutex, K_FOREVER);

    for (size_t i = 0; line[i]; i++) {
        uart_poll_out(uart_dev, (unsigned char)line[i]);
    }
    uart_poll_out(uart_dev, '\r');
    uart_poll_out(uart_dev, '\n');

    k_mutex_unlock(&print_mutex);
}

void oura_console_printf(const char *fmt, ...)
{
    char buf[CONSOLE_LINE_MAX];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(buf, sizeof(buf), fmt, ap);
    va_end(ap);
    oura_console_print(buf);
}

void oura_console_print_banner(void)
{
    oura_console_print("");
    oura_console_print("╔══════════════════════════════════════════╗");
    oura_console_print("║    Oura Ring BLE Emulator Dongle         ║");
    oura_console_print("║    nRF52840  |  nRF Connect SDK          ║");
    oura_console_print("║    AnxietyWatch — DEBUG / TEST ONLY      ║");
    oura_console_print("╠══════════════════════════════════════════╣");
    oura_console_print("║  Type 'help' for commands                ║");
    oura_console_print("╚══════════════════════════════════════════╝");
    oura_console_print("");

    oura_console_print("Shared key: configured");
    oura_console_printf("Scenario: %s", oura_scenario_active()->name);
    oura_console_print("");
}

/* ── Line reader ──────────────────────────────────────────────────────── */

/**
 * Read one line from the ring buffer into `line`.
 * Returns the line length (excluding \r\n), or 0 if no line ready.
 */
static size_t read_line(char *line, size_t max_len)
{
    size_t pos = 0;
    uint8_t ch;

    while (pos < max_len - 1) {
        uint32_t avail = ring_buf_get(&console_rb, &ch, 1);
        if (avail == 0) break;

        if (ch == '\r' || ch == '\n') {
            line[pos] = '\0';
            /* Consume a possible \n after \r */
            uint32_t more = ring_buf_get(&console_rb, &ch, 1);
            if (more > 0 && ch != '\n') {
                /* Put back non-newline char — simple: ignore edge case */
            }
            return pos;
        }

        /* Echo the character */
        uart_poll_out(uart_dev, ch);

        line[pos++] = (char)ch;
    }

    line[pos] = '\0';
    return 0; /* incomplete line, wait for more */
}

/* ── Tokenizer ───────────────────────────────────────────────────────── */

static char *token_arg(char **line_ptr) {
    char *start = *line_ptr;

    /* Skip leading whitespace */
    while (*start && isspace((unsigned char)*start)) start++;
    if (*start == '\0') return NULL;

    /* Find end of token */
    char *end = start;
    while (*end && !isspace((unsigned char)*end)) end++;

    if (*end) {
        *end = '\0';
        *line_ptr = end + 1;
    } else {
        *line_ptr = end;
    }

    return start;
}

/* ── Command handlers ────────────────────────────────────────────────── */

static void cmd_scenario(char *args)
{
    char *name = token_arg(&args);
    if (!name) {
        oura_console_print("Usage: scenario normal|desat|sleep|exercise|idle");
        return;
    }

    const struct oura_scenario *s = oura_scenario_by_name(name);
    if (!s) {
        oura_console_printf("Unknown scenario: %s", name);
        oura_console_print("Available:");
        oura_scenario_list_all(oura_console_print);
        return;
    }

    oura_scenario_activate(s);
    oura_console_printf("Scenario set to: %s", s->name);
}

static void cmd_hr(char *args)
{
    char *tok = token_arg(&args);
    if (!tok) {
        oura_console_print("Usage: hr min <bpm> max <bpm>");
        return;
    }

    /* Expect: min <val> max <val> */
    int min_bpm = -1, max_bpm = -1;
    while (tok) {
        if (strcmp(tok, "min") == 0) {
            tok = token_arg(&args);
            if (tok) min_bpm = atoi(tok);
        } else if (strcmp(tok, "max") == 0) {
            tok = token_arg(&args);
            if (tok) max_bpm = atoi(tok);
        }
        tok = token_arg(&args);
    }

    if (min_bpm <= 0 || max_bpm <= 0 || min_bpm >= max_bpm) {
        oura_console_print("Usage: hr min <bpm> max <bpm>  (e.g. hr min 60 max 80)");
        return;
    }

    struct oura_meas_ibi_params p = {min_bpm, max_bpm, 0, 5};
    oura_meas_set_ibi_params(&p);
    oura_meas_reset_walk();
    oura_console_printf("HR range: %d–%d BPM", min_bpm, max_bpm);
}

static void cmd_spo2(char *args)
{
    char *tok = token_arg(&args);
    if (!tok) {
        oura_console_print("Usage: spo2 min <%> max <%>");
        return;
    }

    int min_pct = -1, max_pct = -1;
    while (tok) {
        if (strcmp(tok, "min") == 0) {
            tok = token_arg(&args);
            if (tok) min_pct = atoi(tok);
        } else if (strcmp(tok, "max") == 0) {
            tok = token_arg(&args);
            if (tok) max_pct = atoi(tok);
        }
        tok = token_arg(&args);
    }

    if (min_pct < 0 || max_pct < 0 || min_pct > max_pct || max_pct > 100) {
        oura_console_print("Usage: spo2 min <%> max <%>  (e.g. spo2 min 88 max 94)");
        return;
    }

    struct oura_meas_spo2_params p = {min_pct, max_pct, -1};
    oura_meas_set_spo2_params(&p);
    oura_meas_reset_walk();
    oura_console_printf("SpO2 range: %d–%d%%", min_pct, max_pct);
}

static void cmd_ibi(char *args)
{
    char *tok = token_arg(&args);
    if (!tok) {
        oura_console_print("Usage: ibi mean <ms> jitter <ms>");
        return;
    }

    int mean_ms = -1, jitter_ms = -1;
    while (tok) {
        if (strcmp(tok, "mean") == 0) {
            tok = token_arg(&args);
            if (tok) mean_ms = atoi(tok);
        } else if (strcmp(tok, "jitter") == 0) {
            tok = token_arg(&args);
            if (tok) jitter_ms = atoi(tok);
        }
        tok = token_arg(&args);
    }

    if (mean_ms < 300 || mean_ms > 2000) {
        oura_console_print("IBI mean must be 300–2000 ms");
        return;
    }
    if (jitter_ms < 0 || jitter_ms > 500) {
        oura_console_print("IBI jitter should be 0–500 ms");
        return;
    }

    struct oura_meas_ibi_params p = {0, 0, mean_ms, jitter_ms};
    oura_meas_set_ibi_params(&p);
    oura_meas_reset_walk();
    oura_console_printf("IBI: mean=%d ms  jitter=±%d ms", mean_ms, jitter_ms);
}

static void cmd_accel(char *args)
{
    char *tok = token_arg(&args);
    if (!tok) {
        oura_console_print("Usage: accel on|off");
        return;
    }

    bool on;
    if (strcmp(tok, "on") == 0) {
        on = true;
    } else if (strcmp(tok, "off") == 0) {
        on = false;
    } else {
        oura_console_print("Usage: accel on|off");
        return;
    }

    struct oura_meas_accel_params p = {on, on ? 2000 : 0};
    oura_meas_set_accel_params(&p);
    oura_console_printf("Accelerometer: %s", on ? "ON (50 Hz)" : "OFF");
}

static void cmd_temp(char *args)
{
    char *tok = token_arg(&args);
    if (!tok || strcmp(tok, "deviation") != 0) {
        oura_console_print("Usage: temp deviation <°C>  (e.g. temp deviation -0.3)");
        return;
    }

    tok = token_arg(&args);
    if (!tok) {
        oura_console_print("Usage: temp deviation <°C>");
        return;
    }

    /* Parse as float, convert to hundredths */
    float dev_c = strtof(tok, NULL);
    int dev_hundredths = (int)(dev_c * 100.0f);

    struct oura_meas_temp_params p = {dev_hundredths, 1};
    oura_meas_set_temp_params(&p);
    oura_console_printf("Temperature deviation: %.2f °C", (double)dev_c);
}

static void cmd_sleep_stage(char *args)
{
    char *tok = token_arg(&args);
    if (!tok) {
        oura_console_print("Usage: sleep <stage>  (0=awake 1=light 2=deep 3=REM, or 'auto')");
        return;
    }

    int stage;
    if (strcmp(tok, "auto") == 0) {
        stage = -1;
    } else {
        stage = atoi(tok);
        if (stage < 0 || stage > 3) {
            oura_console_print("Sleep stage must be 0–3 or 'auto'");
            return;
        }
    }

    struct oura_meas_sleep_params p = {stage};
    oura_meas_set_sleep_params(&p);
    if (stage < 0) {
        oura_console_print("Sleep stage: auto (cycled by scenario)");
    } else {
        static const char *names[] = {"awake", "light", "deep", "REM"};
        oura_console_printf("Sleep stage forced: %s (%d)", names[stage], stage);
    }
}

static void cmd_battery(char *args)
{
    char *tok = token_arg(&args);
    if (!tok) {
        oura_console_print("Usage: battery <pct>  (0–100)");
        return;
    }

    int pct = atoi(tok);
    if (pct < 0 || pct > 100) {
        oura_console_print("Battery: 0–100");
        return;
    }

    struct oura_meas_battery_params p = {pct, false};
    oura_meas_set_battery_params(&p);
    oura_console_printf("Battery: %d%%", pct);
}

static void cmd_key(char *args)
{
    char *hex = token_arg(&args);
    if (!hex) {
        oura_console_print("Usage: key <32-char-hex>");
        return;
    }

    int ret = oura_auth_set_key_hex(hex);
    if (ret == 0) {
        oura_console_print("Key updated.");
    } else {
        oura_console_print("Invalid key. Need exactly 32 hex characters (16 bytes).");
    }
}

static void cmd_status(char *args)
{
    (void)args;

    const struct oura_scenario *s = oura_scenario_active();

    oura_console_print("── Status ──────────────────────────────");
    oura_console_printf("  Scenario:   %s", s->name);
    oura_console_print("  Key:        configured");
    oura_console_printf("  Uptime:     %u s", (unsigned int)(k_uptime_get() / 1000));
    oura_console_printf("  HR range:   %d–%d BPM", s->ibi.hr_min_bpm, s->ibi.hr_max_bpm);
    oura_console_printf("  SpO2 range: %d–%d%%", s->spo2.spo2_min_pct, s->spo2.spo2_max_pct);
    oura_console_printf("  Accel:      %s", s->accel.active ? "ON" : "OFF");
    oura_console_printf("  Temp dev:   %d.%02d °C",
        s->temp.deviation_hundredths / 100,
        abs(s->temp.deviation_hundredths) % 100);
    if (s->sleep.forced_stage >= 0) {
        oura_console_printf("  Sleep:      stage %d", s->sleep.forced_stage);
    } else {
        oura_console_print("  Sleep:      auto");
    }
    oura_console_printf("  Battery:    %d%% %s",
        s->battery.percent, s->battery.charging ? "(charging)" : "");
    oura_console_print("────────────────────────────────────────");
}

static void cmd_help(char *args)
{
    (void)args;
    oura_console_print("Commands:");
    oura_console_print("  scenario normal|desat|sleep|exercise|idle");
    oura_console_print("  hr min <bpm> max <bpm>");
    oura_console_print("  spo2 min <%> max <%>");
    oura_console_print("  ibi mean <ms> jitter <ms>");
    oura_console_print("  accel on|off");
    oura_console_print("  temp deviation <°C>");
    oura_console_print("  sleep <0-3|auto>");
    oura_console_print("  battery <pct>");
    oura_console_print("  key <hex>");
    oura_console_print("  status");
    oura_console_print("  help");
}

/* ── Command dispatch ────────────────────────────────────────────────── */

typedef void (*cmd_handler_t)(char *args);

static struct {
    const char *name;
    cmd_handler_t fn;
} commands[] = {
    {"scenario", cmd_scenario},
    {"hr",       cmd_hr},
    {"spo2",     cmd_spo2},
    {"ibi",      cmd_ibi},
    {"accel",    cmd_accel},
    {"temp",     cmd_temp},
    {"sleep",    cmd_sleep_stage},
    {"battery",  cmd_battery},
    {"key",      cmd_key},
    {"status",   cmd_status},
    {"help",     cmd_help},
    {NULL, NULL}
};

static void dispatch_command(char *line)
{
    /* Skip leading whitespace */
    while (*line && isspace((unsigned char)*line)) line++;
    if (*line == '\0' || *line == '#') return; /* empty or comment */

    char *cmd_name = token_arg(&line);
    char *args = line; /* rest of line after command name */

    for (int i = 0; commands[i].name; i++) {
        if (strcmp(cmd_name, commands[i].name) == 0) {
            commands[i].fn(args);
            return;
        }
    }

    oura_console_printf("Unknown command: %s  (try 'help')", cmd_name);
}

/* ── Console thread ──────────────────────────────────────────────────── */

static void console_thread_fn(void *arg1, void *arg2, void *arg3)
{
    (void)arg1;
    (void)arg2;
    (void)arg3;

    /* Wait for USB enumeration (device ready) */
    k_sleep(K_MSEC(2000));

    /* Set up async UART reception */
    static uint8_t rx_buf[64];
    uart_callback_set(uart_dev, uart_cb, NULL);
    uart_rx_enable(uart_dev, rx_buf, sizeof(rx_buf), SYS_FOREVER_MS);

    oura_console_print_banner();

    char line[CONSOLE_LINE_MAX];
    oura_console_print("oura-emu> ");

    while (1) {
        /* Poll ring buffer for a complete line */
        size_t len = read_line(line, sizeof(line));
        if (len > 0) {
            if (len > 0) {
                oura_console_print(""); /* echo newline after command */
            }
            dispatch_command(line);
            oura_console_print("");
            oura_console_print("oura-emu> ");
        }

        /* Sleep briefly to avoid busy-waiting */
        k_msleep(50);
    }
}
