cmake_minimum_required(VERSION 3.22)
project(vitals C)

# ── Why CMake and not meson ──────────────────────────────────────────────────
# The design spec named meson. CMake is used instead for one reason: JetBrains
# tooling (CLion, IntelliJ with the C/C++ plugin) has first-class CMake support
# and only plugin-grade meson support. Since this project is driven from an
# IntelliJ project, the IDE's native build system wins. Nothing in the design
# depends on the choice — CTest gives the same exit-code-based acceptance the
# plan's Verify steps rely on.

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)   # clangd / IDE indexing

# Warnings are errors. The plan treats a new warning as a failing test.
add_compile_options(-Wall -Wextra -Werror -Wshadow -Wconversion)

option(VITALS_SANITIZE "Build tests with ASan + UBSan" ON)
option(VITALS_FUZZ     "Build libFuzzer targets (requires clang)" OFF)
option(VITALS_COVERAGE "Instrument for gcov/llvm-cov coverage" OFF)

if(VITALS_COVERAGE)
  # Coverage and sanitizers together produce noisy, slow builds; keep them apart.
  set(VITALS_SANITIZE OFF)
  add_compile_options(--coverage -O0 -g)
  add_link_options(--coverage)
endif()

if(VITALS_SANITIZE)
  add_compile_options(-fsanitize=address,undefined -fno-omit-frame-pointer)
  add_link_options(-fsanitize=address,undefined)
endif()

include_directories(${CMAKE_SOURCE_DIR}/vitals/include)
include_directories(${CMAKE_SOURCE_DIR}/vitals/generated)

enable_testing()
add_subdirectory(vitals)
