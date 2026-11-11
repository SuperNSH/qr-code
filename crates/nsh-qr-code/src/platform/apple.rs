//! Apple's camera bridge. AVCaptureDevice answers the permission; live frames arrive with
//! SuperNSH/nsh-camera, which implements `FrameSource`. Until it registers one, there is no
//! camera here.

use std::{cell::Cell, sync::Arc};

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaType, AVMediaTypeVideo};

use crate::{
    backend::{Backend, BackendFuture, PermissionState},
    error::{ErrorKind, QrError},
    frame::FrameSource,
};

pub(crate) struct NativeBackend;

/// The media type AVFoundation authorizes cameras under, or a `Backend` error when the
/// framework did not load its constants.
fn video_media_type() -> Result<&'static AVMediaType, QrError> {
    // SAFETY: a framework constant; reading it runs no code and touches no user data.
    unsafe { AVMediaTypeVideo }.ok_or_else(|| QrError::new(ErrorKind::Backend))
}

/// An AVFoundation status as our permission state. `.restricted` counts as denied:
/// the user cannot grant it, only the system can.
fn status_to_state(status: AVAuthorizationStatus) -> PermissionState {
    if status == AVAuthorizationStatus::Authorized {
        PermissionState::Granted
    } else if status == AVAuthorizationStatus::NotDetermined {
        PermissionState::Undetermined
    } else {
        PermissionState::Denied
    }
}

impl Backend for NativeBackend {
    fn camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        Box::pin(async {
            let media = video_media_type()?;
            // SAFETY: reads the process's current video authorization; no prompt, no blocking.
            let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media) };
            Ok(status_to_state(status))
        })
    }

    fn request_camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        Box::pin(async {
            let media = video_media_type()?;
            let (done, answer) = tokio::sync::oneshot::channel::<bool>();
            // A block must be Fn, so the sender sits in a Cell: the first call settles
            // the future, a second one (which AVFoundation does not make) no-ops.
            let done = Cell::new(Some(done));
            // The framework answers on a queue of its own; the sender is Send, so the
            // block may run anywhere.
            let handler = RcBlock::new(move |granted: Bool| {
                if let Some(done) = done.take() {
                    let _ = done.send(granted.as_bool());
                }
            });
            // SAFETY: AVFoundation's documented class method; the block matches the
            // Fn(Bool) completion handler, of which the framework keeps its own copy.
            unsafe {
                AVCaptureDevice::requestAccessForMediaType_completionHandler(media, &handler);
            }
            // The await ends when the user answers the system dialog: the OS calls the
            // block exactly once. Dropping the future cancels it — the send then no-ops.
            let granted = answer.await.map_err(|_| QrError::new(ErrorKind::Backend))?;
            Ok(if granted {
                PermissionState::Granted
            } else {
                PermissionState::Denied
            })
        })
    }

    // Ok(None): no camera registered; SuperNSH/nsh-camera provides the frame source.
    fn frame_source(&self) -> BackendFuture<'_, Result<Option<Arc<dyn FrameSource>>, QrError>> {
        Box::pin(async { Ok(None) })
    }
}
