//! A backend and a frame source with scripted answers. Tests use them, and so can an app
//! that runs without a camera.

use std::{
    collections::VecDeque,
    future::ready,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use crate::{
    backend::{Backend, BackendFuture, PermissionState},
    error::{ErrorKind, QrError},
    frame::{Frame, FrameFuture, FrameSource},
};

/// A backend whose permission and frame source a test sets. Clones share one state.
#[derive(Clone)]
pub struct FakeBackend {
    state: Arc<Mutex<FakeState>>,
}

struct FakeState {
    permission: Result<PermissionState, QrError>,
    frame_source: Option<Arc<dyn FrameSource>>,
    permission_reads: usize,
    requests: usize,
}

impl Default for FakeBackend {
    /// A backend with the permission unasked and no camera registered.
    fn default() -> Self {
        Self::new()
    }
}

impl FakeBackend {
    /// A backend with the permission unasked and no camera registered.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeState {
                permission: Ok(PermissionState::Undetermined),
                frame_source: None,
                permission_reads: 0,
                requests: 0,
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, FakeState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Sets what both permission calls answer.
    pub fn set_camera_permission(&self, answer: Result<PermissionState, QrError>) {
        self.lock().permission = answer;
    }

    /// Sets the camera the backend hands out; `None` registers no camera.
    pub fn set_frame_source(&self, source: Option<Arc<dyn FrameSource>>) {
        self.lock().frame_source = source;
    }

    /// How many times the permission was read without asking.
    pub fn permission_reads(&self) -> usize {
        self.lock().permission_reads
    }

    /// How many times the permission was asked for.
    pub fn requests(&self) -> usize {
        self.lock().requests
    }
}

impl Backend for FakeBackend {
    fn camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        let answer = {
            let mut state = self.lock();
            state.permission_reads += 1;
            state.permission.clone()
        };
        Box::pin(ready(answer))
    }

    fn request_camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>> {
        let answer = {
            let mut state = self.lock();
            state.requests += 1;
            state.permission.clone()
        };
        Box::pin(ready(answer))
    }

    fn frame_source(&self) -> BackendFuture<'_, Result<Option<Arc<dyn FrameSource>>, QrError>> {
        let answer = Ok(self.lock().frame_source.clone());
        Box::pin(ready(answer))
    }
}

/// A frame source that feeds a queue of frames, then answers closed. Clones share one state.
#[derive(Clone)]
pub struct FakeFrameSource {
    state: Arc<Mutex<FakeSourceState>>,
}

struct FakeSourceState {
    frames: VecDeque<Frame>,
    closed: bool,
    nexts: usize,
}

impl FakeFrameSource {
    /// A source that feeds `frames` in order; when they run out, or after a `cancel`, it
    /// answers `ERR_QR_FRAME_SOURCE_CLOSED`.
    pub fn new(frames: Vec<Frame>) -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeSourceState {
                frames: VecDeque::from(frames),
                closed: false,
                nexts: 0,
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, FakeSourceState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// How many frames were asked for.
    pub fn next_frame_calls(&self) -> usize {
        self.lock().nexts
    }
}

impl FrameSource for FakeFrameSource {
    fn next_frame(&self) -> FrameFuture<'_, Result<Frame, QrError>> {
        let answer = {
            let mut state = self.lock();
            state.nexts += 1;
            if state.closed {
                Err(QrError::new(ErrorKind::FrameSourceClosed))
            } else {
                match state.frames.pop_front() {
                    Some(frame) => Ok(frame),
                    None => Err(QrError::new(ErrorKind::FrameSourceClosed)),
                }
            }
        };
        Box::pin(ready(answer))
    }

    fn cancel(&self) {
        // One close serves every later call; cancelling twice changes nothing.
        self.lock().closed = true;
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;

    use super::*;
    use crate::{options::ScanOptions, scanner::QrScanner};

    fn run<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn the_permission_calls_answer_the_script_and_count() {
        let backend = FakeBackend::new();
        let scanner = QrScanner::with_backend(backend.clone());
        run(async {
            assert_eq!(
                scanner.camera_permission().await.expect("reads"),
                PermissionState::Undetermined
            );
            backend.set_camera_permission(Ok(PermissionState::Denied));
            assert_eq!(
                scanner.request_camera_permission().await.expect("asks"),
                PermissionState::Denied
            );
            assert_eq!(backend.permission_reads(), 1);
            assert_eq!(backend.requests(), 1);
        });
    }

    #[test]
    fn a_denied_permission_carries_its_code() {
        let backend = FakeBackend::new();
        backend.set_camera_permission(Err(QrError::new(ErrorKind::PermissionDenied)));
        let scanner = QrScanner::with_backend(backend);
        run(async {
            let error = scanner.request_camera_permission().await.unwrap_err();
            assert_eq!(error.code(), "ERR_QR_PERMISSION_DENIED");
        });
    }

    #[test]
    fn the_frame_source_hands_out_what_was_set() {
        let backend = FakeBackend::new();
        backend.set_frame_source(Some(Arc::new(FakeFrameSource::new(Vec::new()))));
        let scanner = QrScanner::with_backend(backend);
        run(async {
            // The empty fake source closes at once: the scan sees the registered camera.
            let error = scanner.scan(&ScanOptions::default()).await.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::FrameSourceClosed);
        });
    }

    #[test]
    fn a_source_feeds_its_queue_then_closes() {
        let source = FakeFrameSource::new(vec![
            Frame::new(1, 1, vec![0]).expect("1 pixel"),
            Frame::new(1, 1, vec![255]).expect("1 pixel"),
        ]);
        run(async {
            assert_eq!(source.next_frame().await.expect("frame 1").luma, vec![0]);
            assert_eq!(source.next_frame().await.expect("frame 2").luma, vec![255]);
            let closed = source.next_frame().await.unwrap_err();
            assert_eq!(closed.kind(), ErrorKind::FrameSourceClosed);
            // A closed source keeps answering closed, one answer per call.
            let again = source.next_frame().await.unwrap_err();
            assert_eq!(again.kind(), ErrorKind::FrameSourceClosed);
            assert_eq!(source.next_frame_calls(), 4);
        });
    }

    #[test]
    fn cancelling_closes_the_source() {
        let source = FakeFrameSource::new(vec![Frame::new(1, 1, vec![0]).expect("1 pixel")]);
        run(async {
            assert!(source.next_frame().await.is_ok());
            source.cancel();
            let closed = source.next_frame().await.unwrap_err();
            assert_eq!(closed.kind(), ErrorKind::FrameSourceClosed);
            assert_eq!(source.next_frame_calls(), 2);
        });
    }
}
