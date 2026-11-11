//! QR codes for Native Shell apps.
//!
//! [`QrScanner`] scans frames for a QR code, [`generate`] makes one, and [`decode_frame`]
//! reads a single frame. Every call resolves exactly once: a value, a [`QrError`] with a
//! stable code, or a cancel. Live frames come from the camera repo (SuperNSH/nsh-camera)
//! through [`FrameSource`]; until a camera registers, scans answer `ERR_QR_UNSUPPORTED`.

mod backend;
mod decode;
mod error;
mod fake;
mod frame;
mod generate;
mod native;
mod options;
mod platform;
mod scanner;

use nsh_core::prelude::*;

pub use backend::{Backend, BackendFuture, PermissionState};
pub use decode::decode_frame;
pub use error::{ErrorKind, QrError};
pub use fake::{FakeBackend, FakeFrameSource};
pub use frame::{Frame, FrameFuture, FrameSource};
pub use generate::generate;
pub use native::NativeQrCode;
pub use options::{DecodedQr, EccLevel, GenerateOptions, QrMatrix, ScanOptions};
pub use scanner::QrScanner;

/// The crate's UI set: what TSX imports from `nsh-qr-code/native`.
#[native_module("nsh-qr-code/native")]
fn native(registry: &mut Registry) {
    registry.class::<NativeQrCode>();
}
