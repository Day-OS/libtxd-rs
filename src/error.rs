use std::io;

pub type Result<T> = std::result::Result<T, TxdError>;

#[derive(Debug, thiserror::Error)]
pub enum TxdError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("binary parse error: {0}")]
    Binary(String),

    #[error("expected {expected} chunk, got 0x{actual:08X}")]
    UnexpectedChunk { expected: &'static str, actual: u32 },

    #[error("unsupported platform 0x{0:08X}")]
    UnsupportedPlatform(u32),

    #[error("unsupported compression format")]
    UnsupportedCompression,

    #[error("invalid image dimensions {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("invalid palette data")]
    InvalidPalette,

    #[error("mipmap {0} is missing")]
    MissingMipmap(usize),

    #[error("root chunk is not a TXD dictionary")]
    NotTextureDictionary,
}

impl From<binrw::Error> for TxdError {
    fn from(err: binrw::Error) -> Self {
        Self::Binary(err.to_string())
    }
}

impl From<imagequant::Error> for TxdError {
    fn from(err: imagequant::Error) -> Self {
        Self::Binary(err.to_string())
    }
}
