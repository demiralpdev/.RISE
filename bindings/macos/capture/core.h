/* RISE core FFI header — MS1 stub.
 * MS2 will add the real signature ABI (ES256). */
#ifndef RISE_CORE_H
#define RISE_CORE_H

#include <stddef.h>

/* Computes SHA256(raw frame); the caller allocates a 65-byte out_hex65. */
void rise_hash_frame(const unsigned char *bytes, size_t len, char *out_hex65);

#endif
