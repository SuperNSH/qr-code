//! Web: `navigator.permissions.query({name: "camera"})` reads the state and `getUserMedia`
//! asks for it; the browser's own prompt is the only UI.
//!
//! Chrome reports "prompt" until the first ask and Firefox rejects the camera descriptor
//! outright, so anything but granted or denied reads `Undetermined`. `getUserMedia`'s promise
//! settles when the user answers that prompt, so these futures never hang on their own; on
//! wasm32 the TypeScript layer owns the timeout watchdog, as in biometrics.

use std::sync::Arc;

use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::{
    backend::{Backend, BackendFuture, PermissionState},
    error::{ErrorKind, QrError},
    frame::FrameSource,
};

pub(crate) struct NativeBackend;

/// `{name: "camera"}` as a plain object; web-sys's `PermissionName` enum carries no camera.
fn camera_descriptor() -> Option<js_sys::Object> {
    let descriptor = js_sys::Object::new();
    js_sys::Reflect::set(
        &descriptor,
        &JsValue::from("name"),
        &JsValue::from("camera"),
    )
    .ok()?;
    Some(descriptor)
}

/// Reads the browser's own answer: NotAllowedError is the user's denial, anything else is the
/// platform failing.
fn permission_error(error: JsValue) -> QrError {
    let denied = js_sys::Reflect::get(&error, &JsValue::from("name"))
        .ok()
        .and_then(|name| name.as_string())
        .is_some_and(|name| name == "NotAllowedError");
    if denied {
        QrError::new(ErrorKind::PermissionDenied)
    } else {
        QrError::new(ErrorKind::Backend)
    }
}

impl Backend for NativeBackend {
    fn camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        Box::pin(async {
            let Some(window) = web_sys::window() else {
                return Ok(PermissionState::Undetermined);
            };
            let Ok(permissions) = window.navigator().permissions() else {
                return Ok(PermissionState::Undetermined);
            };
            let Some(descriptor) = camera_descriptor() else {
                return Ok(PermissionState::Undetermined);
            };
            let Ok(promise) = permissions.query(&descriptor) else {
                return Ok(PermissionState::Undetermined);
            };
            let Ok(status) = JsFuture::from(promise).await else {
                return Ok(PermissionState::Undetermined);
            };
            let Some(state) = status
                .dyn_into::<web_sys::PermissionStatus>()
                .ok()
                .map(|status| status.state())
            else {
                return Ok(PermissionState::Undetermined);
            };
            Ok(match state {
                web_sys::PermissionState::Granted => PermissionState::Granted,
                web_sys::PermissionState::Denied => PermissionState::Denied,
                // wasm-bindgen exports enums non_exhaustive with a hidden __Invalid variant;
                // the wildcard (prompt and any future variant) is required to compile.
                _ => PermissionState::Undetermined,
            })
        })
    }

    fn request_camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        Box::pin(async {
            let Some(window) = web_sys::window() else {
                return Err(QrError::new(ErrorKind::Backend));
            };
            let Ok(media_devices) = window.navigator().media_devices() else {
                return Err(QrError::new(ErrorKind::Backend));
            };
            let constraints = web_sys::MediaStreamConstraints::new();
            constraints.set_video_bool(true);
            let Ok(promise) = media_devices.get_user_media_with_constraints(&constraints) else {
                return Err(QrError::new(ErrorKind::Backend));
            };
            let stream = JsFuture::from(promise).await.map_err(permission_error)?;
            let Ok(stream) = stream.dyn_into::<web_sys::MediaStream>() else {
                return Err(QrError::new(ErrorKind::Backend));
            };
            // The prompt answered; stopping the tracks releases the camera right away.
            for track in stream.get_tracks().iter() {
                if let Some(track) = track.dyn_ref::<web_sys::MediaStreamTrack>() {
                    track.stop();
                }
            }
            Ok(PermissionState::Granted)
        })
    }

    /// No frame source ships here: live frames come from the camera repo, SuperNSH/nsh-camera.
    fn frame_source(&self) -> BackendFuture<'_, Result<Option<Arc<dyn FrameSource>>, QrError>> {
        Box::pin(async { Ok(None) })
    }
}
