/*
 * packet.h — 16-byte packet framing with XOR checksum
 *
 * Every Oura BLE packet is exactly 16 bytes:
 *   [command:1][payload:14][checksum:1]
 *
 * Checksum = XOR of bytes 0..14.
 *
 * Measurement payload formats (see measurements.h for builders):
 *
 *   Tag 0x2F (Live HR / IBI):
 *     [0]=0x2F [1]=sub-tag [2:3]=IBI_ms_LE [4:14]=reserved [15]=XOR
 *
 *   Tag 0x07 (SpO2):
 *     [0]=0x07 [1]=SpO2% [2]=quality<<4|contact [3:14]=reserved [15]=XOR
 *
 *   Tag 0x06 (Accelerometer):
 *     [0]=0x06 [1:2]=X_mg_LE [3:4]=Y_mg_LE [5:6]=Z_mg_LE [7:14]=reserved [15]=XOR
 *
 *   Tag 0x08 (Temperature):
 *     [0]=0x08 [1:2]=deviation_hundredths_C_LE [3:14]=reserved [15]=XOR
 *
 *   Tag 0x09 (Sleep Stage):
 *     [0]=0x09 [1]=stage [2:14]=reserved [15]=XOR
 *
 *   Tag 0x0A (Battery):
 *     [0]=0x0A [1]=% [2]=flags [3:14]=reserved [15]=XOR
 *
 *   Cmd 0x10 (Auth Response):
 *     [0]=0x10 [1:14]=encrypted_nonce_bytes [15]=XOR
 *
 *   Cmd 0x01 (SetFeatureMode, client → dongle):
 *     [0]=0x01 [1]=feature_bitmask [2:14]=reserved [15]=XOR
 *
 *   Cmd 0x02 (RequestLatest):
 *     [0]=0x02 [1:14]=feature_mask_to_request [15]=XOR
 *
 *   Cmd 0x03 (StartHistory):
 *     [0]=0x03 [1:14]=reserved [15]=XOR
 */

#ifndef PACKET_H
#define PACKET_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define OURA_PACKET_SIZE         16
#define OURA_PACKET_PAYLOAD_LEN  14
#define OURA_PACKET_CMD_OFFSET   0
#define OURA_PACKET_PAYLOAD_OFF  1
#define OURA_PACKET_XOR_OFFSET   15

/* ── Command / tag constants (mirrors OuraBLEProtocol.swift) ─── */

#define OURA_CMD_SET_FEATURE_MODE  0x01
#define OURA_CMD_REQUEST_LATEST    0x02
#define OURA_CMD_START_HISTORY     0x03
#define OURA_CMD_AUTH_RESPONSE     0x10
#define OURA_CMD_AUTH_CHALLENGE    0x11

/* Feature capability flags (payload[0] of cmdSetFeatureMode) */
#define OURA_CAP_DAYTIME_HR        0x01
#define OURA_CAP_SPO2              0x02
#define OURA_CAP_ACCELEROMETER     0x04
#define OURA_CAP_TEMPERATURE       0x08
#define OURA_CAP_CONNECTED_LIVE    0x80

/* Measurement tags */
#define OURA_TAG_LIVE_HR           0x2F
#define OURA_TAG_ACCELEROMETER     0x06
#define OURA_TAG_SPO2              0x07
#define OURA_TAG_TEMPERATURE       0x08
#define OURA_TAG_SLEEP_STAGE       0x09
#define OURA_TAG_BATTERY           0x0A

/* IBI sub-tag within a tagLiveHR packet */
#define OURA_SUBTAG_IBI            40

/* ── Functions ──────────────────────────────────────────────────── */

/**
 * Compute XOR checksum of first 15 bytes in buf.
 */
static inline uint8_t oura_xor_checksum(const uint8_t *buf) {
    uint8_t cs = 0;
    for (int i = 0; i < OURA_PACKET_XOR_OFFSET; i++) {
        cs ^= buf[i];
    }
    return cs;
}

/**
 * Validate a received packet: checksum byte must match XOR of bytes 0..14.
 * Returns true if valid.
 */
static inline bool oura_packet_validate(const uint8_t *data, uint16_t len) {
    if (len != OURA_PACKET_SIZE) return false;
    return data[OURA_PACKET_XOR_OFFSET] == oura_xor_checksum(data);
}

/**
 * Finalize a packet buffer by writing the XOR checksum into byte 15.
 */
static inline void oura_packet_finalize(uint8_t *buf) {
    buf[OURA_PACKET_XOR_OFFSET] = oura_xor_checksum(buf);
}

/**
 * Zero-fill the payload area of a packet (bytes 1..14).
 */
static inline void oura_packet_clear_payload(uint8_t *buf) {
    for (int i = OURA_PACKET_PAYLOAD_OFF; i < OURA_PACKET_XOR_OFFSET; i++) {
        buf[i] = 0;
    }
}

/**
 * Build a complete 16-byte packet in one call.
 *
 * @param buf    Output buffer (must be 16 bytes).
 * @param cmd    Command/tag byte (byte 0).
 * @param payload Payload bytes (exactly 14 bytes). If NULL, zero-filled.
 */
static inline void oura_packet_build(uint8_t *buf, uint8_t cmd, const uint8_t *payload) {
    buf[0] = cmd;
    if (payload) {
        for (int i = 0; i < OURA_PACKET_PAYLOAD_LEN; i++) {
            buf[OURA_PACKET_PAYLOAD_OFF + i] = payload[i];
        }
    } else {
        oura_packet_clear_payload(buf);
    }
    oura_packet_finalize(buf);
}

/**
 * Parse a SetFeatureMode command and extract the feature bitmask.
 * Returns the bitmask, or 0 if the packet is invalid.
 */
static inline uint8_t oura_packet_parse_feature_mode(const uint8_t *data, uint16_t len) {
    if (!oura_packet_validate(data, len)) return 0;
    if (data[OURA_PACKET_CMD_OFFSET] != OURA_CMD_SET_FEATURE_MODE) return 0;
    return data[OURA_PACKET_PAYLOAD_OFF]; /* feature bitmask in payload byte 0 */
}

#ifdef __cplusplus
}
#endif

#endif /* PACKET_H */
