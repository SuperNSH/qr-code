//! Android: the camera permission over JNI, on the context `ndk-context` carries.
//!
//! The runtime that embeds this library installs that context; with none attached there is no
//! activity to ask, so the calls read `Undetermined` — the discipline `nsh-biometrics` keeps for
//! its `NoActivity`. A request's answer is delivered to the activity's `onRequestPermissionsResult`,
//! which a Rust-only bridge cannot receive, so `request_camera_permission` watches the permission
//! itself: a grant resolves `Granted`, the platform's denial flag turning on resolves `Denied`, and
//! a decline the platform records nowhere — a repeated or a permanent one — resolves `Timeout` at
//! the poll's own deadline, because no caller bounds this call.

use std::{
    panic::catch_unwind,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use jni::{
    Env, JavaVM, jni_sig, jni_str,
    objects::{JObject, JValue},
    sys::{jint, jobject},
};

use crate::{
    backend::{Backend, BackendFuture, PermissionState},
    error::{ErrorKind, QrError},
    frame::FrameSource,
};

pub(crate) struct NativeBackend;

/// `PackageManager.PERMISSION_GRANTED`: what `checkSelfPermission` reports for a held permission.
const PERMISSION_GRANTED: jint = 0;
/// Runtime permissions begin with Android 6.0 (API 23); below it the install-time grant holds.
const SDK_WITH_RUNTIME_PERMISSIONS: jint = 23;
/// This bridge's request code: `QR` in hex, so the app cannot mistake the answer for its own.
const REQUEST_CODE: jint = 0x5152;
/// How long the request poll waits between rounds; a round is a quick pair of JNI calls.
const POLL_EVERY: Duration = Duration::from_millis(200);
/// The poll gives up after this long with no observable answer; nothing upstream bounds this call.
const REQUEST_DEADLINE: Duration = Duration::from_secs(30);
const CAMERA_PERMISSION: &str = "android.permission.CAMERA";

/// `ndk-context` has no try-get: its getter panics while nothing is installed, so a caught probe
/// is the only way to tell. The lock keeps concurrent probes from clobbering the panic hook.
static PROBE: Mutex<()> = Mutex::new(());

/// The context the runtime attached, or none when no runtime attached one.
fn attached_context() -> Option<ndk_context::AndroidContext> {
    let _probe = PROBE.lock().unwrap_or_else(PoisonError::into_inner);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let probe = catch_unwind(|| ndk_context::android_context());
    std::panic::set_hook(previous);
    probe.ok()
}

impl From<jni::errors::Error> for QrError {
    fn from(error: jni::errors::Error) -> Self {
        use jni::errors::{Error as Jni, JniError};
        // Only these two variants carry one of the platform's own codes; the rest carry none.
        let status = match &error {
            Jni::JniCall(JniError::Other(code)) => Some(i64::from(*code)),
            Jni::ThrowFailed(code) => Some(i64::from(*code)),
            _ => None,
        };
        match status {
            Some(status) => QrError::new(ErrorKind::Backend).with_status(status),
            None => QrError::new(ErrorKind::Backend),
        }
    }
}

/// Runs `work` on this thread, attached to the VM `ndk-context` carries, with the context it
/// carries. The scoped attach detaches again, so a caller's thread is never left attached.
fn with_context<R>(
    context: ndk_context::AndroidContext,
    work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> Result<R, QrError>,
) -> Result<R, QrError> {
    // SAFETY: the VM and the context belong to the runtime that installed them in ndk-context,
    // and both outlive this call.
    unsafe {
        let vm = JavaVM::from_raw(context.vm().cast());
        vm.attach_current_thread_for_scope(|env| {
            let java_context = JObject::from_raw(env, context.context() as jobject);
            work(env, &java_context)
        })
    }
}

/// `Build.VERSION.SDK_INT`.
fn sdk_int(env: &mut Env<'_>) -> Result<jint, QrError> {
    Ok(env
        .get_static_field(
            jni_str!("android/os/Build$VERSION"),
            jni_str!("SDK_INT"),
            jni_sig!("I"),
        )?
        .i()?)
}

/// `Context.checkSelfPermission` for the camera permission. Below API 23 there are no runtime
/// permissions to check; the install-time grant reads Granted.
fn camera_state(env: &mut Env<'_>, context: &JObject<'_>) -> Result<PermissionState, QrError> {
    if sdk_int(env)? < SDK_WITH_RUNTIME_PERMISSIONS {
        return Ok(PermissionState::Granted);
    }
    let permission: JObject<'_> = env.new_string(CAMERA_PERMISSION)?.into();
    let granted = env
        .call_method(
            context,
            jni_str!("checkSelfPermission"),
            jni_sig!("(Ljava/lang/String;)I"),
            &[JValue::Object(&permission)],
        )?
        .i()?;
    Ok(if granted == PERMISSION_GRANTED {
        PermissionState::Granted
    } else {
        PermissionState::Denied
    })
}

/// Whether the attached context is an `Activity`: ndk-context usually carries the activity, but a
/// runtime may attach the application instead, and only an activity has `requestPermissions`.
/// A null context answers no: JNI's `IsInstanceOf` would say yes to any class.
fn as_activity(env: &mut Env<'_>, context: &JObject<'_>) -> Result<bool, QrError> {
    if context.as_raw().is_null() {
        return Ok(false);
    }
    let class = env.find_class(jni_str!("android/app/Activity"))?;
    Ok(env.is_instance_of(context, &class)?)
}

/// `Activity.shouldShowRequestPermissionRationale`: the platform's own record that the app's last
/// request for the permission was declined.
fn denied_before(env: &mut Env<'_>, activity: &JObject<'_>) -> Result<bool, QrError> {
    let permission: JObject<'_> = env.new_string(CAMERA_PERMISSION)?.into();
    Ok(env
        .call_method(
            activity,
            jni_str!("shouldShowRequestPermissionRationale"),
            jni_sig!("(Ljava/lang/String;)Z"),
            &[JValue::Object(&permission)],
        )?
        .z()?)
}

/// `Activity.requestPermissions`: shows the system dialog for the camera permission.
fn fire_request(env: &mut Env<'_>, activity: &JObject<'_>) -> Result<(), QrError> {
    let strings = env.find_class(jni_str!("java/lang/String"))?;
    let permission: JObject<'_> = env.new_string(CAMERA_PERMISSION)?.into();
    let permissions = env.new_object_array(1, &strings, permission)?;
    env.call_method(
        activity,
        jni_str!("requestPermissions"),
        jni_sig!("([Ljava/lang/String;I)V"),
        &[JValue::Object(&permissions), JValue::Int(REQUEST_CODE)],
    )?
    .v()?;
    Ok(())
}

/// What one asking round trip settled.
enum Asked {
    /// A state that needed no dialog: no activity to ask, or the permission already holds.
    Answer(PermissionState),
    /// The dialog is up; `denied_before` is the denial flag as it stood when it appeared.
    Pending { denied_before: bool },
}

impl Backend for NativeBackend {
    fn camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        Box::pin(async {
            match attached_context() {
                // No runtime attached a context: there is no activity to ask.
                None => Ok(PermissionState::Undetermined),
                Some(context) => with_context(context, camera_state),
            }
        })
    }

    fn request_camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        Box::pin(async {
            let Some(context) = attached_context() else {
                return Ok(PermissionState::Undetermined);
            };
            let asked = with_context(context, |env, java| {
                if !as_activity(env, java)? {
                    return Ok(Asked::Answer(PermissionState::Undetermined));
                }
                let state = camera_state(env, java)?;
                if state == PermissionState::Granted {
                    return Ok(Asked::Answer(state));
                }
                fire_request(env, java)?;
                Ok(Asked::Pending {
                    denied_before: denied_before(env, java)?,
                })
            })?;
            match asked {
                Asked::Answer(state) => Ok(state),
                Asked::Pending {
                    denied_before: was_denied,
                } => {
                    let poll = async {
                        loop {
                            tokio::time::sleep(POLL_EVERY).await;
                            let answer = with_context(context, |env, java| {
                                Ok((camera_state(env, java)?, denied_before(env, java)?))
                            })?;
                            match answer {
                                (PermissionState::Granted, _) => {
                                    return Ok(PermissionState::Granted);
                                }
                                // The flag turning on is the platform saying the request was
                                // declined; a repeat or a permanent decline turns on no flag.
                                (_, denied) if denied && !was_denied => {
                                    return Ok(PermissionState::Denied);
                                }
                                _ => {}
                            }
                        }
                    };
                    match tokio::time::timeout(REQUEST_DEADLINE, poll).await {
                        Ok(answer) => answer,
                        // Nothing observable changed in time: an unanswered dialog or a decline
                        // the platform records nowhere. The deadline resolves the call.
                        Err(_) => Err(QrError::new(ErrorKind::Timeout)),
                    }
                }
            }
        })
    }

    /// No frame source ships here: live frames come from the camera repo, SuperNSH/nsh-camera.
    fn frame_source(&self) -> BackendFuture<'_, Result<Option<Arc<dyn FrameSource>>, QrError>> {
        Box::pin(async { Ok(None) })
    }
}
