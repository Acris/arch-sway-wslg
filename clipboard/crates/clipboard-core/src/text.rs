use thiserror::Error;

use crate::MAX_TEXT_BYTES;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum TextError {
    #[error("clipboard text contains an embedded NUL")]
    EmbeddedNul,
    #[error("clipboard text is {actual} bytes, limit is {limit}")]
    TooLarge { actual: usize, limit: usize },
    #[error("clipboard text is not valid UTF-8")]
    InvalidUtf8,
    #[error("clipboard text is not valid UTF-16")]
    InvalidUtf16,
}

pub fn validate_utf8(bytes: &[u8]) -> Result<&str, TextError> {
    if bytes.len() > MAX_TEXT_BYTES {
        return Err(TextError::TooLarge {
            actual: bytes.len(),
            limit: MAX_TEXT_BYTES,
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| TextError::InvalidUtf8)?;
    if text.contains('\0') {
        return Err(TextError::EmbeddedNul);
    }
    Ok(text)
}

// Both conversions run over bytes in one pass: CR and LF never appear inside a
// multi-byte UTF-8 sequence, so the output stays valid UTF-8 and a 16 MiB text
// is copied once instead of once per `replace`.
#[must_use]
pub fn windows_to_unix_newlines(text: &str) -> String {
    convert_newlines(text, b"\n")
}

#[must_use]
pub fn unix_to_windows_newlines(text: &str) -> String {
    convert_newlines(text, b"\r\n")
}

fn convert_newlines(text: &str, newline: &[u8]) -> String {
    let bytes = text.as_bytes();
    let mut output = Vec::with_capacity(bytes.len() + bytes.len() / 16);
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' => {
                output.extend_from_slice(newline);
                if bytes.get(index + 1) == Some(&b'\n') {
                    index += 1;
                }
            }
            b'\n' => output.extend_from_slice(newline),
            byte => output.push(byte),
        }
        index += 1;
    }
    String::from_utf8(output).expect("newline conversion preserves UTF-8")
}

pub fn utf16_to_utf8(wide: &[u16]) -> Result<Vec<u8>, TextError> {
    let text = String::from_utf16(wide).map_err(|_| TextError::InvalidUtf16)?;
    if text.contains('\0') {
        return Err(TextError::EmbeddedNul);
    }
    let normalized = windows_to_unix_newlines(&text).into_bytes();
    if normalized.len() > MAX_TEXT_BYTES {
        return Err(TextError::TooLarge {
            actual: normalized.len(),
            limit: MAX_TEXT_BYTES,
        });
    }
    Ok(normalized)
}

pub fn utf8_to_utf16(bytes: &[u8]) -> Result<Vec<u16>, TextError> {
    let text = validate_utf8(bytes)?;
    let mut wide: Vec<u16> = unix_to_windows_newlines(text).encode_utf16().collect();
    wide.push(0);
    Ok(wide)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_newlines_in_both_directions() {
        assert_eq!(windows_to_unix_newlines("a\r\nb\rc\n"), "a\nb\nc\n");
        assert_eq!(unix_to_windows_newlines("a\r\nb\rc\n"), "a\r\nb\r\nc\r\n");
    }

    #[test]
    fn utf_round_trip_preserves_unicode() {
        let input = "中文 😀\nsecond".as_bytes();
        let wide = utf8_to_utf16(input).unwrap();
        assert_eq!(utf16_to_utf8(&wide[..wide.len() - 1]).unwrap(), input);
    }

    #[test]
    fn rejects_embedded_nul() {
        assert_eq!(validate_utf8(b"a\0b"), Err(TextError::EmbeddedNul));
    }
}
