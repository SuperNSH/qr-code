//! Windows: no camera-permission broker answers an unpackaged app, so permission calls report
//! `Granted` and the Settings privacy toggle does its work when capture opens.
//!
//! The references agree there is nothing to bridge: mobile_scanner marks Windows camera
//! scanning unsupported, and react-native-vision-camera ships React Native's Android and iOS
//! only.

use std::sync::Arc;

use crate::{
    backend::{Backend, BackendFuture, PermissionState},
    error::QrError,
    frame::FrameSource,
};

pub(crate) struct NativeBackend;

impl Backend for NativeBackend {
    fn camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        Box::pin(async { Ok(PermissionState::Granted) })
    }

    fn request_camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        Box::pin(async { Ok(PermissionState::Granted) })
    }

    /// No frame source ships here: live frames come from the camera repo, SuperNSH/nsh-camera.
    fn frame_source(&self) -> BackendFuture<'_, Result<Option<Arc<dyn FrameSource>>, QrError>> {
        Box::pin(async { Ok(None) })
    }
}
