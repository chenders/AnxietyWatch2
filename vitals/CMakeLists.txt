# ── vitals ───────────────────────────────────────────────────────────────────
# Two libraries, split by portability. The split is not cosmetic: it is what
# lets the pure logic be written and unit-tested on a Mac in seconds while the
# code that touches radios and the kernel is built and run where it will
# actually live.
#
# It also maps exactly onto the task tags in
# openspec/changes/pi5-vitals-core/tasks.md:
#   host: either  →  vitals_core   (builds and tests on macOS and Linux)
#   host: pi      →  vitals_linux  (Linux only; silently skipped on macOS)

# ── vitals_core · portable ───────────────────────────────────────────────────
# Pure C11. No syscalls, no OS headers, no allocation in the decoders.
# Apple Silicon and the Pi 5 are both arm64 little-endian, so byte layout and
# struct packing agree — these produce identical results on both.
set(VITALS_CORE_SOURCES
    src/version.c
    src/sfloat.c        # T-D2 · IEEE-11073 SFLOAT
    # src/hlc.c           # T-D3 · hybrid logical clock
    # src/bp_decode.c     # T-D6 · 0x2A35 blood pressure measurement
)

add_library(vitals_core STATIC ${VITALS_CORE_SOURCES})
target_include_directories(vitals_core PUBLIC
    ${CMAKE_SOURCE_DIR}/vitals/include
    ${CMAKE_SOURCE_DIR}/vitals/generated)

# ── vitals_linux · Linux only ────────────────────────────────────────────────
# BlueZ over D-Bus, epoll, sd_notify, getrandom(2), /sys reads. None of this
# compiles on macOS, and none of it should be faked there — a mock BlueZ would
# test the mock. Configure on a Mac and these targets are simply absent.
if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(SQLITE3 REQUIRED sqlite3)
    pkg_check_modules(DBUS    QUIET    dbus-1)

    set(VITALS_LINUX_SOURCES
        # src/libvitals.c  # T-D3 · SQLite-backed store
        # src/vitalsd.c    # T-D4 · daemon, epoll, unix socket
        # src/wal_tail.c   # T-D5 · SQLite WAL -> Postgres
    )

    if(VITALS_LINUX_SOURCES)
        add_library(vitals_linux STATIC ${VITALS_LINUX_SOURCES})
        target_link_libraries(vitals_linux PUBLIC vitals_core ${SQLITE3_LIBRARIES})
        target_include_directories(vitals_linux PUBLIC ${SQLITE3_INCLUDE_DIRS})
    endif()
else()
    message(STATUS "vitals: non-Linux host — Linux-only targets skipped. "
                   "Portable core still builds and tests. Use the remote "
                   "toolchain (docs/HERDR-GUIDE.md §4) for host:pi work.")
endif()

# ── tests ────────────────────────────────────────────────────────────────────
# One CTest target per task, so `ctest -R sfloat` runs exactly T-D2's suite —
# which is what that task's Verify line invokes.

function(vitals_test name)
    add_executable(test_${name} tests/test_${name}.c)
    target_link_libraries(test_${name} PRIVATE vitals_core)
    add_test(NAME ${name} COMMAND test_${name})
endfunction()

vitals_test(toolchain)          # proves the build works end to end
vitals_test(sfloat)           # T-D2
# vitals_test(bp_decode)        # T-D6
# vitals_test(hlc)              # T-D3

# ── fuzz targets ─────────────────────────────────────────────────────────────
# Every function that parses bytes off a radio gets one. Decoders are the only
# code here that sees hostile input, and they are the easiest possible shape to
# prove correct: no state, no I/O, one buffer in, one struct out.
#   cmake -B build-fuzz -DVITALS_FUZZ=ON -DVITALS_SANITIZE=OFF -DCMAKE_C_COMPILER=clang

if(VITALS_FUZZ)
    function(vitals_fuzz name)
        add_executable(fuzz_${name} fuzz/fuzz_${name}.c)
        target_link_libraries(fuzz_${name} PRIVATE vitals_core)
        target_compile_options(fuzz_${name} PRIVATE -fsanitize=fuzzer,address,undefined)
        target_link_options(fuzz_${name}    PRIVATE -fsanitize=fuzzer,address,undefined)
    endfunction()

    vitals_fuzz(sfloat)       # T-D2
    # vitals_fuzz(bp_decode)    # T-D6
endif()
