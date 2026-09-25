use std::fmt;
use std::io;

/// Errors that can occur while opening or parsing an ABF file.
///
/// Every variant is produced by validated, bounds-checked parsing: malformed
/// or truncated input always yields one of these instead of panicking.
#[derive(Debug)]
pub enum AbfError {
    /// Failed to open or read the file.
    Io(io::Error),
    /// The first 4 bytes are not a recognized ABF signature (`"ABF2"` / `"ABF "`).
    InvalidSignature,
    /// The file has a recognized but unsupported ABF version.
    UnsupportedVersion(String),
    /// A read ran past the end of the file.
    Truncated {
        section: &'static str,
        offset: usize,
    },
    /// A section's header fields are internally inconsistent (e.g. an
    /// offset/count that overflows or an unexpected sample width).
    InvalidSection {
        section: &'static str,
        reason: String,
    },
    /// The file uses a recognized but not-yet-supported feature.
    Unsupported(String),
    /// `Channel::read_sweep_into` was called with a sweep index that is out
    /// of range for the channel's sweep count.
    SweepOutOfRange { sweep: usize, sweeps_count: usize },
    /// `Channel::read_sweep_into`'s output buffer length didn't match the
    /// channel's sweep length.
    BufferLengthMismatch { expected: usize, actual: usize },
}

impl fmt::Display for AbfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AbfError::Io(e) => write!(f, "I/O error: {e}"),
            AbfError::InvalidSignature => {
                write!(f, "not an ABF file: unrecognized file signature")
            }
            AbfError::UnsupportedVersion(version) => {
                write!(f, "unsupported ABF version: {version}")
            }
            AbfError::Truncated { section, offset } => write!(
                f,
                "file truncated while reading {section} section at offset {offset}"
            ),
            AbfError::InvalidSection { section, reason } => {
                write!(f, "invalid {section} section: {reason}")
            }
            AbfError::Unsupported(reason) => write!(f, "unsupported: {reason}"),
            AbfError::SweepOutOfRange {
                sweep,
                sweeps_count,
            } => write!(
                f,
                "sweep index {sweep} out of range: channel has {sweeps_count} sweep(s)"
            ),
            AbfError::BufferLengthMismatch { expected, actual } => write!(
                f,
                "output buffer length {actual} does not match sweep length {expected}"
            ),
        }
    }
}

impl std::error::Error for AbfError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AbfError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for AbfError {
    fn from(e: io::Error) -> Self {
        AbfError::Io(e)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_every_variant() {
        assert_eq!(
            AbfError::Io(io::Error::new(io::ErrorKind::NotFound, "nope")).to_string(),
            "I/O error: nope"
        );
        assert_eq!(
            AbfError::InvalidSignature.to_string(),
            "not an ABF file: unrecognized file signature"
        );
        assert_eq!(
            AbfError::UnsupportedVersion("ABF1".to_string()).to_string(),
            "unsupported ABF version: ABF1"
        );
        assert_eq!(
            AbfError::Truncated {
                section: "adc",
                offset: 42
            }
            .to_string(),
            "file truncated while reading adc section at offset 42"
        );
        assert_eq!(
            AbfError::InvalidSection {
                section: "data",
                reason: "bad width".to_string()
            }
            .to_string(),
            "invalid data section: bad width"
        );
        assert_eq!(
            AbfError::Unsupported("variable-length event-driven sweeps".to_string()).to_string(),
            "unsupported: variable-length event-driven sweeps"
        );
        assert_eq!(
            AbfError::SweepOutOfRange {
                sweep: 5,
                sweeps_count: 3
            }
            .to_string(),
            "sweep index 5 out of range: channel has 3 sweep(s)"
        );
        assert_eq!(
            AbfError::BufferLengthMismatch {
                expected: 10,
                actual: 4
            }
            .to_string(),
            "output buffer length 4 does not match sweep length 10"
        );
    }

    #[test]
    fn from_io_error_wraps_it() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let err: AbfError = io_err.into();
        assert!(matches!(err, AbfError::Io(_)));
    }

    #[test]
    fn source_is_only_set_for_io_errors() {
        use std::error::Error;
        let io_err = AbfError::Io(io::Error::new(io::ErrorKind::NotFound, "nope"));
        assert!(io_err.source().is_some());
        assert!(AbfError::InvalidSignature.source().is_none());
    }
}
