pub mod protocol;
pub mod state;
pub mod text;

pub const PROTOCOL_VERSION: u16 = 3;
pub const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;
