#!/bin/sh
# FFI end-to-end proof: rise_hash_frame via cdylib matches the Rust CLI.
# Builds the cdylib, compiles a tiny C probe against bindings/macos/capture/core.h,
# hashes the same known bytes both ways, and compares.
set -eu

# Resolve locations relative to this script.
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CORE_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(CDPATH= cd -- "$CORE_DIR/.." && pwd)
HEADER="$REPO_ROOT/bindings/macos/capture/core.h"

# Toolchain checks with a clear reason and non-zero exit.
need_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "ffi_e2e: missing required tool: $1" >&2
    exit 1
  fi
}
need_tool cargo
if command -v cc >/dev/null 2>&1; then
  CC=cc
elif command -v clang >/dev/null 2>&1; then
  CC=clang
elif command -v gcc >/dev/null 2>&1; then
  CC=gcc
else
  echo "ffi_e2e: missing required tool: C compiler (cc/clang/gcc)" >&2
  exit 1
fi
if [ ! -f "$HEADER" ]; then
  echo "ffi_e2e: header not found: $HEADER" >&2
  exit 1
fi

# Build the cdylib (no new deps, Rust sources untouched).
(cd "$CORE_DIR" && cargo build) >&2

# Locate the built library: macOS dylib first, Linux .so fallback.
if [ -f "$CORE_DIR/target/debug/librise_core.dylib" ]; then
  DYLIB="$CORE_DIR/target/debug/librise_core.dylib"
elif [ -f "$CORE_DIR/target/debug/librise_core.so" ]; then
  DYLIB="$CORE_DIR/target/debug/librise_core.so"
else
  echo "ffi_e2e: built library not found (expected librise_core.dylib/.so in $CORE_DIR/target/debug)" >&2
  exit 1
fi

# Scratch space, cleaned up on exit.
TMPDIR_E2E=$(mktemp -d 2>/dev/null || mktemp -d -t rise-ffi-e2e)
trap 'rm -rf "$TMPDIR_E2E"' EXIT INT TERM
FRAME_FILE="$TMPDIR_E2E/frame.bin"
PROBE_C="$TMPDIR_E2E/probe.c"
PROBE_BIN="$TMPDIR_E2E/probe"

# Known input bytes (no trailing newline).
printf '%s' 'rise-ffi-e2e-proof-bytes-001' > "$FRAME_FILE"

# Expected hash from the Rust CLI on the same bytes.
EXPECTED=$(cd "$CORE_DIR" && cargo run --quiet -- hash "$FRAME_FILE")
echo "expected: $EXPECTED"

# Tiny C program: reads the frame file, calls rise_hash_frame, prints hex.
cat > "$PROBE_C" <<'EOF'
#include <stdio.h>
#include <stdlib.h>
#include "core.h"

int main(int argc, char **argv) {
  if (argc != 2) {
    fprintf(stderr, "usage: probe <frame-file>\n");
    return 2;
  }
  FILE *f = fopen(argv[1], "rb");
  if (!f) {
    perror("fopen");
    return 2;
  }
  if (fseek(f, 0, SEEK_END) != 0) {
    perror("fseek");
    fclose(f);
    return 2;
  }
  long n = ftell(f);
  if (n < 0) {
    perror("ftell");
    fclose(f);
    return 2;
  }
  rewind(f);
  unsigned char *buf = NULL;
  if (n > 0) {
    buf = (unsigned char *)malloc((size_t)n);
    if (!buf) {
      fprintf(stderr, "oom\n");
      fclose(f);
      return 2;
    }
    if (fread(buf, 1, (size_t)n, f) != (size_t)n) {
      fprintf(stderr, "short read\n");
      free(buf);
      fclose(f);
      return 2;
    }
  }
  fclose(f);
  char out[65];
  rise_hash_frame(buf ? buf : (const unsigned char *)"", (size_t)n, out);
  free(buf);
  printf("%s\n", out);
  return 0;
}
EOF

# Compile against the header, linking the built library directly.
"$CC" -std=c11 -Wall -Wextra -I "$(dirname -- "$HEADER")" "$PROBE_C" "$DYLIB" -o "$PROBE_BIN" >&2

# Run with the library discoverable at runtime (macOS + Linux).
if [ "$(uname -s)" = "Darwin" ]; then
  GOT=$(DYLD_LIBRARY_PATH="$(dirname -- "$DYLIB")${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}" "$PROBE_BIN" "$FRAME_FILE")
else
  GOT=$(LD_LIBRARY_PATH="$(dirname -- "$DYLIB")${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" "$PROBE_BIN" "$FRAME_FILE")
fi
echo "got:      $GOT"

if [ "$GOT" = "$EXPECTED" ]; then
  echo "FFI_OK"
  exit 0
else
  echo "ffi_e2e: hash mismatch (FFI vs CLI)" >&2
  exit 1
fi
