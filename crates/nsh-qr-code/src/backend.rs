//! The seam between `QrScanner` and a platform: the camera permission, and its frame source.

use std::{future::Future, pin::Pin, sync::Arc};

use crate::{error::QrError, frame::FrameSource};

/// A future a backend returns. Each call's future borrows the backend.
pub type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Where the camera permission stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionState {
    /// Not asked yet, or the platform does not say.
    Undetermined,
    /// The app may use the camera.
    Granted,
    /// The app may not use the camera; the user must change it in the settings.
    Denied,
}

/// What a platform provides. [`crate::QrScanner`] adds the rules: one scan at a time, one
/// answer per call.
pub trait Backend: Send + Sync {
    /// The camera permission as it stands, without asking.
    fn camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>>;

    /// Asks for the camera permission and reports how it ended.
    fn request_camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>>;

    /// The camera's frame source, or `Ok(None)` when no camera is registered yet: live frames
    /// arrive with SuperNSH/nsh-camera, which registers one through `FrameSource`.
    fn frame_source(&self) -> BackendFuture<'_, Result<Option<Arc<dyn FrameSource>>, QrError>>;
}
