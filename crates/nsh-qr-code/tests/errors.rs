//! Every kind's stable code and message. The TS package mirrors this table word for word.

use nsh_qr_code::{ErrorKind, QrError};

const ALL_KINDS: [(ErrorKind, &str, &str); 10] = [
    (
        ErrorKind::DecodeFailed,
        "ERR_QR_DECODE_FAILED",
        "A QR code was found but its payload could not be read. Try a clearer frame.",
    ),
    (
        ErrorKind::PayloadTooLarge,
        "ERR_QR_PAYLOAD_TOO_LARGE",
        "The payload does not fit a QR code. Shorten it or lower the error correction.",
    ),
    (
        ErrorKind::InvalidArgument,
        "ERR_QR_INVALID_ARGUMENT",
        "An argument is empty or not allowed, such as a frame whose size does not match its data.",
    ),
    (
        ErrorKind::PermissionDenied,
        "ERR_QR_PERMISSION_DENIED",
        "Camera permission was denied. Grant it in the system settings.",
    ),
    (
        ErrorKind::Unsupported,
        "ERR_QR_UNSUPPORTED",
        "No camera is registered on this platform yet; live frames come from the camera repo.",
    ),
    (
        ErrorKind::AlreadyRunning,
        "ERR_QR_ALREADY_RUNNING",
        "A scan is already running. Stop it before starting another.",
    ),
    (
        ErrorKind::Cancelled,
        "ERR_QR_CANCELLED",
        "The scan was cancelled.",
    ),
    (
        ErrorKind::Timeout,
        "ERR_QR_TIMEOUT",
        "The scan timed out. Try again.",
    ),
    (
        ErrorKind::FrameSourceClosed,
        "ERR_QR_FRAME_SOURCE_CLOSED",
        "The frame source stopped feeding frames.",
    ),
    (
        ErrorKind::Backend,
        "ERR_QR_BACKEND",
        "The platform or frame source failed unexpectedly.",
    ),
];

#[test]
fn every_kind_has_its_code_and_message() {
    for (kind, code, message) in ALL_KINDS {
        let error = QrError::new(kind);
        assert_eq!(error.kind(), kind);
        assert_eq!(error.code(), code);
        assert_eq!(error.message(), message);
        assert_eq!(error.status(), None);
        assert_eq!(error.clone().with_status(-7).status(), Some(-7));
        assert_eq!(error.with_status(-7).code(), code);
    }
}
