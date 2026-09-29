//! `AirSense` 11 FIG framing.
//!
//! FIG bytes arrive in attacker-controlled BLE notifications. This module is
//! therefore allocation-free and bounded: a length field never controls an
//! allocation, incomplete input yields `NeedMore`, and CRC failures discard
//! only the candidate frame so a later sync word can still be found.

/// FIG sync bytes for protocol integer `0xCAFEBABE`.
pub const FIG_SYNC: [u8; 4] = [0xBE, 0xBA, 0xFE, 0xCA];
/// Bytes before a FIG payload.
pub const FIG_PREFIX_LENGTH: usize = 16;
/// Maximum payload documented by the `AirSense` BLE protocol.
pub const FIG_MAX_PAYLOAD: usize = 7682;
const FIG_MAX_PACKET: usize = FIG_PREFIX_LENGTH + FIG_MAX_PAYLOAD;
const FIG_BUFFER_LENGTH: usize = FIG_MAX_PACKET + FIG_SYNC.len() - 1;

/// A complete FIG packet borrowed from the input or reassembler buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FigPacket<'a> {
    /// The protocol VCID.
    pub vcid: u16,
    /// The payload, without the FIG prefix.
    pub payload: &'a [u8],
    /// The number of bytes occupied by this packet in the source buffer.
    pub encoded_len: usize,
}

/// Why a reassembler did not return a packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigTake<'a> {
    /// More notification bytes are required.
    NeedMore,
    /// A complete, CRC-verified packet.
    Packet(FigPacket<'a>),
    /// The eight-byte header body failed its CRC.
    HeaderCrcMismatch,
    /// The advertised payload exceeded [`FIG_MAX_PAYLOAD`].
    PayloadTooLarge,
    /// The payload failed its CRC.
    PayloadCrcMismatch,
    /// The fragment could not fit in the bounded reassembly buffer.
    BufferFull,
}

/// Fixed-capacity FIG fragment reassembler.
#[derive(Debug)]
pub struct FigReassembler {
    buffer: [u8; FIG_BUFFER_LENGTH],
    begin: usize,
    end: usize,
}

impl FigReassembler {
    /// Creates an empty reassembler.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [0; FIG_BUFFER_LENGTH],
            begin: 0,
            end: 0,
        }
    }

    /// Adds one arbitrary BLE notification fragment.
    ///
    /// Returns `false` when the bounded buffer cannot retain the fragment. The
    /// caller should report that named transport loss and reset the link rather
    /// than dropping bytes silently.
    pub fn feed(&mut self, fragment: &[u8]) -> bool {
        if fragment.len() > FIG_MAX_PACKET {
            return false;
        }
        let retained = self.end.saturating_sub(self.begin);
        if fragment.len() > FIG_BUFFER_LENGTH.saturating_sub(retained) {
            self.compact();
        }
        let Some(new_end) = self.end.checked_add(fragment.len()) else {
            return false;
        };
        if new_end > FIG_BUFFER_LENGTH {
            return false;
        }
        let Some(destination) = self.buffer.get_mut(self.end..new_end) else {
            return false;
        };
        destination.copy_from_slice(fragment);
        self.end = new_end;
        true
    }

    /// Takes the next packet or protocol result.
    ///
    /// A returned packet borrows the reassembler until the next mutation. A
    /// malformed candidate advances past its sync bytes so a later candidate
    /// can be recovered from the same notification stream.
    pub fn take(&mut self) -> FigTake<'_> {
        let Some(sync) = find_sync(&self.buffer, self.begin, self.end) else {
            self.retain_sync_suffix();
            return FigTake::NeedMore;
        };
        self.begin = sync;
        if self.end.saturating_sub(self.begin) < FIG_PREFIX_LENGTH {
            return FigTake::NeedMore;
        }

        let Some(header_body) = self.buffer.get(self.begin + 4..self.begin + 12) else {
            return FigTake::NeedMore;
        };
        let Some(header_crc) = read_u32(&self.buffer, self.begin + 12) else {
            return FigTake::NeedMore;
        };
        if crc32_ieee(header_body) != header_crc {
            self.begin = self.begin.saturating_add(FIG_SYNC.len());
            return FigTake::HeaderCrcMismatch;
        }

        let Some(payload_length) = read_u16(&self.buffer, self.begin + 6) else {
            return FigTake::NeedMore;
        };
        let payload_length = usize::from(payload_length);
        if payload_length > FIG_MAX_PAYLOAD {
            self.begin = self.begin.saturating_add(FIG_SYNC.len());
            return FigTake::PayloadTooLarge;
        }
        let Some(packet_length) = FIG_PREFIX_LENGTH.checked_add(payload_length) else {
            self.begin = self.begin.saturating_add(FIG_SYNC.len());
            return FigTake::PayloadTooLarge;
        };
        if self.end.saturating_sub(self.begin) < packet_length {
            return FigTake::NeedMore;
        }
        let payload_begin = self.begin + FIG_PREFIX_LENGTH;
        let Some(payload_end) = payload_begin.checked_add(payload_length) else {
            self.begin = self.begin.saturating_add(FIG_SYNC.len());
            return FigTake::PayloadTooLarge;
        };
        let Some(payload) = self.buffer.get(payload_begin..payload_end) else {
            return FigTake::NeedMore;
        };
        let Some(payload_crc) = read_u32(&self.buffer, self.begin + 8) else {
            return FigTake::NeedMore;
        };
        if crc32_ieee(payload) != payload_crc {
            self.begin = self.begin.saturating_add(FIG_SYNC.len());
            return FigTake::PayloadCrcMismatch;
        }
        let Some(vcid) = read_u16(&self.buffer, self.begin + 4) else {
            return FigTake::NeedMore;
        };
        let packet_begin = self.begin;
        self.begin = packet_begin.saturating_add(packet_length);
        FigTake::Packet(FigPacket {
            vcid,
            payload,
            encoded_len: packet_length,
        })
    }

    /// Drops all buffered bytes after a transport reset.
    pub const fn clear(&mut self) {
        self.begin = 0;
        self.end = 0;
    }

    fn compact(&mut self) {
        if self.begin == 0 {
            return;
        }
        let retained = self.end.saturating_sub(self.begin);
        if retained != 0 {
            self.buffer.copy_within(self.begin..self.end, 0);
        }
        self.begin = 0;
        self.end = retained;
    }

    fn retain_sync_suffix(&mut self) {
        let retained = core::cmp::min(
            self.end.saturating_sub(self.begin),
            FIG_SYNC.len().saturating_sub(1),
        );
        if retained != 0 {
            let from = self.end.saturating_sub(retained);
            self.buffer.copy_within(from..self.end, 0);
        }
        self.begin = 0;
        self.end = retained;
    }
}

impl Default for FigReassembler {
    fn default() -> Self {
        Self::new()
    }
}

/// Decodes one FIG packet at the beginning of `buf`.
///
/// The input may contain subsequent bytes; [`FigPacket::encoded_len`] tells the
/// caller where the next packet begins. No bytes are allocated or indexed based
/// on the advertised length.
#[must_use]
pub fn decode_fig(buf: &[u8]) -> Option<FigPacket<'_>> {
    if buf.get(..FIG_SYNC.len())? != FIG_SYNC {
        return None;
    }
    if buf.len() < FIG_PREFIX_LENGTH {
        return None;
    }
    let header_body = buf.get(4..12)?;
    let header_crc = read_u32(buf, 12)?;
    if crc32_ieee(header_body) != header_crc {
        return None;
    }
    let payload_length = usize::from(read_u16(buf, 6)?);
    if payload_length > FIG_MAX_PAYLOAD {
        return None;
    }
    let packet_length = FIG_PREFIX_LENGTH.checked_add(payload_length)?;
    let payload_end = FIG_PREFIX_LENGTH.checked_add(payload_length)?;
    let payload = buf.get(FIG_PREFIX_LENGTH..payload_end)?;
    let payload_crc = read_u32(buf, 8)?;
    if crc32_ieee(payload) != payload_crc {
        return None;
    }
    Some(FigPacket {
        vcid: read_u16(buf, 4)?,
        payload,
        encoded_len: packet_length,
    })
}

fn find_sync(buf: &[u8], begin: usize, end: usize) -> Option<usize> {
    buf.get(begin..end)?
        .windows(FIG_SYNC.len())
        .position(|window| window == FIG_SYNC)
        .and_then(|offset| begin.checked_add(offset))
}

fn read_u16(buf: &[u8], offset: usize) -> Option<u16> {
    let bytes: [u8; 2] = buf.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

fn read_u32(buf: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = buf.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::vec::Vec;

    fn frame(vcid: u16, payload: &[u8]) -> Vec<u8> {
        let payload_crc = crc32_ieee(payload);
        let mut header = Vec::new();
        header.extend_from_slice(&vcid.to_le_bytes());
        header.extend_from_slice(
            &u16::try_from(payload.len())
                .expect("test payload fits")
                .to_le_bytes(),
        );
        header.extend_from_slice(&payload_crc.to_le_bytes());
        let header_crc = crc32_ieee(&header);
        let mut out = Vec::new();
        out.extend_from_slice(&FIG_SYNC);
        out.extend_from_slice(&header);
        out.extend_from_slice(&header_crc.to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn decodes_a_valid_packet_and_reports_consumed_length() {
        let bytes = frame(0x0396, br#"{"method":"StreamData"}"#);
        let packet = decode_fig(&bytes).expect("valid FIG");
        assert_eq!(packet.vcid, 0x0396);
        assert_eq!(packet.payload, br#"{"method":"StreamData"}"#);
        assert_eq!(packet.encoded_len, bytes.len());
    }

    #[test]
    fn accepts_a_second_packet_after_the_first() {
        let mut bytes = frame(0x0392, b"one");
        let second = frame(0x0396, b"two");
        bytes.extend_from_slice(&second);
        let first = decode_fig(&bytes).expect("first FIG");
        assert_eq!(first.payload, b"one");
        let next =
            decode_fig(bytes.get(first.encoded_len..).expect("second slice")).expect("second FIG");
        assert_eq!(next.payload, b"two");
    }

    #[test]
    fn reassembles_arbitrary_fragment_boundaries() {
        let bytes = frame(0x0396, b"fragmented payload");
        let mut reassembler = FigReassembler::new();
        for fragment in bytes.chunks(3) {
            assert!(reassembler.feed(fragment));
            if let FigTake::Packet(packet) = reassembler.take() {
                assert_eq!(packet.vcid, 0x0396);
                assert_eq!(packet.payload, b"fragmented payload");
                return;
            }
        }
        panic!("fragmented FIG was not emitted");
    }

    #[test]
    fn emits_multiple_packets_from_one_fragment() {
        let mut bytes = frame(0x0392, b"one");
        bytes.extend_from_slice(&frame(0x0396, b"two"));
        let mut reassembler = FigReassembler::new();
        assert!(reassembler.feed(&bytes));
        let first = match reassembler.take() {
            FigTake::Packet(packet) => packet,
            other => panic!("expected first packet, got {other:?}"),
        };
        assert_eq!(first.payload, b"one");
        let second = match reassembler.take() {
            FigTake::Packet(packet) => packet,
            other => panic!("expected second packet, got {other:?}"),
        };
        assert_eq!(second.payload, b"two");
    }

    #[test]
    fn bad_header_and_payload_crc_are_rejected() {
        let mut header_bad = frame(0x0396, b"payload");
        if let Some(byte) = header_bad.get_mut(4) {
            *byte ^= 1;
        }
        assert_eq!(decode_fig(&header_bad), None);
        let mut payload_bad = frame(0x0396, b"payload");
        if let Some(byte) = payload_bad.get_mut(FIG_PREFIX_LENGTH) {
            *byte ^= 1;
        }
        assert_eq!(decode_fig(&payload_bad), None);
    }

    #[test]
    fn truncation_and_oversize_never_decode() {
        let full = frame(0x0396, b"payload");
        for len in 0..full.len() {
            assert_eq!(decode_fig(full.get(..len).expect("prefix exists")), None);
        }
        let mut oversized = std::vec![0; FIG_PREFIX_LENGTH];
        oversized[..FIG_SYNC.len()].copy_from_slice(&FIG_SYNC);
        oversized
            .get_mut(6..8)
            .expect("length field")
            .copy_from_slice(
                &u16::try_from(FIG_MAX_PAYLOAD + 1)
                    .expect("fits u16")
                    .to_le_bytes(),
            );
        assert_eq!(decode_fig(&oversized), None);
    }

    #[test]
    fn noise_and_a_split_sync_are_retained() {
        let bytes = frame(0x0396, b"ok");
        let mut reassembler = FigReassembler::new();
        assert!(reassembler.feed(b"noise\xbe"));
        assert!(matches!(reassembler.take(), FigTake::NeedMore));
        assert!(reassembler.feed(bytes.get(1..).expect("split after sync byte")));
        match reassembler.take() {
            FigTake::Packet(packet) => assert_eq!(packet.payload, b"ok"),
            other => panic!("expected recovered packet, got {other:?}"),
        }
    }

    #[test]
    fn reassembler_reports_bounded_transport_and_crc_errors() {
        let mut oversized = FigReassembler::new();
        assert!(!oversized.feed(&std::vec![0; FIG_MAX_PACKET + 1]));
        assert!(oversized.feed(&std::vec![0; FIG_MAX_PACKET]));
        assert!(oversized.feed(&std::vec![0; FIG_BUFFER_LENGTH - FIG_MAX_PACKET]));
        assert!(!oversized.feed(b"x"));
        oversized.clear();
        assert!(matches!(oversized.take(), FigTake::NeedMore));

        let mut header_bad = frame(0x0396, b"payload");
        *header_bad.get_mut(4).expect("vcid byte") ^= 1;
        assert!(oversized.feed(&header_bad));
        assert!(matches!(oversized.take(), FigTake::HeaderCrcMismatch));
        oversized.clear();

        let mut payload_bad = frame(0x0396, b"payload");
        *payload_bad.get_mut(FIG_PREFIX_LENGTH).expect("payload byte") ^= 1;
        assert!(oversized.feed(&payload_bad));
        assert!(matches!(oversized.take(), FigTake::PayloadCrcMismatch));
        oversized.clear();

        let payload_crc = crc32_ieee(b"");
        let mut header = Vec::new();
        header.extend_from_slice(&0x0396_u16.to_le_bytes());
        header.extend_from_slice(&u16::try_from(FIG_MAX_PAYLOAD + 1).expect("fits").to_le_bytes());
        header.extend_from_slice(&payload_crc.to_le_bytes());
        let header_crc = crc32_ieee(&header);
        let mut too_large = Vec::from(FIG_SYNC);
        too_large.extend_from_slice(&header);
        too_large.extend_from_slice(&header_crc.to_le_bytes());
        assert!(oversized.feed(&too_large));
        assert!(matches!(oversized.take(), FigTake::PayloadTooLarge));
    }

    #[test]
    fn compaction_and_low_level_helpers_are_bounded() {
        let mut reassembler = FigReassembler::default();
        reassembler.buffer[1..3].copy_from_slice(b"ab");
        reassembler.begin = 1;
        reassembler.end = 3;
        reassembler.compact();
        assert_eq!(&reassembler.buffer[..2], b"ab");
        assert_eq!(reassembler.begin, 0);
        assert_eq!(reassembler.end, 2);
        reassembler.buffer[..3].copy_from_slice(&FIG_SYNC[..3]);
        reassembler.begin = 0;
        reassembler.end = 3;
        reassembler.retain_sync_suffix();
        assert_eq!(reassembler.end, 3);
        assert_eq!(find_sync(&reassembler.buffer, 0, reassembler.end), None);
        assert_eq!(read_u16(&[1, 2], 0), Some(0x0201));
        assert_eq!(read_u16(&[1], 0), None);
        assert_eq!(read_u32(&[1, 2, 3, 4], 0), Some(0x0403_0201));
        assert_eq!(read_u32(&[1, 2, 3], 0), None);
        assert_eq!(crc32_ieee(b""), 0);
    }
}
