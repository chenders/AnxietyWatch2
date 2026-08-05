#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

int polar_console_init(void);
void polar_console_print(const char *line);
void polar_console_printf(const char *fmt, ...);
void polar_console_print_banner(void);

#ifdef __cplusplus
}
#endif
