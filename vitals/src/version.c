#include "vitals.h"

#define VSTR2(x) #x
#define VSTR(x)  VSTR2(x)

const char *vitals_version(void) {
    return VSTR(VITALS_VERSION_MAJOR) "." VSTR(VITALS_VERSION_MINOR)
           "." VSTR(VITALS_VERSION_PATCH);
}
