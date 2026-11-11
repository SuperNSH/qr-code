//! A target with no camera bridge: the permission reads unasked, and no camera registers.

use std::{future::ready, sync::Arc};

use crate::{
    backend::{Backend, BackendFuture, PermissionState},
    error::{ErrorKind, QrError},
    frame::FrameSource,
};

pub(crate) struct NativeBackend;

impl Backend for NativeBackend {
    fn camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        Box::pin(ready(Ok::<PermissionState, QrError>(
            PermissionState::Undetermined,
        )))
    }

    fn request_camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        Box::pin(ready(Ok::<PermissionState, QrError>(
            PermissionState::Undetermined,
        )))
    }

    fn frame_source(&self) -> BackendFuture<'_, Result<Option<Arc<dyn FrameSource>>, QrError>> {
        Box::pin(ready(Err::<Option<Arc<dyn FrameSource>>, QrError>(
            QrError::new(ErrorKind::Unsupported),
        )))
    }
}
