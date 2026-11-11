//! `QrScanner` through the fake backend: one scan at a time, the first code wins, stop, timeout.

mod common;

use std::{
    future::{Future, pending},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use nsh_qr_code::{
    ErrorKind, FakeBackend, FakeFrameSource, Frame, FrameSource, GenerateOptions, PermissionState,
    QrError, QrScanner, ScanOptions,
};

/// Runs on a single thread: the crate's boxed futures are not `Send`, so scans spawn locally.
fn run<F: Future>(future: F) -> F::Output {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    tokio::task::LocalSet::new().block_on(&runtime, future)
}

/// Yields until `ready` holds, so a spawned scan has reached its frame source.
async fn settle(ready: impl Fn() -> bool) {
    for _ in 0..1000 {
        if ready() {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("the scan never started");
}

/// A source whose frame never arrives: a scan parks on it until stop or its timeout ends it.
#[derive(Clone, Default)]
struct SilentFrameSource {
    calls: Arc<AtomicUsize>,
}

impl SilentFrameSource {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl FrameSource for SilentFrameSource {
    fn next_frame(&self) -> Pin<Box<dyn Future<Output = Result<Frame, QrError>> + '_>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(pending())
    }

    fn cancel(&self) {}
}

#[allow(clippy::field_reassign_with_default)]
fn scan_options(timeout: Option<Duration>) -> ScanOptions {
    let mut options = ScanOptions::default();
    options.timeout = timeout;
    options
}

fn code_frame(text: &[u8]) -> Frame {
    common::qr_frame(text, &GenerateOptions::default(), 4)
}

#[test]
fn scan_returns_first_code() {
    let fake = FakeBackend::new();
    fake.set_frame_source(Some(Arc::new(FakeFrameSource::new(vec![
        code_frame(b"first"),
        code_frame(b"second"),
    ]))));
    let scanner = QrScanner::with_backend(fake);
    run(async {
        let code = scanner
            .scan(&scan_options(None))
            .await
            .expect("the first frame carries a code");
        assert_eq!(code.text, "first");
    });
}

#[test]
fn scan_second_call_is_already_running() {
    let fake = FakeBackend::new();
    let silent = SilentFrameSource::default();
    fake.set_frame_source(Some(Arc::new(silent.clone())));
    let scanner = QrScanner::with_backend(fake);
    run(async {
        let first = {
            let scanner = scanner.clone();
            tokio::task::spawn_local(async move { scanner.scan(&scan_options(None)).await })
        };
        settle(|| silent.calls() >= 1).await;

        let error = scanner.scan(&scan_options(None)).await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::AlreadyRunning);
        assert_eq!(error.code(), "ERR_QR_ALREADY_RUNNING");

        assert!(scanner.stop().await.expect("a scan is running"));
        let cancelled = first.await.unwrap().unwrap_err();
        assert_eq!(cancelled.kind(), ErrorKind::Cancelled);
    });
}

#[test]
fn stop_resolves_cancelled_and_returns_true() {
    let fake = FakeBackend::new();
    let silent = SilentFrameSource::default();
    fake.set_frame_source(Some(Arc::new(silent.clone())));
    let scanner = QrScanner::with_backend(fake.clone());
    run(async {
        let call = {
            let scanner = scanner.clone();
            tokio::task::spawn_local(async move { scanner.scan(&scan_options(None)).await })
        };
        settle(|| silent.calls() >= 1).await;

        assert!(scanner.stop().await.expect("a scan is running"));
        let error = call.await.unwrap().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Cancelled);
        assert_eq!(error.code(), "ERR_QR_CANCELLED");
        assert!(!scanner.stop().await.expect("no scan is running any more"));

        // The gate is free again, so a scan with frames resolves the next code.
        fake.set_frame_source(Some(Arc::new(FakeFrameSource::new(vec![code_frame(
            b"after the stop",
        )]))));
        let code = scanner
            .scan(&scan_options(None))
            .await
            .expect("the gate is free");
        assert_eq!(code.text, "after the stop");
    });
}

#[test]
fn stop_without_scan_returns_false() {
    let scanner = QrScanner::with_backend(FakeBackend::new());
    run(async {
        assert!(!scanner.stop().await.expect("no scan is running"));
    });
}

#[test]
fn scan_timeout_resolves_timeout() {
    let fake = FakeBackend::new();
    let silent = SilentFrameSource::default();
    fake.set_frame_source(Some(Arc::new(silent.clone())));
    let scanner = QrScanner::with_backend(fake);
    run(async {
        tokio::time::pause();
        let call = {
            let scanner = scanner.clone();
            tokio::task::spawn_local(async move {
                scanner
                    .scan(&scan_options(Some(Duration::from_secs(1))))
                    .await
            })
        };
        settle(|| silent.calls() >= 1).await;
        tokio::time::advance(Duration::from_secs(2)).await;

        let error = call.await.unwrap().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Timeout);
        assert_eq!(error.code(), "ERR_QR_TIMEOUT");
    });
}

#[test]
fn closed_source_ends_scan_with_frame_source_closed() {
    let fake = FakeBackend::new();
    fake.set_frame_source(Some(Arc::new(FakeFrameSource::new(Vec::new()))));
    let scanner = QrScanner::with_backend(fake);
    run(async {
        let error = scanner.scan(&scan_options(None)).await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::FrameSourceClosed);
        assert_eq!(error.code(), "ERR_QR_FRAME_SOURCE_CLOSED");
    });
}

#[test]
fn scan_without_camera_is_unsupported() {
    run(async {
        let error = QrScanner::new()
            .scan(&scan_options(None))
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Unsupported);

        // A registered backend with no frame source is just as camera-less.
        let scanner = QrScanner::with_backend(FakeBackend::new());
        let error = scanner.scan(&scan_options(None)).await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Unsupported);
        assert_eq!(error.code(), "ERR_QR_UNSUPPORTED");
    });
}

#[test]
fn permission_states_round_trip() {
    let fake = FakeBackend::new();
    let scanner = QrScanner::with_backend(fake.clone());
    run(async {
        for state in [
            PermissionState::Undetermined,
            PermissionState::Granted,
            PermissionState::Denied,
        ] {
            fake.set_camera_permission(Ok(state));
            assert_eq!(
                scanner.camera_permission().await.expect("the state reads"),
                state
            );
            assert_eq!(
                scanner
                    .request_camera_permission()
                    .await
                    .expect("the state reads"),
                state
            );
        }
        assert_eq!(fake.permission_reads(), 3);
        assert_eq!(fake.requests(), 3);

        fake.set_camera_permission(Err(QrError::new(ErrorKind::PermissionDenied)));
        let error = scanner.camera_permission().await.unwrap_err();
        assert_eq!(error.code(), "ERR_QR_PERMISSION_DENIED");
    });
}
