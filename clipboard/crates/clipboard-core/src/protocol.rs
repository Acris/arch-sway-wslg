use std::io::{self, Read, Write};

use thiserror::Error;

use crate::{MAX_TEXT_BYTES, PROTOCOL_VERSION};

const MAGIC: [u8; 4] = *b"ASWC";
const HEADER_LEN: usize = 28;
const MAX_CONTROL_BYTES: usize = 1024;

pub const HELLO_HAS_TEXT: u32 = 1;
/// The agent could not obtain a stable startup snapshot.
pub const HELLO_READ_ERROR: u32 = 1 << 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum MessageKind {
    Hello = 1,
    WindowsText = 2,
    SetWindowsText = 3,
    SetWindowsOk = 4,
    SetWindowsError = 5,
    Ping = 6,
    Pong = 7,
    ProtocolError = 8,
    WindowsUnavailable = 9,
}

impl MessageKind {
    const fn payload_limit(self) -> usize {
        match self {
            Self::Hello | Self::WindowsText | Self::SetWindowsText => MAX_TEXT_BYTES,
            _ => MAX_CONTROL_BYTES,
        }
    }
}

impl TryFrom<u16> for MessageKind {
    type Error = ProtocolError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::WindowsText),
            3 => Ok(Self::SetWindowsText),
            4 => Ok(Self::SetWindowsOk),
            5 => Ok(Self::SetWindowsError),
            6 => Ok(Self::Ping),
            7 => Ok(Self::Pong),
            8 => Ok(Self::ProtocolError),
            9 => Ok(Self::WindowsUnavailable),
            other => Err(ProtocolError::UnknownKind(other)),
        }
    }
}

/// One message on the broker <-> agent pipe.
///
/// `sequence` is the Windows clipboard sequence number the frame describes; the
/// agent fills it in and the broker never sends a meaningful value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub kind: MessageKind,
    pub request_id: u64,
    pub sequence: u32,
    pub flags: u32,
    pub payload: Vec<u8>,
}

struct Header {
    kind: MessageKind,
    request_id: u64,
    sequence: u32,
    flags: u32,
    payload_len: usize,
}

impl Header {
    fn parse(header: &[u8; HEADER_LEN]) -> Result<Self, ProtocolError> {
        if header[..4] != MAGIC {
            return Err(ProtocolError::BadMagic);
        }
        let version = u16::from_le_bytes([header[4], header[5]]);
        if version != PROTOCOL_VERSION {
            return Err(ProtocolError::Version(version));
        }
        let kind = MessageKind::try_from(u16::from_le_bytes([header[6], header[7]]))?;
        let request_id = u64::from_le_bytes(header[8..16].try_into().expect("fixed header"));
        let sequence = u32::from_le_bytes(header[16..20].try_into().expect("fixed header"));
        let payload_len =
            u32::from_le_bytes(header[20..24].try_into().expect("fixed header")) as usize;
        let flags = u32::from_le_bytes(header[24..28].try_into().expect("fixed header"));
        let limit = kind.payload_limit();
        if payload_len > limit {
            return Err(ProtocolError::PayloadTooLarge {
                actual: payload_len,
                limit,
            });
        }
        Ok(Self {
            kind,
            request_id,
            sequence,
            flags,
            payload_len,
        })
    }

    fn into_frame(self, payload: Vec<u8>) -> Frame {
        Frame {
            kind: self.kind,
            request_id: self.request_id,
            sequence: self.sequence,
            flags: self.flags,
            payload,
        }
    }
}

impl Frame {
    #[must_use]
    pub fn new(kind: MessageKind) -> Self {
        Self {
            kind,
            request_id: 0,
            sequence: 0,
            flags: 0,
            payload: Vec::new(),
        }
    }

    pub fn read_from(reader: &mut impl Read) -> Result<Self, ProtocolError> {
        let mut header = [0_u8; HEADER_LEN];
        reader.read_exact(&mut header)?;
        let header = Header::parse(&header)?;
        let mut payload = vec![0_u8; header.payload_len];
        reader.read_exact(&mut payload)?;
        Ok(header.into_frame(payload))
    }

    pub fn write_to(&self, writer: &mut impl Write) -> Result<(), ProtocolError> {
        let limit = self.kind.payload_limit();
        if self.payload.len() > limit {
            return Err(ProtocolError::PayloadTooLarge {
                actual: self.payload.len(),
                limit,
            });
        }
        let payload_len = u32::try_from(self.payload.len()).expect("payload within limit");

        let mut header = [0_u8; HEADER_LEN];
        header[..4].copy_from_slice(&MAGIC);
        header[4..6].copy_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        header[6..8].copy_from_slice(&(self.kind as u16).to_le_bytes());
        header[8..16].copy_from_slice(&self.request_id.to_le_bytes());
        header[16..20].copy_from_slice(&self.sequence.to_le_bytes());
        header[20..24].copy_from_slice(&payload_len.to_le_bytes());
        header[24..28].copy_from_slice(&self.flags.to_le_bytes());
        writer.write_all(&header)?;
        writer.write_all(&self.payload)?;
        writer.flush()?;
        Ok(())
    }
}

/// Incremental decoder for a non-blocking byte stream of frames.
///
/// The header is validated before its payload is buffered, so the buffer never
/// grows beyond one frame's declared size plus the caller's read chunk, and a
/// completed frame takes its allocation with it instead of pinning it here.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
}

impl FrameDecoder {
    pub fn extend(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn next_frame(&mut self) -> Result<Option<Frame>, ProtocolError> {
        if self.buffer.len() < HEADER_LEN {
            return Ok(None);
        }
        let header = Header::parse(self.buffer[..HEADER_LEN].try_into().expect("fixed header"))?;
        let total = HEADER_LEN + header.payload_len;
        if self.buffer.len() < total {
            return Ok(None);
        }
        // Splitting moves the payload once; a drain would copy it and then shift
        // whatever follows it.
        let mut payload = self.buffer.split_off(HEADER_LEN);
        self.buffer = payload.split_off(header.payload_len);
        Ok(Some(header.into_frame(payload)))
    }
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("invalid protocol magic")]
    BadMagic,
    #[error("unsupported protocol version {0}")]
    Version(u16),
    #[error("unknown message kind {0}")]
    UnknownKind(u16),
    #[error("payload is {actual} bytes, limit is {limit}")]
    PayloadTooLarge { actual: usize, limit: usize },
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn sample_frame() -> Frame {
        Frame {
            kind: MessageKind::SetWindowsText,
            request_id: 23,
            sequence: 42,
            flags: 7,
            payload: "hello 世界".as_bytes().to_vec(),
        }
    }

    #[test]
    fn frame_round_trip() {
        let frame = sample_frame();
        let mut encoded = Vec::new();
        frame.write_to(&mut encoded).unwrap();
        assert_eq!(Frame::read_from(&mut Cursor::new(encoded)).unwrap(), frame);
    }

    #[test]
    fn decoder_reassembles_split_frames() {
        let frame = sample_frame();
        let mut encoded = Vec::new();
        frame.write_to(&mut encoded).unwrap();
        frame.write_to(&mut encoded).unwrap();

        let mut decoder = FrameDecoder::default();
        let (head, tail) = encoded.split_at(HEADER_LEN + 3);
        decoder.extend(head);
        assert_eq!(decoder.next_frame().unwrap(), None);
        decoder.extend(tail);
        assert_eq!(decoder.next_frame().unwrap(), Some(frame.clone()));
        assert_eq!(decoder.next_frame().unwrap(), Some(frame));
        assert_eq!(decoder.next_frame().unwrap(), None);
    }

    #[test]
    fn rejects_unknown_version_before_allocating() {
        let mut encoded = vec![0_u8; HEADER_LEN];
        encoded[..4].copy_from_slice(&MAGIC);
        encoded[4..6].copy_from_slice(&(PROTOCOL_VERSION + 1).to_le_bytes());
        assert!(matches!(
            Frame::read_from(&mut Cursor::new(encoded)),
            Err(ProtocolError::Version(_))
        ));
    }

    #[test]
    fn rejects_oversized_text_before_reading_payload() {
        let mut encoded = vec![0_u8; HEADER_LEN];
        encoded[..4].copy_from_slice(&MAGIC);
        encoded[4..6].copy_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        encoded[6..8].copy_from_slice(&(MessageKind::WindowsText as u16).to_le_bytes());
        encoded[20..24].copy_from_slice(&((MAX_TEXT_BYTES + 1) as u32).to_le_bytes());
        assert!(matches!(
            Frame::read_from(&mut Cursor::new(encoded.clone())),
            Err(ProtocolError::PayloadTooLarge { .. })
        ));
        let mut decoder = FrameDecoder::default();
        decoder.extend(&encoded);
        assert!(matches!(
            decoder.next_frame(),
            Err(ProtocolError::PayloadTooLarge { .. })
        ));
    }
}
