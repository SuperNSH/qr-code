# Parity

Where each public API of the two references lands in this library: [mobile_scanner](https://pub.dev/packages/mobile_scanner) (Flutter) and the barcode scanner of [react-native-vision-camera](https://github.com/mrousavy/react-native-vision-camera), package `react-native-vision-camera-barcode-scanner`. Both are cloned read-only under `.refs/`; every API named below was read there.

Status vocabulary:

- **implemented — awaiting first passing CI run** — the contract names the feature, but no CI run on `master` has passed a test for it, so the row is unverified.
- **not ours — camera controls belong to the camera repo (nsh-camera)** — a camera control this library deliberately lacks; [SuperNSH/nsh-camera](https://github.com/SuperNSH/nsh-camera) owns it.
- **out of scope: QR only** — rides on multi-symbology support this library never has.
- **no equivalent** — a reference state our design cannot express.

Rust names are `snake_case` in `crates/nsh-qr-code`; TS names are the camelCase mirrors in `packages/nsh-qr-code` (`scan`, `stop`, `cameraPermission`, `requestCameraPermission`, `decodeFrame`, `generate`, `toSvg`, `memoryBackend`, `rustBackend`, `useQrScanner`).

## mobile_scanner

### `MobileScanner` widget

| Reference API | Ours | Status | Why |
|---|---|---|---|
| `MobileScanner` (the widget itself) | none — the app renders the camera repo's preview and calls `QrScanner.scan` (`useQrScanner` in TS) | not ours — camera controls belong to the camera repo (nsh-camera) | the widget is a live camera preview; this library decodes and generates |
| `controller` | `QrScanner::new` / `QrScanner::with_backend`; TS `new QrScanner(backend)` | implemented — awaiting first passing CI run | |
| `onDetect` | `scan` — the first `DecodedQr` wins; a second concurrent scan fails `ERR_QR_ALREADY_RUNNING` | implemented — awaiting first passing CI run | |
| `onDetectError` | every failure returns from `scan` as an `ERR_QR_*` code; nothing is swallowed silently | implemented — awaiting first passing CI run | |
| `errorBuilder` | none — the app renders the coded failure itself | not ours — camera controls belong to the camera repo (nsh-camera) | error UI for a dead preview is preview UI |
| `placeholderBuilder` | none | not ours — camera controls belong to the camera repo (nsh-camera) | the placeholder covers camera warm-up |
| `fit` | none | not ours — camera controls belong to the camera repo (nsh-camera) | preview scaling belongs to the preview |
| `overlayBuilder` (`BarcodeOverlay`, `ScanWindowOverlay`) | none — the app draws its own chrome; `QrMatrix.to_svg` renders generated codes | not ours — camera controls belong to the camera repo (nsh-camera) | overlay geometry is preview geometry |
| `scanWindow`, `scanWindowUpdateThreshold`, `updateScanWindow()` | none — `decode_frame` reads the whole frame; a caller crops the luma plane to restrict the region | not ours — camera controls belong to the camera repo (nsh-camera) | the window is laid out in preview coordinates, which exist only with the camera repo's preview |
| `useAppLifecycleState` | none — `stop` resolves a running scan with `ERR_QR_CANCELLED` when the app goes away | not ours — camera controls belong to the camera repo (nsh-camera) | camera lifecycle on background/foreground is the host app's and the camera repo's |
| `tapToFocus` | none | not ours — camera controls belong to the camera repo (nsh-camera) | focus is camera hardware |

### `MobileScannerController`

| Reference API | Ours | Status | Why |
|---|---|---|---|
| `start()` | `QrScanner.scan` with `ScanOptions { timeout }` — one scan at a time; permission is requested through the backend on demand | implemented — awaiting first passing CI run | |
| `stop()` / `pause()` | `stop() -> bool` — resolves the running scan with `ERR_QR_CANCELLED` and returns `true`; a stop with nothing running returns `false`. Not scanning is "paused": frames are pulled, never pushed | implemented — awaiting first passing CI run | |
| `dispose()` | no method — drop the `QrScanner` (Rust `Drop` releases it); `stop` first to cancel a scan still running | implemented — awaiting first passing CI run | |
| `autoStart` | none — a new `QrScanner` starts nothing; scanning begins only at `scan` | implemented — awaiting first passing CI run | the guard is `ERR_QR_ALREADY_RUNNING` on a second `scan` |
| `torchEnabled`, `autoTorch`, `toggleTorch()`, `TorchState` | none | not ours — camera controls belong to the camera repo (nsh-camera) | the flashlight is camera hardware |
| `initialZoom`, `autoZoom`, `setZoomScale()`, `resetZoomScale()`, `zoomScale`, `setFocusPoint()` | none | not ours — camera controls belong to the camera repo (nsh-camera) | zoom and focus are camera hardware |
| `facing`, `switchCamera()` (`ToggleDirection`, `ToggleLensType`, `SelectCamera`), `getSupportedLenses()`, `getBestCloseRangeScanningLens()`, `CameraFacing`, `CameraLensType` | none — which camera feeds the frames is the camera repo's choice; ours decodes any luma frame | not ours — camera controls belong to the camera repo (nsh-camera) | camera selection |
| `buildCameraView()` | none — the preview view is the camera repo's; the app embeds it, not us | not ours — camera controls belong to the camera repo (nsh-camera) | it renders the camera preview |
| `analyzeImage(path)` | `decode_frame` — a still image decodes once the caller turns it into a luma `Frame` (the TS native module's `decodeImage` does that conversion) | implemented — awaiting first passing CI run | ours takes bytes, never file paths |
| `barcodes` stream and `BarcodeCapture` (`barcodes`, `image`, `raw`, `size`) | `scan` yields one `DecodedQr`; `decode_frame` returns every code in one frame as `Vec<DecodedQr>` | implemented — awaiting first passing CI run | no broadcast stream, and the input frame does not come back — it belongs to the `FrameSource` |
| `formats` (`List<BarcodeFormat>`) | none — there is no format list to configure | out of scope: QR only | rqrr decodes QR; the other symbologies are out |
| `detectionTimeoutMs` | `ScanOptions { timeout }` — ends the scan with `ERR_QR_TIMEOUT` | implemented — awaiting first passing CI run | theirs throttles pushes; ours deadlines a pull |
| `cameraResolution`, `lensType`, `detectionSpeed`, `returnImage`, `invertImage` | none — the camera repo owns the pipeline behind `FrameSource`; `decode_frame` takes the luma plane as given (pre-invert white-on-black codes there) and returns only `DecodedQr` values | not ours — camera controls belong to the camera repo (nsh-camera) | these knobs tune the frame producer, which this repo does not have |
| `MobileScannerState` (incl. `hasCameraPermission`) | no state object — permission reads go through `Backend::camera_permission`; "running" is whether a `scan` is in flight | implemented — awaiting first passing CI run | ours has no camera state to mirror — the camera owns it |

### `Barcode` and its objects

| Reference API | Ours | Status | Why |
|---|---|---|---|
| `Barcode` | `DecodedQr { text, bytes, ec_level, version, corners }` | implemented — awaiting first passing CI run | |
| `corners`, `scaleCorners()` | `DecodedQr.corners` — four frame-relative points; the app scales them to view coordinates | implemented — awaiting first passing CI run | |
| `size` | none as a field — derive the box from `corners` | implemented — awaiting first passing CI run | `QrMatrix.size` is the generated module count, not a decode result |
| `rawValue`, `displayValue` | `DecodedQr.text` — lossy UTF-8 | implemented — awaiting first passing CI run | one value, not two: nothing is parsed out of the payload |
| `rawBytes`, `rawDecodedBytes` (`BarcodeBytes`) | `DecodedQr.bytes` — payload verbatim, so non-UTF-8 survives | implemented — awaiting first passing CI run | |
| `format` / `BarcodeFormat` (all values) | none — a decode that succeeds is QR, so `DecodedQr` carries no format field | out of scope: QR only | `qrCode` is the one value this library keeps |
| `type` / `BarcodeType`, and the structured payloads `calendarEvent`, `contactInfo`, `driverLicense`, `email`, `geoPoint`, `phone`, `sms`, `url`, `wifi` | none — `text` and `bytes` come back unparsed; URL, WiFi and contact schemas are app logic | out of scope: QR only | payload classification rides on multi-symbology support |

### `MobileScannerErrorCode` (all eight) and the exception types

| Reference API | Ours | Status | Why |
|---|---|---|---|
| `controllerAlreadyInitialized` | `ERR_QR_ALREADY_RUNNING` | implemented — awaiting first passing CI run | |
| `controllerInitializing` | `ERR_QR_ALREADY_RUNNING` — a second `scan` while one runs | implemented — awaiting first passing CI run | |
| `permissionDenied` | `ERR_QR_PERMISSION_DENIED` | implemented — awaiting first passing CI run | |
| `unsupported` | `ERR_QR_UNSUPPORTED` — also what a `scan` answers while `frame_source()` returns `Ok(None)` | implemented — awaiting first passing CI run | |
| `genericError` | `ERR_QR_BACKEND` | implemented — awaiting first passing CI run | |
| `controllerDisposed` | none — Rust ownership makes use-after-drop unrepresentable; the TS wrapper dies with the scanner | no equivalent | the state cannot occur |
| `controllerUninitialized` | none — `QrScanner::new` is ready immediately | no equivalent | there is no uninitialized state |
| `controllerNotAttached` | none — there is no widget to attach to | no equivalent | the state cannot occur |
| `MobileScannerException`, `MobileScannerErrorDetails`, `MobileScannerBarcodeException` | one `QrError` per failure with a stable code and message (`Display` prints the code); a found-but-unreadable code is `ERR_QR_DECODE_FAILED` | implemented — awaiting first passing CI run | |

### `MobileScannerAuthorizationState`

| Reference API | Ours | Status | Why |
|---|---|---|---|
| `MobileScannerAuthorizationState` (`undetermined`, `authorized`, `denied`) | `PermissionState { Undetermined, Granted, Denied }` from `Backend::camera_permission` and `Backend::request_camera_permission`; TS `cameraPermission()` / `requestCameraPermission()` | implemented — awaiting first passing CI run | |

## react-native-vision-camera

### Scanner

| Reference API | Ours | Status | Why |
|---|---|---|---|
| `useBarcodeScanner({ barcodeFormats })`, `createBarcodeScanner()` | TS `useQrScanner` hook and `new QrScanner(backend)` (`rustBackend`, `memoryBackend`); Rust `QrScanner::new` / `QrScanner::with_backend` | implemented — awaiting first passing CI run | |
| `BarcodeScannerOptions.barcodeFormats`, `TargetBarcodeFormat` (`'all-formats'`) | none — no format list exists | out of scope: QR only | QR is the only symbology |
| `BarcodeScanner.scanCodes(frame)` | `decode_frame` / TS `decodeFrame` | implemented — awaiting first passing CI run | a plain call, not a worklet |
| `BarcodeScanner.scanCodesAsync(frame)` | the same `decode_frame` / `decodeFrame`, awaited | implemented — awaiting first passing CI run | every call resolves exactly once |
| `BarcodeScanner.scanCodesInImageAsync(image)` | `decode_frame` on a luma frame built from the image (TS native `decodeImage`) | implemented — awaiting first passing CI run | |

### Output and view

| Reference API | Ours | Status | Why |
|---|---|---|---|
| `useBarcodeScannerOutput(...)`, `createBarcodeScannerOutput(...)` (`onBarcodeScanned`, `onError`) | `QrScanner.scan` pulling `Backend::frame_source` — `FrameSource::next_frame` / `cancel` is the seam the camera repo implements; the first `DecodedQr` wins, failures arrive as `ERR_QR_*` | implemented — awaiting first passing CI run | |
| `BarcodeScannerOutputOptions.outputResolution` (`'preview'`, `'full'`) | none | not ours — camera controls belong to the camera repo (nsh-camera) | buffer size is the frame producer's choice |
| `<CodeScanner />` (`isActive`, `style`) | none — the app composes its own view and scans with `useQrScanner` | not ours — camera controls belong to the camera repo (nsh-camera) | it is a camera preview component |

### Types

| Reference API | Ours | Status | Why |
|---|---|---|---|
| `BarcodeFormat` union, `TargetBarcodeFormat` | none — `DecodedQr` has no format field | out of scope: QR only | `'qr-code'` is the one member kept |
| `BarcodeValueType` | none — payload classification is app logic over `text` / `bytes` | out of scope: QR only | |
| `Point { x, y }` | `DecodedQr.corners` — four points, no standalone type | implemented — awaiting first passing CI run | |
| `Rect { left, top, right, bottom }` (`boundingBox`) | `corners` — four corners instead of an axis-aligned rect | implemented — awaiting first passing CI run | corners survive perspective |

### Permissions

| Reference API | Ours | Status | Why |
|---|---|---|---|
| `PermissionStatus` (`'not-determined'`, `'authorized'`, `'denied'`, `'restricted'`) | `PermissionState { Undetermined, Granted, Denied }` | implemented — awaiting first passing CI run | `'restricted'` collapses into `Denied` |
| `VisionCamera.cameraPermissionStatus`, `VisionCamera.requestCameraPermission(): Promise<boolean>` | `Backend::camera_permission` / `Backend::request_camera_permission`, bridged on Apple, Android, Windows, Linux and web | implemented — awaiting first passing CI run | |
| `useCameraPermission()` (`status`, `requestPermission`, `hasPermission`, `canRequestPermission`) | TS `QrScanner.cameraPermission()` / `requestCameraPermission()`; `Granted` answers `hasPermission`, `Undetermined` answers `canRequestPermission` | implemented — awaiting first passing CI run | |
| `useMicrophonePermission()` | none | no equivalent | a QR library has no microphone |

## Our error codes against both references

The ten `ERR_QR_*` codes, and the nearest thing each reference has:

| Code | Nearest in mobile_scanner | Nearest in vision-camera |
|---|---|---|
| `ERR_QR_DECODE_FAILED` | `MobileScannerBarcodeException` | `Error` in `onError` |
| `ERR_QR_PAYLOAD_TOO_LARGE` | none | none |
| `ERR_QR_INVALID_ARGUMENT` | none — Dart asserts | none — throws `TypeError` |
| `ERR_QR_PERMISSION_DENIED` | `MobileScannerErrorCode.permissionDenied` | `'denied'` permission status |
| `ERR_QR_UNSUPPORTED` | `MobileScannerErrorCode.unsupported` | none |
| `ERR_QR_ALREADY_RUNNING` | `controllerAlreadyInitialized`, `controllerInitializing` | none |
| `ERR_QR_CANCELLED` | none — `stop()` is silent | none |
| `ERR_QR_TIMEOUT` | none — `detectionTimeoutMs` throttles, it does not deadline | none |
| `ERR_QR_FRAME_SOURCE_CLOSED` | none | none |
| `ERR_QR_BACKEND` | `MobileScannerErrorCode.genericError` | `Error` in `onError` |

## Ours beyond both references

Neither reference generates codes, and both own their cameras. Everything below is implemented — awaiting first passing CI run.

| Ours | What it is |
|---|---|
| `generate(data, &GenerateOptions) -> QrMatrix` / TS `generate` | builds the module matrix; empty data is `ERR_QR_INVALID_ARGUMENT`, oversize data is `ERR_QR_PAYLOAD_TOO_LARGE` |
| `GenerateOptions { ec_level, quiet_zone }` | `ec_level: None` picks by payload length: High up to 255 bytes, Medium up to 1071, Low above |
| `EccLevel { Low, Medium, Quartile, High }` | the error-correction choice for generation |
| `QrMatrix { size, modules, ec_level, version }`, `to_svg()` / TS `toSvg` | the SVG keeps the quiet zone |
| `FrameSource` (`next_frame`, `cancel`) | the seam nsh-camera implements; each `next_frame` resolves exactly once, and a closed source keeps answering `ERR_QR_FRAME_SOURCE_CLOSED` |
| `FakeBackend`, `FakeFrameSource` / TS `memoryBackend`, `fakeFrameSource` | headless tests with no camera |
| Platform bridges (Apple, Android, Windows, Linux, web) | permission bridges now; `frame_source()` answers `Ok(None)` until the camera repo registers a source, and a `scan` then fails `ERR_QR_UNSUPPORTED` |
