//! The errors a call reports. Each has a stable code; a message says what to do.

use std::fmt;

/// What went wrong, as a stable code; see [`QrError::code`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// A QR code was found but its payload could not be read: blur, glare or damage.
    DecodeFailed,
    /// The payload does not fit a QR code, even at the lowest correction.
    PayloadTooLarge,
    /// An argument is empty or not allowed, such as a frame whose size does not match its data.
    InvalidArgument,
    /// The camera permission was denied.
    PermissionDenied,
    /// No camera is registered on this platform yet; live frames come from the camera repo.
    Unsupported,
    /// A scan is already running. One scan runs at a time.
    AlreadyRunning,
    /// The scan was cancelled by a `stop` call.
    Cancelled,
    /// The scan gave up waiting: `ScanOptions::timeout` passed with no code found.
    Timeout,
    /// The frame source stopped feeding frames.
    FrameSourceClosed,
    /// The platform or frame source failed; `status` holds its own code where it gives one.
    Backend,
}

/// A failed call.
///
/// `Display` prints the stable code. Neither the error nor its message holds user data.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}", .kind.code())]
pub struct QrError {
    kind: ErrorKind,
    status: Option<i64>,
}

impl QrError {
    /// An error of `kind`, with no platform status.
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, status: None }
    }

    /// The same error, carrying the platform's own status code.
    #[must_use]
    pub fn with_status(mut self, status: i64) -> Self {
        self.status = Some(status);
        self
    }

    /// What went wrong.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The platform's status code, when the platform gave one.
    pub fn status(&self) -> Option<i64> {
        self.status
    }

    /// The stable code, such as `ERR_QR_CANCELLED`. Apps match on this.
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// What to do about it, in one sentence.
    pub fn message(&self) -> &'static str {
        self.kind.message()
    }
}

impl ErrorKind {
    /// The stable code of this kind.
    pub fn code(self) -> &'static str {
        match self {
            ErrorKind::DecodeFailed => "ERR_QR_DECODE_FAILED",
            ErrorKind::PayloadTooLarge => "ERR_QR_PAYLOAD_TOO_LARGE",
            ErrorKind::InvalidArgument => "ERR_QR_INVALID_ARGUMENT",
            ErrorKind::PermissionDenied => "ERR_QR_PERMISSION_DENIED",
            ErrorKind::Unsupported => "ERR_QR_UNSUPPORTED",
            ErrorKind::AlreadyRunning => "ERR_QR_ALREADY_RUNNING",
            ErrorKind::Cancelled => "ERR_QR_CANCELLED",
            ErrorKind::Timeout => "ERR_QR_TIMEOUT",
            ErrorKind::FrameSourceClosed => "ERR_QR_FRAME_SOURCE_CLOSED",
            ErrorKind::Backend => "ERR_QR_BACKEND",
        }
    }

    /// What to do about it, in one sentence.
    pub fn message(self) -> &'static str {
        match self {
            ErrorKind::DecodeFailed => {
                "A QR code was found but its payload could not be read. Try a clearer frame."
            }
            ErrorKind::PayloadTooLarge => {
                "The payload does not fit a QR code. Shorten it or lower the error correction."
            }
            ErrorKind::InvalidArgument => {
                "An argument is empty or not allowed, such as a frame whose size does not match \
                 its data."
            }
            ErrorKind::PermissionDenied => {
                "Camera permission was denied. Grant it in the system settings."
            }
            ErrorKind::Unsupported => {
                "No camera is registered on this platform yet; live frames come from the camera \
                 repo."
            }
            ErrorKind::AlreadyRunning => {
                "A scan is already running. Stop it before starting another."
            }
            ErrorKind::Cancelled => "The scan was cancelled.",
            ErrorKind::Timeout => "The scan timed out. Try again.",
            ErrorKind::FrameSourceClosed => "The frame source stopped feeding frames.",
            ErrorKind::Backend => "The platform or frame source failed unexpectedly.",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_KINDS: [(ErrorKind, &str); 10] = [
        (ErrorKind::DecodeFailed, "ERR_QR_DECODE_FAILED"),
        (ErrorKind::PayloadTooLarge, "ERR_QR_PAYLOAD_TOO_LARGE"),
        (ErrorKind::InvalidArgument, "ERR_QR_INVALID_ARGUMENT"),
        (ErrorKind::PermissionDenied, "ERR_QR_PERMISSION_DENIED"),
        (ErrorKind::Unsupported, "ERR_QR_UNSUPPORTED"),
        (ErrorKind::AlreadyRunning, "ERR_QR_ALREADY_RUNNING"),
        (ErrorKind::Cancelled, "ERR_QR_CANCELLED"),
        (ErrorKind::Timeout, "ERR_QR_TIMEOUT"),
        (ErrorKind::FrameSourceClosed, "ERR_QR_FRAME_SOURCE_CLOSED"),
        (ErrorKind::Backend, "ERR_QR_BACKEND"),
    ];

    #[test]
    fn every_kind_has_its_code_and_a_message() {
        for (kind, code) in ALL_KINDS {
            let error = QrError::new(kind);
            assert_eq!(error.code(), code);
            assert_eq!(error.to_string(), code, "Display prints the code");
            assert_eq!(kind.to_string(), code);
            assert_eq!(error.message(), kind.message());
            assert!(!kind.message().is_empty());
        }
    }

    #[test]
    fn a_platform_status_is_kept_and_replaced() {
        let error = QrError::new(ErrorKind::Backend).with_status(-7);
        assert_eq!(error.kind(), ErrorKind::Backend);
        assert_eq!(error.status(), Some(-7));
        assert_eq!(error.with_status(3).status(), Some(3));
        assert_eq!(QrError::new(ErrorKind::Timeout).status(), None);
    }
}
