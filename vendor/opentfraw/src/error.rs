use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid magic number: expected 0xa101, got 0x{0:04x}")]
    BadMagic(u16),

    #[error("invalid signature: expected \"Finnigan\", got {0:?}")]
    BadSignature(String),

    #[error("unsupported version: {0}")]
    UnsupportedVersion(u32),

    #[error("unexpected end of file at offset {offset:#x} (needed {needed} bytes)")]
    UnexpectedEof { offset: u64, needed: usize },

    #[error("invalid UTF-16 data at offset {0:#x}")]
    InvalidUtf16(u64),

    #[error("address out of range: {0:#x}")]
    AddressOutOfRange(u64),

    #[error("invalid generic data type code: {0}")]
    InvalidTypeCode(u32),

    #[error("operation not supported for this scan-data format: {0}")]
    UnsupportedOperation(&'static str),

    /// A structure the file cannot do without could not be read: which one, the format version, the file size and the reason.
    #[error("{stage} (format v{version}, file of {file_len} bytes): {source}")]
    Stage {
        stage: &'static str,
        version: u32,
        file_len: u64,
        source: Box<Error>,
    },

    /// The file ends before `stage` is complete: a copy or an acquisition that stopped early, with no scan readable (`readable`, of the `declared` ones, when known).
    #[error(
        "file cut at {file_len} bytes: {stage} needs data up to byte {needed} ({readable} of {declared} scans readable)"
    )]
    Truncated {
        stage: &'static str,
        file_len: u64,
        needed: u64,
        readable: u32,
        declared: u32,
    },

    #[error(
        "refusing to allocate for {requested} bytes at offset {offset:#x}: \
         only {available} bytes remain in the input"
    )]
    AllocationTooLarge {
        offset: u64,
        requested: u64,
        available: u64,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
