//! `QrScanner`: the calls an app makes, with one scan at a time and one answer per call.

use std::{
    sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError},
    time::Duration,
};

use tokio::sync::oneshot;

use crate::{
    backend::{Backend, PermissionState},
    decode::decode_frame,
    error::{ErrorKind, QrError},
    frame::FrameSource,
    options::{DecodedQr, ScanOptions},
};

type Answer = Result<DecodedQr, QrError>;

/// The scan gate. `busy` lasts until the scan's frames stop coming, even after the call was
/// cancelled, so a second scan never takes a source the first still reads.
///
/// `waiting` names the generation that owns the gate, so a stop, a timeout or a drop of an
/// older generation cannot touch a newer call that took over after the gate was freed.
#[derive(Default)]
struct Gate {
    busy: bool,
    next_id: u64,
    waiting: Option<(u64, oneshot::Sender<Answer>)>,
    source: Option<Arc<dyn FrameSource>>,
}

struct Shared {
    backend: Arc<dyn Backend>,
    gate: Mutex<Gate>,
}

impl Shared {
    fn gate(&self) -> MutexGuard<'_, Gate> {
        self.gate.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Scanning camera frames for a QR code.
///
/// Clones share one scan gate. [`QrScanner::new`] returns the platform's shared handle, so two
/// calls in one app share it too.
#[derive(Clone)]
pub struct QrScanner {
    shared: Arc<Shared>,
}

impl QrScanner {
    /// The platform's scanner, with one scan gate for the whole process.
    pub fn new() -> Self {
        static PLATFORM: OnceLock<QrScanner> = OnceLock::new();
        PLATFORM
            .get_or_init(|| Self::from_arc(crate::platform::default_backend()))
            .clone()
    }

    /// Scanning through `backend`, with its own scan gate. Tests and fakes use this.
    pub fn with_backend(backend: impl Backend + 'static) -> Self {
        Self::from_arc(Arc::new(backend))
    }

    fn from_arc(backend: Arc<dyn Backend>) -> Self {
        Self {
            shared: Arc::new(Shared {
                backend,
                gate: Mutex::new(Gate::default()),
            }),
        }
    }

    /// The camera permission as it stands, without asking.
    pub async fn camera_permission(&self) -> Result<PermissionState, QrError> {
        self.shared.backend.camera_permission().await
    }

    /// Asks for the camera permission and reports how it ended.
    pub async fn request_camera_permission(&self) -> Result<PermissionState, QrError> {
        self.shared.backend.request_camera_permission().await
    }

    /// Scans frames until one carries a QR code, and returns the first code it reads.
    ///
    /// Every outcome resolves exactly once with a code: the code found, `ERR_QR_UNSUPPORTED`
    /// when no camera is registered, `ERR_QR_FRAME_SOURCE_CLOSED` when the frames stop,
    /// `ERR_QR_ALREADY_RUNNING` for a second scan while one runs, `ERR_QR_TIMEOUT` when
    /// `ScanOptions::timeout` passes, and `ERR_QR_CANCELLED` for a `stop` call. A frame that
    /// does not decode is ordinary — blur, rotation and low light all look like that — so the
    /// scan keeps going. Dropping the call cancels the frame source.
    pub async fn scan(&self, options: &ScanOptions) -> Result<DecodedQr, QrError> {
        if options.timeout == Some(Duration::ZERO) {
            return Err(QrError::new(ErrorKind::InvalidArgument));
        }
        let (sender, mut receiver) = oneshot::channel();
        // The owner guards the gate from the moment it is taken, before the first await: a
        // drop at any await point frees the gate, or a dropped call wedges every later scan.
        let mut owner = {
            let mut gate = self.shared.gate();
            if gate.busy {
                return Err(QrError::new(ErrorKind::AlreadyRunning));
            }
            gate.busy = true;
            let generation = gate.next_id;
            gate.next_id += 1;
            gate.waiting = Some((generation, sender));
            Owner {
                shared: Arc::clone(&self.shared),
                generation,
                source: None,
                armed: true,
            }
        };
        // The timer bounds the whole scan, the backend's source call and the frames alike. It
        // needs tokio's time driver, which wasm32 has no runtime for; there the option is
        // accepted and unused.
        #[cfg(not(target_arch = "wasm32"))]
        let answer = match options.timeout {
            Some(timeout) => tokio::select! {
                answer = self.scan_frames(&mut receiver, &mut owner) => answer,
                _ = tokio::time::sleep(timeout) => Err(QrError::new(ErrorKind::Timeout)),
            },
            None => self.scan_frames(&mut receiver, &mut owner).await,
        };
        #[cfg(target_arch = "wasm32")]
        let answer = self.scan_frames(&mut receiver, &mut owner).await;
        owner.armed = false;
        // Cancel before the gate opens: on a multithreaded runtime a successor scan can take
        // the same shared source the instant the gate frees, and this cancel must not close
        // its feed. A timeout leaves the source mid-feed; other answers already ended it.
        if matches!(&answer, Err(error) if error.kind() == ErrorKind::Timeout)
            && let Some(source) = owner.source.take()
        {
            source.cancel();
        }
        free_gate(&self.shared, owner.generation);
        answer
    }

    /// The scan past the gate: take the backend's frame source, then read frames until one
    /// decodes or the source ends. A `stop` call resolves through `receiver`.
    async fn scan_frames(
        &self,
        receiver: &mut oneshot::Receiver<Answer>,
        owner: &mut Owner,
    ) -> Answer {
        let source = match self.shared.backend.frame_source().await {
            Ok(Some(source)) => source,
            Ok(None) => return Err(QrError::new(ErrorKind::Unsupported)),
            Err(error) => return Err(error),
        };
        {
            let mut gate = self.shared.gate();
            if matches!(&gate.waiting, Some((id, _)) if *id == owner.generation) {
                gate.source = Some(Arc::clone(&source));
            } else {
                // A stop landed while the source was being fetched: no later path holds this
                // source to cancel it, so it stops feeding here, as `stop` promises. The
                // stop already sent the scan's Cancelled; answering it again keeps one
                // resolution.
                drop(gate);
                source.cancel();
                return Err(QrError::new(ErrorKind::Cancelled));
            }
        }
        owner.source = Some(Arc::clone(&source));
        tokio::select! {
            answer = scan_source(&source) => answer,
            answer = receiver => answer.unwrap_or_else(|_| Err(QrError::new(ErrorKind::Backend))),
        }
    }

    /// Stops the running scan. The pending `scan` call resolves with `ERR_QR_CANCELLED`.
    /// Returns `true` when a scan was running; a second `stop` returns `false`.
    pub async fn stop(&self) -> Result<bool, QrError> {
        let (waiting, source) = {
            let mut gate = self.shared.gate();
            (gate.waiting.take(), gate.source.take())
        };
        // The source stops feeding frames even when its scan already answered elsewhere.
        if let Some(source) = source {
            source.cancel();
        }
        match waiting {
            Some((_, sender)) => {
                let _ = sender.send(Err(QrError::new(ErrorKind::Cancelled)));
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

impl Default for QrScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Reads frames until one decodes. A frame that does not decode is ordinary, so only the
/// source ending the feed — or failing — ends the loop.
async fn scan_source(source: &Arc<dyn FrameSource>) -> Answer {
    loop {
        let frame = match source.next_frame().await {
            Ok(frame) => frame,
            Err(error) => return Err(error),
        };
        if let Ok(found) = decode_frame(&frame)
            && let Some(first) = found.into_iter().next()
        {
            return Ok(first);
        }
    }
}

/// Frees the gate for a call that answered, timed out, was stopped, was dropped or never
/// started. A generation the gate no longer holds is left alone: a newer call owns it. A gate
/// whose `waiting` a `stop` took is still the caller's to free; nothing else clears it.
fn free_gate(shared: &Shared, generation: u64) {
    let mut gate = shared.gate();
    let mine = matches!(&gate.waiting, Some((id, _)) if *id == generation);
    if mine || (gate.waiting.is_none() && gate.busy) {
        gate.waiting = None;
        gate.source = None;
        gate.busy = false;
    }
}

/// Guards the gate and the frame source from the moment the scan takes them: a drop of the
/// call at any await point frees the gate and cancels the source.
struct Owner {
    shared: Arc<Shared>,
    generation: u64,
    source: Option<Arc<dyn FrameSource>>,
    armed: bool,
}

impl Drop for Owner {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        // The owner frees its own gate, whatever await the drop landed on: no later answer
        // comes to free it for a dropped call. The generation check inside leaves a newer
        // call's gate alone after a timeout freed this one for a fresh scan.
        if let Some(source) = self.source.take() {
            source.cancel();
        }
        free_gate(&self.shared, self.generation);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::{Future, pending},
        pin::pin,
        sync::{Arc, Mutex, MutexGuard, PoisonError},
        time::Duration,
    };

    use super::*;
    use crate::{
        fake::{FakeBackend, FakeFrameSource},
        frame::Frame,
        generate::generate,
        options::{GenerateOptions, ScanOptions},
    };

    fn run<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(future)
    }

    /// Yields until `ready` holds, so a scan polled beside this has reached its frames.
    /// The crate's boxed futures are not `Send`, so tests interleave scans with `join!` and
    /// `select!` on one task instead of spawning them.
    async fn settle(ready: impl Fn() -> bool) {
        for _ in 0..1000 {
            if ready() {
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("the scan never started");
    }

    /// A source whose frames never arrive, staging a scan that stays mid-frame — the case
    /// `FakeFrameSource` cannot stage.
    #[derive(Clone, Default)]
    struct StalledSource {
        calls: Arc<Mutex<usize>>,
        cancels: Arc<Mutex<usize>>,
    }

    impl StalledSource {
        fn calls(&self) -> usize {
            *self.lock()
        }

        fn cancels(&self) -> usize {
            *self.cancels.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn lock(&self) -> MutexGuard<'_, usize> {
            self.calls.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    impl FrameSource for StalledSource {
        fn next_frame(&self) -> crate::frame::FrameFuture<'_, Result<Frame, QrError>> {
            *self.lock() += 1;
            Box::pin(pending())
        }

        fn cancel(&self) {
            *self.cancels.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        }
    }

    fn scanner_with(source: Arc<dyn FrameSource>) -> QrScanner {
        let backend = FakeBackend::new();
        backend.set_frame_source(Some(source));
        QrScanner::with_backend(backend)
    }

    /// A frame with a code in it, and one without.
    fn code_and_blank_frames() -> Vec<Frame> {
        let matrix = generate(b"scan me", &GenerateOptions::default()).expect("encodes");
        let size = matrix.size + 8;
        let mut luma = vec![255u8; size * size];
        for y in 0..matrix.size {
            for x in 0..matrix.size {
                luma[(y + 4) * size + x + 4] = u8::from(!matrix.modules[y * matrix.size + x]) * 255;
            }
        }
        vec![
            Frame::new(8, 8, vec![128; 64]).expect("8*8 pixels"),
            Frame::new(size, size, luma).expect("one luma per pixel"),
        ]
    }

    #[test]
    fn a_scan_reads_the_code_from_a_later_frame() {
        let scanner = scanner_with(Arc::new(FakeFrameSource::new(code_and_blank_frames())));
        run(async {
            let decoded = scanner.scan(&ScanOptions::default()).await.expect("found");
            assert_eq!(decoded.text, "scan me");
        });
    }

    #[test]
    fn a_second_scan_is_refused_while_one_runs() {
        let stalled = Arc::new(StalledSource::default());
        let scanner = scanner_with(Arc::clone(&stalled) as Arc<dyn FrameSource>);
        run(async {
            let options = ScanOptions::default();
            let other = scanner.clone();
            let watched = Arc::clone(&stalled);
            let (answer, ()) = tokio::join!(scanner.scan(&options), async move {
                settle(|| watched.calls() == 1).await;
                let options = ScanOptions::default();
                let refused = other.scan(&options).await.unwrap_err();
                assert_eq!(refused.kind(), ErrorKind::AlreadyRunning);
                assert!(other.stop().await.expect("stop works"));
            });
            assert_eq!(answer.unwrap_err().kind(), ErrorKind::Cancelled);

            // The cancelled scan freed the gate: the next one reaches its frames.
            let next = scanner.clone();
            let watched = Arc::clone(&stalled);
            let (answer, ()) = tokio::join!(scanner.scan(&options), async move {
                settle(|| watched.calls() == 2).await;
                assert!(next.stop().await.expect("stop works"));
            });
            assert_eq!(answer.unwrap_err().kind(), ErrorKind::Cancelled);
        });
    }

    #[test]
    fn a_second_stop_reports_false() {
        let stalled = Arc::new(StalledSource::default());
        let scanner = scanner_with(Arc::clone(&stalled) as Arc<dyn FrameSource>);
        run(async {
            let options = ScanOptions::default();
            let other = scanner.clone();
            let watched = Arc::clone(&stalled);
            let (answer, ()) = tokio::join!(scanner.scan(&options), async move {
                settle(|| watched.calls() == 1).await;
                assert!(other.stop().await.expect("first stop"));
                assert!(!other.stop().await.expect("second stop"));
            });
            assert_eq!(answer.unwrap_err().kind(), ErrorKind::Cancelled);
        });
    }

    #[test]
    fn stop_reports_false_with_no_scan_running() {
        let scanner = QrScanner::with_backend(FakeBackend::new());
        run(async {
            assert!(!scanner.stop().await.expect("stop works"));
        });
    }

    #[test]
    fn a_scan_without_a_registered_camera_is_unsupported() {
        let backend = FakeBackend::new();
        backend.set_frame_source(None);
        let scanner = QrScanner::with_backend(backend);
        run(async {
            let error = scanner.scan(&ScanOptions::default()).await.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Unsupported);
            // The failed start freed the gate.
            let again = scanner.scan(&ScanOptions::default()).await.unwrap_err();
            assert_eq!(again.kind(), ErrorKind::Unsupported);
        });
    }

    #[test]
    fn frames_ending_closes_the_scan() {
        let source = FakeFrameSource::new(vec![Frame::new(8, 8, vec![128; 64]).expect("8*8")]);
        let scanner = scanner_with(Arc::new(source));
        run(async {
            let error = scanner.scan(&ScanOptions::default()).await.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::FrameSourceClosed);
        });
    }

    #[test]
    fn a_zero_timeout_is_refused_without_a_scan() {
        let stalled = Arc::new(StalledSource::default());
        let scanner = scanner_with(Arc::clone(&stalled) as Arc<dyn FrameSource>);
        run(async {
            let options = ScanOptions {
                timeout: Some(Duration::ZERO),
            };
            let error = scanner.scan(&options).await.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidArgument);
            assert_eq!(stalled.calls(), 0, "no frames were asked for");
        });
    }

    #[test]
    fn a_scan_times_out_and_frees_the_gate() {
        let stalled = Arc::new(StalledSource::default());
        let scanner = scanner_with(Arc::clone(&stalled) as Arc<dyn FrameSource>);
        let options = ScanOptions {
            timeout: Some(Duration::from_secs(1)),
        };
        run(async {
            tokio::time::pause();
            let watched = Arc::clone(&stalled);
            let (answer, ()) = tokio::join!(scanner.scan(&options), async move {
                settle(|| watched.calls() == 1).await;
                tokio::time::advance(Duration::from_secs(2)).await;
            });
            assert_eq!(answer.unwrap_err().kind(), ErrorKind::Timeout);
            assert_eq!(
                stalled.cancels(),
                1,
                "the timed-out scan cancelled its source"
            );

            // The timeout freed the gate: the next scan starts and times out too.
            let timed = options.clone();
            let watched = Arc::clone(&stalled);
            let (answer, ()) = tokio::join!(scanner.scan(&timed), async move {
                settle(|| watched.calls() == 2).await;
                tokio::time::advance(Duration::from_secs(2)).await;
            });
            assert_eq!(answer.unwrap_err().kind(), ErrorKind::Timeout);
        });
    }

    /// A backend whose `frame_source` hangs until the test sets one, staging a scan that
    /// awaits its backend — the case `FakeBackend` cannot stage.
    #[derive(Clone, Default)]
    struct SlowBackend {
        calls: Arc<Mutex<usize>>,
        source: Arc<Mutex<Option<Arc<dyn FrameSource>>>>,
    }

    impl SlowBackend {
        fn calls(&self) -> usize {
            *self.calls.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn set_source(&self, source: Arc<dyn FrameSource>) {
            *self.source.lock().unwrap_or_else(PoisonError::into_inner) = Some(source);
        }
    }

    impl Backend for SlowBackend {
        fn camera_permission(
            &self,
        ) -> crate::backend::BackendFuture<'_, Result<PermissionState, QrError>> {
            Box::pin(std::future::ready(Ok(PermissionState::Undetermined)))
        }

        fn request_camera_permission(
            &self,
        ) -> crate::backend::BackendFuture<'_, Result<PermissionState, QrError>> {
            Box::pin(std::future::ready(Ok(PermissionState::Undetermined)))
        }

        fn frame_source(
            &self,
        ) -> crate::backend::BackendFuture<'_, Result<Option<Arc<dyn FrameSource>>, QrError>>
        {
            *self.calls.lock().unwrap_or_else(PoisonError::into_inner) += 1;
            let source = self
                .source
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
            match source {
                Some(source) => Box::pin(std::future::ready(Ok(Some(source)))),
                None => Box::pin(pending()),
            }
        }
    }

    #[test]
    fn a_scan_dropped_while_awaiting_its_source_frees_the_gate() {
        let backend = SlowBackend::default();
        let scanner = QrScanner::with_backend(backend.clone());
        run(async {
            let options = ScanOptions::default();
            {
                let mut scan = pin!(scanner.scan(&options));
                let watched = backend.clone();
                tokio::select! {
                    answer = &mut scan => panic!("the sourceless scan answered: {answer:?}"),
                    () = settle(move || watched.calls() == 1) => {}
                }
                // The block's end drops the scan mid-await, as a caller's abort does.
            }
            // The dropped scan freed the gate: the next one runs and reads its code.
            backend.set_source(Arc::new(FakeFrameSource::new(code_and_blank_frames())));
            let decoded = scanner.scan(&options).await.expect("the gate was freed");
            assert_eq!(decoded.text, "scan me");
        });
    }

    #[test]
    fn a_slow_backend_call_is_bound_by_the_timeout() {
        let backend = SlowBackend::default();
        let scanner = QrScanner::with_backend(backend.clone());
        run(async {
            tokio::time::pause();
            let options = ScanOptions {
                timeout: Some(Duration::from_secs(1)),
            };
            let watched = backend.clone();
            let (answer, ()) = tokio::join!(scanner.scan(&options), async move {
                settle(|| watched.calls() == 1).await;
                tokio::time::advance(Duration::from_secs(2)).await;
            });
            assert_eq!(answer.unwrap_err().kind(), ErrorKind::Timeout);
        });
    }

    /// A backend that answers `frame_source` only once the test has set one, re-checking on
    /// every poll — staging a source that arrives after a stop.
    #[derive(Clone, Default)]
    struct LateSourceBackend {
        calls: Arc<Mutex<usize>>,
        source: Arc<Mutex<Option<Arc<dyn FrameSource>>>>,
    }

    impl LateSourceBackend {
        fn calls(&self) -> usize {
            *self.calls.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn set_source(&self, source: Arc<dyn FrameSource>) {
            *self.source.lock().unwrap_or_else(PoisonError::into_inner) = Some(source);
        }
    }

    impl Backend for LateSourceBackend {
        fn camera_permission(
            &self,
        ) -> crate::backend::BackendFuture<'_, Result<PermissionState, QrError>> {
            Box::pin(std::future::ready(Ok(PermissionState::Undetermined)))
        }

        fn request_camera_permission(
            &self,
        ) -> crate::backend::BackendFuture<'_, Result<PermissionState, QrError>> {
            Box::pin(std::future::ready(Ok(PermissionState::Undetermined)))
        }

        fn frame_source(
            &self,
        ) -> crate::backend::BackendFuture<'_, Result<Option<Arc<dyn FrameSource>>, QrError>>
        {
            *self.calls.lock().unwrap_or_else(PoisonError::into_inner) += 1;
            let source = Arc::clone(&self.source);
            Box::pin(async move {
                loop {
                    let held = source
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .clone();
                    if let Some(source) = held {
                        return Ok(Some(source));
                    }
                    tokio::task::yield_now().await;
                }
            })
        }
    }

    #[test]
    fn a_source_arriving_after_a_stop_is_cancelled() {
        let backend = LateSourceBackend::default();
        let scanner = QrScanner::with_backend(backend.clone());
        let stalled = Arc::new(StalledSource::default());
        run(async {
            let options = ScanOptions::default();
            let other = scanner.clone();
            let watched = backend.clone();
            let feed = Arc::clone(&stalled);
            let (answer, ()) = tokio::join!(scanner.scan(&options), async move {
                settle(|| watched.calls() == 1).await;
                assert!(other.stop().await.expect("a scan was running"));
                watched.set_source(Arc::clone(&feed) as Arc<dyn FrameSource>);
            });
            assert_eq!(answer.unwrap_err().kind(), ErrorKind::Cancelled);
            assert_eq!(
                stalled.cancels(),
                1,
                "the late source stops feeding on arrival"
            );
        });
    }

    #[test]
    fn dropping_the_call_cancels_the_frame_source() {
        let stalled = Arc::new(StalledSource::default());
        let scanner = scanner_with(Arc::clone(&stalled) as Arc<dyn FrameSource>);
        run(async {
            let options = ScanOptions::default();
            // The block's end drops the still-pending scan, as a caller's abort does: its
            // owner cancels the source.
            {
                let mut scan = pin!(scanner.scan(&options));
                let watched = Arc::clone(&stalled);
                tokio::select! {
                    answer = &mut scan => panic!("the stalled scan answered: {answer:?}"),
                    () = settle(move || watched.calls() == 1) => {}
                }
            }
            assert_eq!(stalled.cancels(), 1, "the owner going cancels the source");

            // The dropped scan freed the gate: the next one reaches its frames and stops.
            let next = scanner.clone();
            let watched = Arc::clone(&stalled);
            let (answer, ()) = tokio::join!(scanner.scan(&options), async move {
                settle(|| watched.calls() == 2).await;
                assert!(next.stop().await.expect("stop works"));
            });
            assert_eq!(answer.unwrap_err().kind(), ErrorKind::Cancelled);
        });
    }
}
