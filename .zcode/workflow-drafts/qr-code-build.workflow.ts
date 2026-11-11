// Build nsh-qr-code: QR decode + generate in a Rust crate and a TypeScript package,
// laid out like SuperNSH/biometrics, with a FrameSource trait for the camera repo,
// per-platform bridges, and CI for both halves. Eight areas run their own chains in
// parallel; the brief's five gates run once at the join with a bounded fix loop.

export const meta = {
  name: "nsh-qr-code capability build",
  description:
    "Rust crate + TS package for QR decode/generate with a frame-source trait, platform bridges and CI, in eight parallel areas with three audit rounds each",
  phases: [
    { title: "Implement every area in parallel" },
    { title: "Run the full gates once and fix what they catch" },
  ],
};

const MODELS = {
  pro: model("account:zai-individual-coding-plan/GLM-5.3$high"),
  flash: model("account:zai-individual-coding-plan/GLM-5.3-Flash$high"),
};

const KIT = "/Users/hmziq/ZCodeProject/ai/harness/zcode/agents";

async function specialist(name: string, fallback: string): Promise<{ persona: string; source: string }> {
  const adapter =
    `You are the ${name} specialist for one area of a parallel workflow. ` +
    `Your full specialist contract follows verbatim — apply it to this area's work; ` +
    `where it conflicts with your ask, the ask wins.` + "\n\n";
  for (const p of [`.zcode/agents/${name}.md`, `${KIT}/${name}.md`]) {
    try {
      const raw = await files.read(p);
      return { persona: adapter + raw.replace(/^---\n[\s\S]*?\n---\n/, "").trim(), source: p };
    } catch { /* try next source */ }
  }
  return { persona: fallback, source: "fallback" };
}

// The frozen API contract. Every area implements exactly this; no renames anywhere.
const CONTRACT = `
FROZEN CONTRACT (every area implements exactly these names and behaviors; no renames):

Workspace: root Cargo.toml becomes a workspace (members = ["crates/nsh-qr-code"], resolver = "3",
workspace.package: version "0.1.0", edition "2024", license "MIT", repository
"https://github.com/SuperNSH/qr-code", authors ["hmziqrs"]). The crate nsh-qr-code (lib name
nsh_qr_code) lives at crates/nsh-qr-code; the existing root src/lib.rs scaffold moves there
(the status() stub is deleted). Layout mirrors .refs/biometrics.

Errors (src/error.rs, exactly the shape of .refs/biometrics/crates/nsh-biometrics/src/error.rs):
ErrorKind, non_exhaustive; QrError { kind: ErrorKind, status: Option<i64> } with kind(), code(),
status(), with_status(i64), message(); Display prints the code. Kind -> code -> message:
  DecodeFailed      ERR_QR_DECODE_FAILED       "A QR code was found but its payload could not be read. Try a clearer frame."
  PayloadTooLarge   ERR_QR_PAYLOAD_TOO_LARGE   "The payload does not fit a QR code. Shorten it or lower the error correction."
  InvalidArgument   ERR_QR_INVALID_ARGUMENT    "An argument is empty or not allowed, such as a frame whose size does not match its data."
  PermissionDenied  ERR_QR_PERMISSION_DENIED   "Camera permission was denied. Grant it in the system settings."
  Unsupported       ERR_QR_UNSUPPORTED         "No camera is registered on this platform yet; live frames come from the camera repo."
  AlreadyRunning    ERR_QR_ALREADY_RUNNING     "A scan is already running. Stop it before starting another."
  Cancelled         ERR_QR_CANCELLED           "The scan was cancelled."
  Timeout           ERR_QR_TIMEOUT             "The scan timed out. Try again."
  FrameSourceClosed ERR_QR_FRAME_SOURCE_CLOSED "The frame source stopped feeding frames."
  Backend           ERR_QR_BACKEND             "The platform or frame source failed unexpectedly."

Types (src/frame.rs, src/backend.rs, src/options.rs):
  Frame { width: usize, height: usize, luma: Vec<u8> }; Frame::new(width, height, luma) ->
    Result<Frame, QrError>, refusing a length that does not equal width*height or a zero area
    with InvalidArgument. Luma is grayscale, row-major, 0 black, 255 white.
  FrameFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>
  trait FrameSource: Send + Sync {
      fn next_frame(&self) -> FrameFuture<'_, Result<Frame, QrError>>;
      fn cancel(&self);
  }
  Each next_frame call resolves exactly once; a cancelled or ended source keeps answering
  FrameSourceClosed; implementations never wait unboundedly. This trait is the seam the camera
  repo (SuperNSH/nsh-camera, a different repo, read-only) will implement for live capture.
  PermissionState { Undetermined, Granted, Denied }
  BackendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>
  trait Backend: Send + Sync {
      fn camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>>;
      fn request_camera_permission(&self) -> BackendFuture<'_, Result<PermissionState, QrError>>;
      fn frame_source(&self) -> BackendFuture<'_, Result<Option<Arc<dyn FrameSource>>, QrError>>;
  }
  frame_source returning Ok(None) means no camera is registered yet.
  EccLevel { Low, Medium, Quartile, High } (qrcode-rust L/M/Q/H; rqrr ecc_level 0/1/2/3, mapping documented).
  ScanOptions { timeout: Option<Duration> } (Default: none).
  GenerateOptions { ec_level: Option<EccLevel>, quiet_zone: u32 } (Default: None, 4).
  DecodedQr { text: String, bytes: Vec<u8>, ec_level: EccLevel, version: u8,
              corners: [(usize, usize); 4] } — corners top-left, top-right, bottom-right, bottom-left.
  QrMatrix { size: usize, modules: Vec<bool>, ec_level: EccLevel, version: u8 } (row-major).

Functions and behavior (src/decode.rs, src/generate.rs, src/scanner.rs):
  decode_frame(frame: &Frame) -> Result<Vec<DecodedQr>, QrError> — rqrr via
  PreparedImage::prepare_from_greyscale (no img feature), detect_grids, decode_to. No grids:
  Ok(empty vec). All found grids fail to decode: DecodeFailed. Partial: the decodable ones.
  rqrr DeQRError mapping: DataEcc, FormatEcc, UnknownDataType, DataUnderflow, InvalidVersion,
  InvalidGridSize -> DecodeFailed; DataOverflow -> PayloadTooLarge; IoError -> Backend.
  Bytes come from decode_to; text = String::from_utf8_lossy, so non-UTF-8 payloads keep their
  bytes with lossy text.
  generate(data: &[u8], options: &GenerateOptions) -> Result<QrMatrix, QrError> — qrcode-rust
  with features std+svg only. Empty data: InvalidArgument. Payload too long for a QR code:
  PayloadTooLarge. ec_level None picks by payload length: <= 255 bytes High, <= 1071 Medium,
  else Low (document the rationale: smaller payloads get the strongest correction).
  QrMatrix::to_svg(&self) -> String (dark modules, quiet_zone border).
  QrScanner — Clone; new() returns the platform's shared handle via platform::default_backend()
  (OnceLock, like biometrics LocalAuth::new); with_backend(impl Backend + 'static) for tests.
    camera_permission() / request_camera_permission() -> Result<PermissionState, QrError>.
    scan(&self, options: &ScanOptions) -> Result<DecodedQr, QrError> — one scan at a time (a
    second concurrent call gets AlreadyRunning); takes backend.frame_source() (None ->
    Unsupported); loops next_frame -> decode_frame and returns the first DecodedQr. Empty or
    DecodeFailed frames keep the scan going (blur, rotation, low light are ordinary frames).
    FrameSourceClosed ends the scan with that error. options.timeout ends it with Timeout
    (tokio::select!; on wasm32, which has no timer runtime, accept the option and scan without
    it, like biometrics does). Resolves exactly once; never hangs past its timeout; never panics.
    Gate mechanics (generation ids, free_after_timeout, armed Drop owner) mirror
    .refs/biometrics/crates/nsh-biometrics/src/auth.rs.
    stop(&self) -> Result<bool, QrError> — the running scan resolves Cancelled; returns whether
    one was running; a second stop returns false.
  FakeBackend + FakeFrameSource (src/fake.rs, Clone, Mutex state, counters — the shape of
  biometrics FakeBackend): set_camera_permission(Result<PermissionState, QrError>),
  set_frame_source(Option<Arc<dyn FrameSource>>), read/request counters; FakeFrameSource feeds a
  queue of frames then FrameSourceClosed, cancel() closes it, counts next_frame calls. Exported
  for tests and apps without a camera.

  native.rs: #[class] NativeQrCode exposing to TSX what the TS package's rust.ts wraps: new(),
  decodeImage(...), generate(...), cameraPermission(), requestCameraPermission(), scan(...),
  stop(); async members like biometrics native.rs; rejections carry the stable code via
  JsError::Error(...).with_code(error.code()). Verify parameter and return types against
  .refs/nsh/crates/nsh-core source; keep to plain types (bool, numbers, String, Vec, Option).
  lib.rs mirrors biometrics lib.rs: mod list, pub use of the public API, and the
  #[native_module("nsh-qr-code/native")] registration for NativeQrCode.

Platform (src/platform/): mod.rs mirrors biometrics platform/mod.rs cfg gates (android /
target_vendor apple / linux / windows / wasm32 web / unsupported fallback for everything else)
plus default_backend() -> Arc<dyn Backend>. unsupported.rs answers Undetermined and Unsupported.
Each real bridge implements camera_permission + request_camera_permission through its
platform's own API and frame_source() -> Ok(None) with a doc comment naming
SuperNSH/nsh-camera as the future frame provider. Coded errors only; never panic; never block
unboundedly. web.rs (wasm32): navigator.permissions.query({name: "camera"}) when available
(Undetermined when unavailable), request via getUserMedia({video: true}) then stop the track.

TypeScript package packages/nsh-qr-code mirrors the crate with the same names, camelCased, in
biometrics' file shapes: errors.ts (QrErrorCode union of the ten codes, QrError class with
code + message, qrError(code), asQrError(error) parsing ERR_NSH_HOST_ERROR rejections whose
message starts with "ERR_QR_...: ", unknown -> ERR_QR_BACKEND); frames.ts (Frame
{width, height, luma: Uint8Array}, FrameSource {nextFrame(): Promise<Frame>, cancel(): void});
options.ts (PermissionState "undetermined"|"granted"|"denied", EccLevel
"low"|"medium"|"quartile"|"high", ScanOptions {timeoutMs?: number}, GenerateOptions
{ecLevel?: EccLevel, quietZone?: number}); backend.ts (QrBackend interface matching Backend);
decode.ts decodeFrame(frame): Promise<DecodedQr[]>; generate.ts generate(data: Uint8Array |
string, options?): Promise<QrMatrix> with the same ECC-by-length rule and QrMatrix.toSvg();
scanner.ts QrScanner class ({backend?: QrBackend} injection, one scan at a time, same codes,
timeoutMs, stop() — biometrics auth.ts gate shape); memory.ts (memoryBackend(config) +
fakeFrameSource(frames) — biometrics memory.ts shape); rust.ts (rustBackend() via
"nsh-qr-code/native"); hooks.ts (useQrScanner mirroring vision-camera's useBarcodeScanner:
onCodeScanned callback, QR only, nsh signal/effect/cleanup like availability.ts); index.ts
re-exports; App.tsx demo (biometrics App.tsx shape: mount, signals, testID texts, Scan/Stop
buttons); package.json / tsconfig.json / nsh.json copied from biometrics' shapes (nsh.json:
name "nsh-qr-code tests", id "dev.supernsh.qrcode.tests", version "0.1.0", build 1, rust
"../../crates/nsh-qr-code", capabilities {}).
`;

const COMMON = `
Hard rules for this run: Work directly in /Users/hmziq/fo/nsh-qr-code (this checkout only).
No branches, no worktrees, no commits, no pushes, no git commands at all. Never edit anything
under .refs/ (the six cloned reference repos are read-only reference material), never edit
/Users/hmziq/fo/nsh-camera or any other repo. AGENTS.md rules are hard: comments are 1-2 lines
and state a non-obvious constraint; dead code is deleted; no speculative abstractions; docs and
README state only what a passing CI run has verified. Every operation resolves exactly once —
success, a coded failure, or a cancel; nothing hangs; nothing panics on user data. Read
AGENTS.md, docs/brief.md and every file you own before writing. Read reference code under
.refs/ before inventing anything — never invent an API name: verify it in the reference or the
resolved crate source. cargo and nsh run locally here as pure toolchain checks (real CI runs on
GitHub); do not build or run emulators or simulators. If a check is impossible to pass or your
instructions contradict each other, escalate and say so plainly rather than working around it.
Return {done, summary, contractLine, blockers?}: contractLine must be one sentence copied
VERBATIM from your specialist contract that most shaped the work.
`;

// The pinned test names. docs/goals.md maps every edge case to these; the tests area
// implements exactly these.
const RUST_TEST_NAMES = `
crates/nsh-qr-code/tests/errors.rs: every_kind_has_its_code_and_message
crates/nsh-qr-code/tests/frames.rs: frame_rejects_size_mismatch, frame_rejects_zero_area
crates/nsh-qr-code/tests/generate.rs: generate_rejects_empty_data,
  generate_rejects_payload_too_large, generate_picks_ec_by_payload_length,
  round_trip_each_ec_level, svg_contains_quiet_zone_and_matrix
crates/nsh-qr-code/tests/decode.rs: decode_frame_without_codes_is_empty,
  decode_corrupt_code_reports_decode_failed, decode_finds_multiple_codes,
  decode_rotated_still_decodes, decode_non_utf8_keeps_bytes
crates/nsh-qr-code/tests/scanner.rs: scan_returns_first_code,
  scan_second_call_is_already_running, stop_resolves_cancelled_and_returns_true,
  stop_without_scan_returns_false, scan_timeout_resolves_timeout,
  closed_source_ends_scan_with_frame_source_closed, scan_without_camera_is_unsupported,
  permission_states_round_trip`;

const TS_TEST_NAMES = `
packages/nsh-qr-code/tests/errors.test.ts: codes_match_the_rust_table,
  as_error_parses_host_prefix, unknown_becomes_backend
packages/nsh-qr-code/tests/generate.test.ts: generate_rejects_empty_data,
  generate_rejects_payload_too_large, generate_picks_ec_by_payload_length,
  round_trip_generate_then_decode, to_svg_matches_matrix
packages/nsh-qr-code/tests/decode.test.ts: decode_frame_without_codes_is_empty,
  decode_finds_multiple_codes
packages/nsh-qr-code/tests/scanner.test.ts: scan_returns_first_code,
  second_call_is_already_running, stop_resolves_cancelled, stop_without_scan_returns_false,
  timeout_resolves_timeout, closed_source_ends_scan, memory_backend_permission_round_trip
packages/nsh-qr-code/tests/hooks.test.tsx: use_qr_scanner_reports_codes`;

interface AuditFinding {
  /** Workspace-relative path, with a line when it applies. */
  where: string;
  /** One sentence: what is wrong, not how to fix it. */
  problem: string;
  /** How much it matters. Reserve "high" for a crash, a hang, or a wrong result. */
  severity: "low" | "medium" | "high";
}

interface AuditFindings {
  findings: AuditFinding[];
}

interface ImplResult {
  /** True when the area's own files are written and its scoped check (if any) was left green. */
  done: boolean;
  /** Two or three sentences on what was built. */
  summary: string;
  /** One sentence copied verbatim from the specialist contract. */
  contractLine: string;
  /** Anything that blocks this area and needs a decision. */
  blockers?: string[];
}

interface AreaOutcome {
  name: string;
  status: "done" | "failed";
  /** What was implemented, or what failed. */
  summary: string;
  /** False when the audit cap hit with findings still open. */
  converged: boolean;
  /** Audit rounds actually used. */
  rounds: number;
  /** False when the contractLine check could not confirm the specialist body arrived. */
  personaVerified: boolean;
  notes: string[];
}

interface AreaPlan {
  name: string;
  specialist: string;
  headline: string;
  ask: string;
  auditLens: string;
  /** Scoped deterministic cargo checks, run before each audit ask. */
  check?: Array<{ args: string[]; timeoutMs: number }>;
}

const AREAS: AreaPlan[] = [
  {
    name: "docs-goals",
    specialist: "z-nsh-engineer",
    headline: "goals, acceptance criteria, error contract, edge cases, decisions, README",
    ask: `Area (a). You own exactly three files: docs/goals.md, docs/decisions.md, README.md. Do not touch any other file. First read AGENTS.md, docs/brief.md, .refs/biometrics/README.md, .refs/biometrics/docs/*.md, .refs/mobile_scanner/README.md and .refs/react-native-vision-camera/docs/content/docs/barcode-scanner.mdx. Then write:
- docs/goals.md: the goals (decode QR codes from frames and images; generate QR codes; define the FrameSource trait here as the seam for the camera repo SuperNSH/nsh-camera, which this repo does not include and must not edit); acceptance criteria (each mapped to a gate: cargo fmt --all -- --check, cargo clippy --all-targets -- -D warnings, cargo test --all-targets, nsh check, nsh test — all run on GitHub CI, never on a developer machine); the error enum table EXACTLY as the CONTRACT lists it (kind, stable code, message); and an edge-case list where every case maps to a named test. Cover at least: empty generate payload; payload over capacity; frame size mismatch; zero-size frame; no code in the frame; corrupt code (ECC failure); multiple codes in one frame; rotation still decodes; blur and low light reduce decode rate (decode only sees the frames it is given — the caller controls the rate); non-UTF-8 payload keeps bytes with lossy text; camera busy (AlreadyRunning); permission denied per platform; cancel resolves Cancelled; timeout resolves Timeout; closed frame source; double stop returns false; scan on a platform with no camera (Unsupported, web included); generation ECC picked by payload length. The named tests are EXACTLY these, implemented by a sibling area — write them verbatim:\n${RUST_TEST_NAMES}\n${TS_TEST_NAMES}
- docs/decisions.md: move the open decisions to Decided with one-line reasons: library choice rqrr 0.11 (decode; pure Rust; luma prepared images, no img feature needed) and qrcode 0.14 (generate; std+svg features); NSH version 0.4.0 (matches biometrics and the installed nsh); platforms in scope: Apple, Android, Windows, Linux, web — permission bridges now, live frame sources arrive with the camera repo; reference note: react-native-vision-camera's code scanner moved into its react-native-vision-camera-barcode-scanner package inside the same monorepo (cloned at .refs/react-native-vision-camera/packages/react-native-vision-camera-barcode-scanner), with mrousavy/react-native-data-scanner as the one-shot Apple alternative — that is the canonical location.
- README.md: update Layout for the workspace (crates/nsh-qr-code, packages/nsh-qr-code, docs/, site/, .refs/ ignored) and keep the status honest: implemented, unverified until a CI run passes (AGENTS.md: a claim with no passing run is unverified). Checks section: the GitHub Actions gates.
Style: direct, concrete, no marketing filler, like the biometrics docs.`,
    auditLens: `Audit ONLY docs/goals.md, docs/decisions.md and README.md. Read them plus AGENTS.md and docs/brief.md. Check: the error table matches the CONTRACT codes verbatim (all ten, exact strings); every edge case maps to a named test from the pinned list and every pinned test name appears; no status claim says verified/done in a way that implies a passing CI run; the decisions match the brief's facts (rqrr 0.11, qrcode 0.14, NSH 0.4.0, five platforms, vision-camera barcode-scanner package as the canonical RN code-scanner location); README layout matches the CONTRACT workspace layout; docs read like a competent human wrote them and contain no filler. Do not edit any file.`,
  },
  {
    name: "docs-parity",
    specialist: "z-nsh-engineer",
    headline: "API parity table against mobile_scanner and vision-camera barcode-scanner, plus the status page",
    ask: `Area (b). You own exactly two files: docs/parity.md and site/index.html. Do not touch any other file. First read AGENTS.md, docs/brief.md, the existing docs/parity.md and site/index.html. Then read the reference APIs in depth: .refs/mobile_scanner/lib/src/mobile_scanner_controller.dart, mobile_scanner_exception.dart, mobile_scanner.dart, the enums/ and objects/ directories; and .refs/react-native-vision-camera/packages/react-native-vision-camera-barcode-scanner/src/ (index.ts, useBarcodeScanner.ts, useBarcodeScannerOutput.ts, views/, specs/) plus the permission hooks in .refs/react-native-vision-camera/packages/react-native-vision-camera/src/hooks/. Write docs/parity.md: a table of EVERY public API of both references — mobile_scanner: MobileScanner widget and its properties, MobileScannerController (start, stop, dispose, autoStart, torch, zoom, analyzeImage, barcodesStream, facing, formats), Barcode and its objects (BarcodeType, BarcodeFormat, corner points, size, value), MobileScannerErrorCode (all eight) and MobileScannerAuthorizationState; vision-camera barcode-scanner: useBarcodeScanner, the <CodeScanner /> view, useBarcodeScannerOutput/CameraOutput, BarcodeFormat union, BarcodeValueType, Point, Rect, and the core permission APIs (requestPermission/usePermission). For each row: what ours does instead, named from the CONTRACT (QrScanner.scan/stop, camera_permission/request_camera_permission, decode_frame, generate, FrameSource, DecodedQr, QrMatrix.to_svg, the ten ERR_QR_* codes), and an honest status: implemented features say "implemented — awaiting first passing CI run" (AGENTS.md: a claim with no passing run is unverified); features we deliberately do not have (torch, zoom, camera selection, barcode formats other than QR, Flutter/RN widgets themselves) say "not ours — camera controls belong to the camera repo (nsh-camera)" or "out of scope: QR only", with a one-line reason. Keep the file's existing top-note style. Then update site/index.html so its rows match docs/parity.md exactly and keep it a plain static page.`,
    auditLens: `Audit ONLY docs/parity.md and site/index.html. Read them, then spot-check at least eight rows against the actual reference sources under .refs/mobile_scanner/lib/src and .refs/react-native-vision-camera/packages/react-native-vision-camera-barcode-scanner/src. Check: no public reference API is missing a row; ours-column names match the CONTRACT exactly; no row claims a passing CI run; torch/zoom/camera rows say the camera repo owns them; site/index.html matches the table. Do not edit any file.`,
  },
  {
    name: "rust-core",
    specialist: "z-rust-engineer",
    headline: "Rust workspace, core API, error types, frame-source trait, scanner, fakes, native class",
    ask: `Area (c). You own the Rust workspace skeleton and the whole core crate EXCEPT the bodies of five platform bridge files (siblings replace those): you create crates/nsh-qr-code/src/platform/{apple,android,windows,linux,web}.rs as COMPILING stubs — a NativeBackend struct implementing Backend with Undetermined permissions and frame_source -> Ok(None), doc comment naming SuperNSH/nsh-camera — each exactly the shape the sibling will grow into, so the crate compiles at every moment. Also owned by you: root Cargo.toml (convert to the workspace per the CONTRACT), the removal of the old root src/ directory (move lib content per the CONTRACT; the stub status() is deleted), crates/nsh-qr-code/Cargo.toml with EXACTLY these dependencies (verify they resolve; if a pinned version does not exist on crates.io, take the nearest published version and record the deviation in your result summary):
[package] name "nsh-qr-code", lib name nsh_qr_code, workspace-inherited metadata;
[dependencies] nsh-core = "0.4.0", thiserror = "2", tokio = { version = "1", default-features = false, features = ["sync", "time"] }, rqrr = { version = "0.11", default-features = false }, qrcode = { version = "0.14", default-features = false, features = ["std", "svg"] };
[target.'cfg(target_vendor = "apple")'.dependencies] objc2-av-foundation = { version = "0.3", default-features = false, features = ["std", "AVCaptureDevice"] };
[target.'cfg(target_os = "android")'.dependencies] jni = "0.22", ndk-context = "0.1";
[target.'cfg(target_os = "windows")'.dependencies] windows = { version = "0.62", features = ["Media_Capture", "Foundation"] }, windows-future = { version = "0.3", features = ["std"] };
[target.'cfg(target_arch = "wasm32")'.dependencies] wasm-bindgen = "0.2", web-sys = { version = "0.3", features = ["Window", "Navigator", "MediaDevices", "MediaStreamConstraints", "MediaStream", "MediaStreamTrack", "Permissions", "PermissionStatus"] };
[dev-dependencies] tokio = { version = "1", default-features = false, features = ["rt", "time", "macros", "test-util" ] };
and src/{lib,error,frame,backend,options,decode,generate,scanner,fake,native}.rs plus src/platform/{mod.rs,unsupported.rs} — all implementing the CONTRACT. Read .refs/biometrics/crates/nsh-biometrics/src/{lib,error,backend,auth,fake,native}.rs and src/platform/mod.rs as the style and mechanics source, .refs/rqrr/src/{lib.rs,prepare.rs} (PreparedImage::prepare_from_greyscale — no img feature), .refs/qrcode-rust/src/{lib.rs,types.rs,render/} and .refs/nsh/crates/nsh-core/src/ for the #[class]/#[native_module]/JsObject/JsError/Instance types native.rs uses. Add inline #[cfg(test)] unit tests inside your own files for: the error code table, Frame::new validation, the generate ECC rule at its 255/1071 boundaries, svg output shape, a decode round-trip through FakeFrameSource, and the scanner gate (AlreadyRunning, Cancelled, Timeout — tokio current_thread runtime, biometrics tests/auth.rs shape). When done run this scoped check and leave it green: cargo check --manifest-path crates/nsh-qr-code/Cargo.toml --target-dir target/core-check`,
    auditLens: `Audit ONLY the Rust workspace files: root Cargo.toml, crates/nsh-qr-code/Cargo.toml, crates/nsh-qr-code/src/** (you may read the bridge stubs but their real bodies belong to sibling areas' audits). Check against the CONTRACT: names, signatures and the ten codes verbatim; QrScanner gate mechanics match biometrics auth.rs (one scan at a time, generation-guarded cancel and timeout, resolves exactly once); decode and generate behaviors match (rqrr mapping table, ECC-by-length rule, lossy text with bytes preserved); no unwrap/expect on user data paths, no panic, no unbounded wait; comments 1-2 lines stating non-obvious constraints; no dead code; inline unit tests are real assertions; native.rs types verified against .refs/nsh/crates/nsh-core. Do not edit any file.`,
    check: [{ args: ["check", "--manifest-path", "crates/nsh-qr-code/Cargo.toml", "--target-dir", "target/core-check"], timeoutMs: 480000 }],
  },
  {
    name: "bridge-apple",
    specialist: "z-rust-engineer",
    headline: "Apple camera-permission bridge over AVFoundation",
    ask: `Area (d, Apple). You own exactly one file: crates/nsh-qr-code/src/platform/apple.rs — replace the compiling stub a sibling created with the real bridge. First read the CONTRACT, .refs/biometrics/crates/nsh-biometrics/src/platform/apple.rs (style), and the objc2-av-foundation API (source on crates.io docs via the dependency .refs/nsh does not carry it — verify names against the crate source that 'cargo check' fetches, or the module docs shipped in the registry checkout under ~/.cargo/registry). Implement NativeBackend per the CONTRACT: camera_permission reads AVCaptureDevice authorizationStatus(forMediaType: AVMediaTypeVideo) — .notDetermined -> Undetermined, .authorized -> Granted, .denied/.restricted -> Denied; request_camera_permission awaits AVCaptureDevice requestAccess(forMediaType:) and maps the answer the same way; frame_source() -> Ok(None) with a doc comment naming SuperNSH/nsh-camera as the future provider. Coded errors only (Backend with the platform status via with_status where one exists); never panic; never block unboundedly (await the async request; a scan's own timeout still bounds the caller). A sibling owns the rest of Cargo.toml, but if your code needs a different feature set you may edit ONLY the [target.'cfg(target_vendor = "apple")'.dependencies] block of crates/nsh-qr-code/Cargo.toml — nothing else in that file; record any such change in your result summary. The crate manifest and your file's stub may not exist yet when you start (a sibling creates them in parallel): if cargo cannot find the manifest, say so in your result and finish your file; the audit loop re-runs the check next round. When the manifest exists, run this scoped check and leave it green: cargo check --manifest-path crates/nsh-qr-code/Cargo.toml --target-dir target/apple-host`,
    auditLens: `Audit ONLY crates/nsh-qr-code/src/platform/apple.rs (read src/platform/mod.rs and the Backend trait for context). Check: the AVFoundation API names are real (verify against the fetched crate source under ~/.cargo/registry or cargo check output — no invented selectors); the permission mapping matches mobile_scanner's authorization semantics (.notDetermined/.authorized/.denied/.restricted); coded errors only, no unwrap/panic on the objc2 boundary, no unbounded block; frame_source is Ok(None) with the camera-repo doc comment; the cfg gate in src/platform/mod.rs is untouched. Do not edit any file.`,
    check: [{ args: ["check", "--manifest-path", "crates/nsh-qr-code/Cargo.toml", "--target-dir", "target/apple-host"], timeoutMs: 480000 }],
  },
  {
    name: "bridge-android",
    specialist: "z-rust-engineer",
    headline: "Android camera-permission bridge over JNI",
    ask: `Area (d, Android). You own exactly one file: crates/nsh-qr-code/src/platform/android.rs — replace the compiling stub a sibling created with the real bridge. First read the CONTRACT and .refs/biometrics/crates/nsh-biometrics/src/platform/android.rs (JNI + ndk-context style, error discipline). Implement NativeBackend per the CONTRACT with jni + ndk-context: camera_permission reads the attached Android context via ndk_context — Context.checkSelfPermission("android.permission.CAMERA") on API 23+, Granted before M; no attached context -> Undetermined (documented, like biometrics' NoActivity discipline); Denied -> Denied. request_camera_permission finds the current activity and calls requestPermissions with CAMERA, mapping the result (granted -> Granted, otherwise Denied); no activity -> Undetermined, never panic. JNI errors map to Backend with the platform status via with_status where one exists. frame_source() -> Ok(None) with the camera-repo doc comment. Never block unboundedly; the scanner's timeout still bounds callers. You may edit ONLY the [target.'cfg(target_os = "android")'.dependencies] block of crates/nsh-qr-code/Cargo.toml if needed — nothing else in that file; record any change in your result summary. If the crate manifest does not exist yet when you start (a sibling creates it in parallel), finish your file and say so; the audit loop re-runs the check. When the manifest exists, run this scoped check and leave it green: cargo check --manifest-path crates/nsh-qr-code/Cargo.toml --target aarch64-linux-android --target-dir target/android-check`,
    auditLens: `Audit ONLY crates/nsh-qr-code/src/platform/android.rs (read src/platform/mod.rs and the Backend trait for context). Check against .refs/biometrics/crates/nsh-biometrics/src/platform/android.rs: JNI usage is sound (no leaked local refs across calls, no panic on the JNI boundary, attach discipline matches), the permission mapping matches Android semantics (pre-M Granted, API 23+ checkSelfPermission, requestPermissions needs an activity), coded errors only, frame_source Ok(None) with the camera-repo doc, the cfg gate untouched. Do not edit any file.`,
    check: [{ args: ["check", "--manifest-path", "crates/nsh-qr-code/Cargo.toml", "--target", "aarch64-linux-android", "--target-dir", "target/android-check"], timeoutMs: 480000 }],
  },
  {
    name: "bridge-desktop-web",
    specialist: "z-rust-engineer",
    headline: "Windows, Linux and web camera-permission bridges",
    ask: `Area (d, Windows + Linux + web). You own exactly three files: crates/nsh-qr-code/src/platform/windows.rs, crates/nsh-qr-code/src/platform/linux.rs and crates/nsh-qr-code/src/platform/web.rs — replace each compiling stub a sibling created with the real bridge. First read the CONTRACT, .refs/biometrics/crates/nsh-biometrics/src/platform/{windows,linux}.rs (style), and the docs of the references on what these platforms do for camera permission (mobile_scanner and vision-camera do not support Windows/Linux camera scanning — record that honestly in doc comments). Implement per the CONTRACT:
- windows.rs: Windows has no camera-permission broker query an unpackaged app can make; follow the references' behavior: camera_permission -> Ok(Granted) (documented why), request_camera_permission -> Ok(Granted); frame_source() -> Ok(None) with the camera-repo doc comment. You may use the pinned windows/windows-future dependencies for a compile-verified probe if you find a cheap one, or leave them unused — but no panic and no unbounded block either way.
- linux.rs: same honest semantics (no camera-permission broker outside sandbox portals): Granted, documented; frame_source Ok(None) + camera-repo note; no extra dependencies.
- web.rs (wasm32, wasm-bindgen + web-sys): camera_permission queries navigator.permissions.query({name: "camera"}) when the API exists — granted -> Granted, denied -> Denied, anything else (API missing, prompt state) -> Undetermined; request_camera_permission calls navigator.mediaDevices.getUserMedia({video: true}), stops the tracks it got, maps success -> Granted and failure -> PermissionDenied (NotAllowedError) or Backend otherwise; never hang — the browser's own prompt resolves the promise, and a scan's timeout still bounds the caller (document that, like biometrics' wasm note); frame_source() -> Ok(None) + camera-repo note.
You may edit ONLY the [target.'cfg(target_os = "windows")'] and [target.'cfg(target_arch = "wasm32")'.dependencies] blocks of crates/nsh-qr-code/Cargo.toml if needed — nothing else in that file; record any change in your result summary. If the crate manifest does not exist yet when you start, finish your files and say so; the audit loop re-runs the checks. Targets x86_64-pc-windows-gnu, x86_64-unknown-linux-gnu and wasm32-unknown-unknown are installed locally; if one were missing you would run rustup target add <name> first and report honestly if that failed. When the manifest exists, run these three scoped checks in order and leave all green:
cargo check --manifest-path crates/nsh-qr-code/Cargo.toml --target x86_64-pc-windows-gnu --target-dir target/windows-check
cargo check --manifest-path crates/nsh-qr-code/Cargo.toml --target x86_64-unknown-linux-gnu --target-dir target/linux-check
cargo check --manifest-path crates/nsh-qr-code/Cargo.toml --target wasm32-unknown-unknown --target-dir target/web-check`,
    auditLens: `Audit ONLY crates/nsh-qr-code/src/platform/{windows,linux,web}.rs (read src/platform/mod.rs and the Backend trait for context). Check: the Windows and Linux Granted semantics are honestly documented as matching the references' lack of a camera-permission broker (no invented WinRT/polkit API); web.rs uses real web-sys API names (verify against the fetched crate source under ~/.cargo/registry), stops the tracks it opens, maps NotAllowedError to PermissionDenied, and carries the wasm no-timer note; all three return Ok(None) frame sources with the camera-repo doc comment; coded errors only, no panic, no unbounded block; the cfg gates in src/platform/mod.rs are untouched. Do not edit any file.`,
    check: [
      { args: ["check", "--manifest-path", "crates/nsh-qr-code/Cargo.toml", "--target", "x86_64-pc-windows-gnu", "--target-dir", "target/windows-check"], timeoutMs: 480000 },
      { args: ["check", "--manifest-path", "crates/nsh-qr-code/Cargo.toml", "--target", "x86_64-unknown-linux-gnu", "--target-dir", "target/linux-check"], timeoutMs: 480000 },
      { args: ["check", "--manifest-path", "crates/nsh-qr-code/Cargo.toml", "--target", "wasm32-unknown-unknown", "--target-dir", "target/web-check"], timeoutMs: 480000 },
    ],
  },
  {
    name: "ts-package",
    specialist: "z-nsh-engineer",
    headline: "TypeScript package mirroring the Rust API",
    ask: `Area (e). You own everything under packages/nsh-qr-code/ EXCEPT its tests/ directory (a sibling owns the tests): package.json, tsconfig.json, nsh.json, App.tsx and src/. Do not touch any other path. First read the CONTRACT, .refs/biometrics/packages/nsh-biometrics/{package.json,tsconfig.json,nsh.json,App.tsx,src/*.ts} (your style source, file by file), .refs/biometrics/packages/nsh-biometrics/tests/*.test.tsx (what nsh/test tests look like), and .refs/react-native-vision-camera/packages/react-native-vision-camera-barcode-scanner/src/useBarcodeScanner.ts (the hook shape to mirror). Build the package per the CONTRACT's TypeScript section: src/{index,errors,options,frames,backend,decode,generate,scanner,memory,rust,hooks}.ts with the exact camelCased names, the same ten codes and messages as the Rust crate, the same ECC-by-length rule, QrScanner's one-scan gate with timeoutMs and stop() (biometrics auth.ts shape), memoryBackend + fakeFrameSource (biometrics memory.ts shape), rustBackend() importing NativeQrCode from "nsh-qr-code/native" (biometrics rust.ts shape), useQrScanner in hooks.ts (nsh signal/effect/cleanup like availability.ts, onCodeScanned callback, QR only), index.ts re-exports, App.tsx demo (biometrics App.tsx shape with testID texts and Scan/Stop buttons), package.json/tsconfig.json/nsh.json copied from biometrics' shapes with the values the CONTRACT lists. Keep the package dependency-free: nsh and nsh/test are runtime-provided; never npm install anything. Note: tsconfig.json extends ./.nsh/tsconfig.json which nsh check generates — that is expected and correct; the type gate runs at the end of the run. Write types strict-clean by reading, since you cannot typecheck locally.`,
    auditLens: `Audit ONLY packages/nsh-qr-code/{package.json,tsconfig.json,nsh.json,App.tsx,src/**} (the sibling tests/ dir is out of scope). Check against the CONTRACT and .refs/biometrics/packages/nsh-biometrics: every exported name exists and matches camelCase-for-snake_case exactly (ten codes with the Rust strings, QrScanner.scan/stop/cameraPermission/requestCameraPermission, generate ECC rule, toSvg, asQrError ERR_NSH_HOST_ERROR parsing, memoryBackend + fakeFrameSource, rustBackend, useQrScanner); the busy gate resolves each call exactly once (no double resolve on timeout + answer); no invented nsh API (verify signal/effect/cleanup/mount against .refs/biometrics usage); no dependency added to package.json; nsh.json points at ../../crates/nsh-qr-code. Do not edit any file.`,
  },
  {
    name: "qr-tests",
    specialist: "z-nsh-engineer",
    headline: "Rust and TypeScript tests for every goal, edge case and error path, plus CI",
    ask: `Area (f). You own exactly: crates/nsh-qr-code/tests/ (five files), packages/nsh-qr-code/tests/ (five files) and .github/workflows/ci.yml. Do not touch any other path. First read the CONTRACT, AGENTS.md, docs/brief.md, .refs/biometrics/crates/nsh-biometrics/tests/auth.rs (Rust test harness style) and .refs/biometrics/packages/nsh-biometrics/tests/auth.test.tsx (nsh/test style). Write the pinned tests EXACTLY under these names (docs/goals.md documents the same list):\n${RUST_TEST_NAMES}\n${TS_TEST_NAMES}
Rust integration tests: public-API only (use nsh_qr_code::{...}), FakeBackend/FakeFrameSource-driven, tokio current_thread runtime, real assertions — every edge case and error path from the CONTRACT (the codes table, frame validation, generate rules and boundaries, round-trips per ECC level via your own generate -> Frame -> decode_frame, corrupted matrix beyond ECC -> DecodeFailed, multi-code frames, rotated luma still decodes, non-UTF-8 bytes preserved with lossy text, scanner gate: first code, AlreadyRunning, stop -> Cancelled, double stop false, timeout, closed source, Unsupported without a camera, permission round-trip). TS tests mirror the same behaviors through the TS package's memoryBackend/fakeFrameSource with the exact pinned names, importing from "nsh/test" and "../src". Build QR matrices in tests with the package's own generate(), never binary fixtures. Then rewrite .github/workflows/ci.yml mirroring .refs/biometrics/.github/workflows/ci.yml for THIS repo: keep its action pins and NSH v0.4.0 install steps; a format job (cargo fmt --all -- --check); a rust job on ubuntu/macos/windows running cargo clippy -p nsh-qr-code --all-targets -D warnings and cargo test -p nsh-qr-code; a cross-check job (aarch64-apple-ios, aarch64-linux-android with biometrics' NDK env lines, wasm32-unknown-unknown) running cargo clippy -p nsh-qr-code --target <target> -D warnings; a typescript job running, working-directory packages/nsh-qr-code: nsh check; nsh build 2>&1 | tee build.log; ! grep -i warn build.log; nsh test. You may run a scoped check yourself when the crate exists (cargo test --manifest-path crates/nsh-qr-code/Cargo.toml --target-dir target/tests --test scanner); if it does not exist yet, say so — the run's final gates compile and run everything once, so do not run whole-tree suites yourself.`,
    auditLens: `Audit ONLY crates/nsh-qr-code/tests/**, packages/nsh-qr-code/tests/** and .github/workflows/ci.yml. Check: every pinned test name exists with a real assertion (no tautologies, no empty tests); the behaviors tested match the CONTRACT (codes table strings, ECC boundaries, gate semantics, rqrr mapping outcomes); tests use only the public API and the fake backends; the workflow mirrors biometrics ci.yml's pins, jobs, NSH install steps and adds nothing that runs on a developer machine; the typescript job's commands are exactly nsh check, nsh build with the warn grep, nsh test. Do not edit any file.`,
  },
];

function ownerOf(path: string): string {
  const p = path.replace(/\\\\/g, "/").replace(/^\.\//, "");
  if (p.startsWith("crates/nsh-qr-code/src/platform/apple")) return "bridge-apple";
  if (p.startsWith("crates/nsh-qr-code/src/platform/android")) return "bridge-android";
  if (p.startsWith("crates/nsh-qr-code/src/platform/windows") || p.startsWith("crates/nsh-qr-code/src/platform/linux") || p.startsWith("crates/nsh-qr-code/src/platform/web")) return "bridge-desktop-web";
  if (p.startsWith("packages/nsh-qr-code/tests")) return "qr-tests";
  if (p.startsWith("packages/nsh-qr-code")) return "ts-package";
  if (p.startsWith("crates/nsh-qr-code/tests")) return "qr-tests";
  if (p.startsWith("crates/nsh-qr-code")) return "rust-core";
  return "rust-core";
}

function manifestPending(output: string): boolean {
  return output.includes("could not find `Cargo.toml`");
}

function trimOutput(text: string, cap = 4000): string {
  return text.length > cap ? "…" + text.slice(-cap) : text;
}

artifact.board("areas", {
  key: "name",
  status: "phase",
  columns: ["implementing", "auditing", "fixing", "done", "failed"],
  title: "QR code build areas",
});

const handles: Record<string, { impl: ReturnType<typeof agent>; audit: ReturnType<typeof agent> }> = {};

async function runArea(area: AreaPlan): Promise<AreaOutcome> {
  const notes: string[] = [];
  try {
    const { persona, source } = await specialist(area.specialist, "You are a careful, honest senior engineer implementing one area of a parallel workflow.");
    if (source === "fallback") notes.push("specialist file missing: ran on the plain fallback persona");
    report({ name: area.name, phase: "implementing", summary: area.headline }, "areas");
    const impl = agent(`Implementer · ${area.name}`, { system: persona, model: MODELS.pro });
    const audit = agent(`Auditor · ${area.name}`, { system: persona, model: MODELS.pro });
    handles[area.name] = { impl, audit };

    const ask = `${area.ask}\n\nCONTRACT AND RULES:\n${CONTRACT}${COMMON}`;
    const first = await impl.ask<ImplResult>(ask);
    let personaVerified = typeof first.contractLine === "string" && persona.includes(first.contractLine.trim());
    if (!personaVerified) {
      const retry = await impl.ask<ImplResult>(
        "Your result's contractLine was not a verbatim sentence of your specialist contract. Copy ONE sentence from it EXACTLY, character for character.",
      );
      personaVerified = typeof retry.contractLine === "string" && persona.includes(retry.contractLine.trim());
      if (!personaVerified) notes.push("contractLine could not be verified against the specialist body");
    }
    if (first.blockers && first.blockers.length > 0) notes.push(...first.blockers);
    if (!first.done) {
      report({ name: area.name, phase: "failed", summary: first.summary }, "areas");
      return { name: area.name, status: "failed", summary: first.summary, converged: false, rounds: 0, personaVerified, notes };
    }

    let converged = false;
    let rounds = 0;
    for (let round = 1; round <= 3; round++) {
      rounds = round;
      report({ name: area.name, phase: "auditing", summary: `audit round ${round}` }, "areas");
      let feedback = "";
      if (area.check) {
        for (const check of area.check) {
          const run = await world.run("cargo", check.args, { timeoutMs: check.timeoutMs });
          if (run.exitCode === 0) continue;
          const out = trimOutput((run.stderr || "") + "\n" + (run.stdout || ""));
          if (manifestPending(out)) {
            notes.push("scoped check deferred this round: the crate manifest a sibling creates did not exist yet");
            break;
          }
          feedback = `The scoped check \`cargo ${check.args.join(" ")}\` failed (exit ${run.exitCode}). Fix your files until it passes.\n${out}`;
          break;
        }
      }
      if (!feedback) {
        const found = await audit.ask<AuditFindings>(
          `${area.auditLens}\n\nThe CONTRACT you judge against:\n${CONTRACT}\nReturn {"findings": [...]} — ONLY actionable findings with where/problem/severity, high reserved for a crash, hang, or wrong result; an empty array means the area is clean. Never report a PASS note.`,
        );
        if (found.findings.length === 0) {
          converged = true;
          break;
        }
        feedback = `Audit findings to fix in YOUR files:\n${JSON.stringify(found.findings, null, 2)}`;
      }
      report({ name: area.name, phase: "fixing", summary: `fixing audit round ${round}` }, "areas");
      await impl.ask<ImplResult>(
        `${feedback}\n\nFix exactly these in the files you own; touch nothing else; keep the CONTRACT unchanged. Return {done, summary, contractLine}.`,
      );
    }
    if (!converged) notes.push(`audit cap hit after ${rounds} rounds with findings still open`);
    const summary = `${first.summary} ${converged ? `Audit clean after ${rounds} round(s).` : `Audit did NOT converge after ${rounds} round(s).`}`;
    report({ name: area.name, phase: converged ? "done" : "failed", summary }, "areas");
    return { name: area.name, status: "done", summary, converged, rounds, personaVerified, notes };
  } catch (error) {
    const summary = `Area failed: ${error instanceof Error ? error.message : String(error)}`;
    report({ name: area.name, phase: "failed", summary }, "areas");
    return { name: area.name, status: "failed", summary, converged: false, rounds: 0, personaVerified: false, notes };
  }
}

phase("Implement every area in parallel");
log(`${AREAS.length} areas start their own chains: implement, then up to three audit rounds each`);
const outcomes = await Promise.all(AREAS.map(runArea));

phase("Run the full gates once and fix what they catch");
const GATES = [
  { name: "cargo fmt --all -- --check", defaultOwner: "rust-core", args: ["-c", "cd /Users/hmziq/fo/nsh-qr-code && cargo fmt --all -- --check"], timeoutMs: 120000 },
  { name: "cargo clippy --all-targets -- -D warnings", defaultOwner: "rust-core", args: ["-c", "cd /Users/hmziq/fo/nsh-qr-code && cargo clippy --all-targets -- -D warnings"], timeoutMs: 600000 },
  { name: "cargo test --all-targets", defaultOwner: "rust-core", args: ["-c", "cd /Users/hmziq/fo/nsh-qr-code && cargo test --all-targets"], timeoutMs: 600000 },
  { name: "TypeScript typecheck (nsh check)", defaultOwner: "ts-package", args: ["-c", "cd /Users/hmziq/fo/nsh-qr-code/packages/nsh-qr-code && nsh check"], timeoutMs: 600000 },
  { name: "TypeScript tests (nsh test)", defaultOwner: "ts-package", args: ["-c", "cd /Users/hmziq/fo/nsh-qr-code/packages/nsh-qr-code && nsh test"], timeoutMs: 900000 },
];

let gatesGreen = true;
let gateRounds = 0;
for (let round = 1; round <= 3; round++) {
  gateRounds = round;
  log(`full gates, pass ${round}`);
  const failing: string[] = [];
  const feedbackByArea: Record<string, string> = {};
  for (const gate of GATES) {
    const run = await world.run("sh", gate.args, { timeoutMs: gate.timeoutMs });
    if (run.exitCode === 0) {
      log(`${gate.name}: green`);
      continue;
    }
    gatesGreen = false;
    failing.push(gate.name);
    const out = trimOutput((run.stderr || "") + "\n" + (run.stdout || ""));
    const firstPath = /((?:crates|packages|docs|site)\/[^\s:(]+|(?:src|tests)\/[^\s:(]+)/.exec(out)?.[1] ?? "";
    let owner = gate.defaultOwner;
    if (firstPath.startsWith("crates/") || firstPath.startsWith("packages/") || firstPath.startsWith("docs/") || firstPath.startsWith("site/")) {
      owner = ownerOf(firstPath);
    } else if (firstPath.startsWith("tests/")) {
      owner = "qr-tests";
    }
    feedbackByArea[owner] = (feedbackByArea[owner] ?? "") + `\n\nGate "${gate.name}" failed (exit ${run.exitCode}):\n${out}`;
    log(`${gate.name}: red, routed to ${owner}`);
  }
  if (failing.length === 0) {
    gatesGreen = true;
    break;
  }
  if (round === 3) break;
  for (const [areaName, feedback] of Object.entries(feedbackByArea)) {
    const handle = handles[areaName];
    if (!handle) continue;
    await handle.impl.ask<ImplResult>(
      `The whole-tree gates failed in a way that touches your files. Fix ONLY what you own; keep the CONTRACT unchanged; a failing line in a file you do not own belongs to another area, so leave it and say so in summary.\n${feedback}\nReturn {done, summary, contractLine}.`,
    );
  }
}

const unverified: string[] = [];
if (!gatesGreen) unverified.push("the full gate set never went green within 3 fix passes; the failures above name the owners");
for (const o of outcomes) {
  if (!o.converged) unverified.push(`${o.name}: audit did not converge (3 rounds) — findings still open`);
  if (!o.personaVerified) unverified.push(`${o.name}: specialist persona delivery unverified`);
}
unverified.push("windows/linux/android/web bridge code is compile-checked per target locally, but never executed anywhere — no device or emulator runs, per AGENTS.md");
unverified.push("no CI run exists: nothing here is 'verified' in the AGENTS.md sense until the next push to master runs ci.yml");

const failed = outcomes.filter((o) => o.status === "failed");
const notConverged = outcomes.filter((o) => o.status === "done" && !o.converged);
const conclusion = gatesGreen
  ? `All ${outcomes.length} areas landed and the full gate set (cargo fmt/clippy/test, nsh check, nsh test) is green. ${failed.length} failed, ${notConverged.length} never converged. The crate and package still await their first passing CI run.`
  : `${outcomes.length - failed.length} of ${outcomes.length} areas landed, but the full gate set never went green within ${gateRounds} fix passes. Read the failed gates and the per-area notes; ${failed.length} area(s) failed outright and ${notConverged.length} never converged.`;

const report_md = [
  `# nsh-qr-code capability build`,
  ``,
  conclusion,
  ``,
  `## Areas`,
  ...outcomes.map(
    (o) =>
      `- **${o.name}** — ${o.status}${o.converged ? ", audit clean" : ", audit did NOT converge"} after ${o.rounds} audit round(s): ${o.summary}${o.notes.length ? ` Notes: ${o.notes.join(" ")}` : ""}`,
  ),
  ``,
  `## Gates`,
  ...GATES.map((g) => `- ${g.name}`),
  ``,
  `## Not verified`,
  ...unverified.map((u) => `- ${u}`),
].join("\n");

await artifact.markdown("report", report_md, {
  title: "QR code capability build report",
  description: "Per-area outcomes, audit convergence and the honest not-verified list",
  primary: true,
});

return {
  conclusion,
  areas: outcomes.map((o) => ({ name: o.name, status: o.status, summary: o.summary, converged: o.converged, rounds: o.rounds, personaVerified: o.personaVerified, notes: o.notes })),
  findings: [],
  verified: [
    "per-area scoped checks: cargo check for the core (host) and each bridge on its own target (apple host, aarch64-linux-android, x86_64-pc-windows-gnu, x86_64-unknown-linux-gnu, wasm32-unknown-unknown), each with its own target dir",
    "whole-tree gates at the join: cargo fmt --all -- --check; cargo clippy --all-targets -- -D warnings; cargo test --all-targets; nsh check and nsh test in packages/nsh-qr-code",
  ],
  notCovered: unverified,
  notes:
    "User asked for GLM-5.3 on every area: every subagent is pinned account:zai-individual-coding-plan/GLM-5.3$high. Audit budget: 3 rounds per area, clean-or-cap, reported honestly. AGENTS.md forbids building/testing on developer machines; the coordinator's brief resolves the tension: these gates are pure host toolchain checks (cargo check/clippy/test, nsh check/test headless), no emulators, no devices, no cross-run binaries executed. Six references were cloned into .refs/ (gitignored, read-only); no URL failed, and the vision-camera code scanner's canonical home (its react-native-vision-camera-barcode-scanner package) is recorded by the docs area in docs/decisions.md.",
};
