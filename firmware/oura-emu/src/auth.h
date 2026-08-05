/*
 * auth.h — AES-ECB nonce challenge for Oura Ring authentication
 *
 * Flow (matches OuraBLEProtocol.swift):
 *   1. Client writes 16-byte nonce to Auth characteristic.
 *   2. Dongle encrypts nonce with AES-ECB using the pre-shared 16-byte key.
 *   3. Dongle notifies encrypted response (16 bytes).
 *   4. Client decrypts and writes the original nonce back.
 *   5. Dongle compares → auth success or failure.
 *
 * The default key is 00112233445566778899AABBCCDDEEFF (the same as
 * OuraBLEActorTests uses).  It can be changed at runtime via the USB
 * console: `key <hex>`.
 */

#ifndef AUTH_H
#define AUTH_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

#define OURA_AUTH_KEY_LEN  16
#define OURA_AUTH_NONCE_LEN 16

/* Default shared key (documented in the plan; matches iOS test key) */
#define OURA_AUTH_DEFAULT_KEY \
    { 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, \
      0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF }

/**
 * Initialize the mbedTLS AES context with the given key.
 * Must be called once at startup.
 *
 * @param key  16-byte shared key. If NULL the default key is used.
 */
int oura_auth_init(const uint8_t *key);

/**
 * Process a nonce challenge from the client.
 *
 * Encrypts the nonce with AES-ECB and stores the expected response
 * internally.  Call oura_auth_notify_response() to send the result back.
 *
 * @param nonce  16-byte nonce received from client.
 * @return 0 on success, negative errno on failure.
 */
int oura_auth_handle_challenge(const uint8_t *nonce);

/**
 * Get the encrypted response to send back to the client.
 * Call after oura_auth_handle_challenge().
 *
 * @param out   Must be at least 16 bytes.
 */
void oura_auth_get_response(uint8_t *out);

/**
 * Verify the client's decrypted nonce against the stored nonce.
 *
 * @param response  16 bytes written by client to confirm auth.
 * @return true if auth succeeded.
 */
bool oura_auth_verify_response(const uint8_t *response);

/**
 * Set a new shared key at runtime (via USB console `key <hex>`).
 *
 * @param hex  32-character hex string representing 16 bytes.
 * @return 0 on success, -EINVAL if the hex string is malformed.
 */
int oura_auth_set_key_hex(const char *hex);

/**
 * Read the current key as a hex string.  Useful for `status` console command.
 *
 * @param out  Buffer of at least 33 bytes (32 hex + NUL).
 */
void oura_auth_get_key_hex(char *out);

/**
 * Reset auth state (e.g., on disconnect).
 */
void oura_auth_reset(void);

#ifdef __cplusplus
}
#endif

#endif /* AUTH_H */
