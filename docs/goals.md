# Goals

What this library must do, what CI must pass for it to count, the errors it reports, and the edge cases a test must cover.

## The library

- **Decode QR codes.** `decode_frame(&Frame)` finds every QR code in a luma frame and returns its bytes, lossy text, error-correction level, version and corners. A still image decodes the same way once the caller turns it into a frame; the native module's `decodeImage` does that for TSX apps.
- **Generate QR codes.** `generate(data, &GenerateOptions)` returns a module matrix and an SVG rendering, with the error-correction level picked by payload length when the caller does not choose one.
- **Define the `FrameSource` seam.** This repo owns the trait the camera repo implements:

  ```rust
  trait FrameSource: Send + Sync {
      fn next_frame(&self) -> FrameFuture<'_, Result<Frame, QrError>>;
      fn cancel(&self);
  }
  ```

  Each `next_frame` call resolves exactly once. A cancelled or ended source keeps answering `FrameSourceClosed`, and an implementation never waits unboundedly. [SuperNSH/nsh-camera](https://github.com/SuperNSH/nsh-camera) is a different repo: this repo does not include it and must not edit it. Until a backend registers a frame source, `frame_source()` answers `Ok(None)` and a scan reports `Unsupported`.

## Acceptance criteria

Every gate runs on GitHub CI, never on a developer machine. A criterion counts only when its gate passes there.

| Criterion | Gate |
|---|---|
| The workspace is formatted | `cargo fmt --all -- --check` |
| No Clippy warnings over all targets, on every platform target CI compiles | `cargo clippy --all-targets -- -D warnings` |
| The Rust suite passes on every runner, through `FakeBackend` and `FakeFrameSource`, no camera needed | `cargo test --all-targets` |
| The TypeScript package type-checks | `nsh check` |
| The TypeScript suite passes headless, through the memory backend | `nsh test` |

The Rust and TypeScript suites must also prove the process rules: a scan resolves exactly once — a code, a coded failure, or a cancel; it never hangs past its timeout; nothing panics on user data. The remaining named tests back the criteria directly: the error table below (`every_kind_has_its_code_and_message`, `codes_match_the_rust_table`), host rejections (`as_error_parses_host_prefix`, `unknown_becomes_backend`), generate/decode round trips and the SVG border (`round_trip_each_ec_level`, `svg_contains_quiet_zone_and_matrix`, `round_trip_generate_then_decode`, `to_svg_matches_matrix`), and the reactive read (`use_qr_scanner_reports_codes`).

## Errors

Every failure is one of these kinds, with a stable code an app can match on. `Display` prints the code.

| Kind | Code | Message |
|---|---|---|
| DecodeFailed | `ERR_QR_DECODE_FAILED` | A QR code was found but its payload could not be read. Try a clearer frame. |
| PayloadTooLarge | `ERR_QR_PAYLOAD_TOO_LARGE` | The payload does not fit a QR code. Shorten it or lower the error correction. |
| InvalidArgument | `ERR_QR_INVALID_ARGUMENT` | An argument is empty or not allowed, such as a frame whose size does not match its data. |
| PermissionDenied | `ERR_QR_PERMISSION_DENIED` | Camera permission was denied. Grant it in the system settings. |
| Unsupported | `ERR_QR_UNSUPPORTED` | No camera is registered on this platform yet; live frames come from the camera repo. |
| AlreadyRunning | `ERR_QR_ALREADY_RUNNING` | A scan is already running. Stop it before starting another. |
| Cancelled | `ERR_QR_CANCELLED` | The scan was cancelled. |
| Timeout | `ERR_QR_TIMEOUT` | The scan timed out. Try again. |
| FrameSourceClosed | `ERR_QR_FRAME_SOURCE_CLOSED` | The frame source stopped feeding frames. |
| Backend | `ERR_QR_BACKEND` | The platform or frame source failed unexpectedly. |

## Edge cases

Every case maps to a named test, listed verbatim. Rust tests live in `crates/nsh-qr-code/tests/`, TypeScript tests in `packages/nsh-qr-code/tests/`.

| Case | Behavior | Test |
|---|---|---|
| Empty generate payload | `generate` refuses empty data with `InvalidArgument` | `generate.rs` `generate_rejects_empty_data`; `generate.test.ts` `generate_rejects_empty_data` |
| Payload over capacity | Data longer than the largest QR code holds is `PayloadTooLarge` | `generate.rs` `generate_rejects_payload_too_large`; `generate.test.ts` `generate_rejects_payload_too_large` |
| Frame size mismatch | `Frame::new` refuses `luma.len() != width * height` with `InvalidArgument` | `frames.rs` `frame_rejects_size_mismatch` |
| Zero-size frame | `Frame::new` refuses a zero width or height with `InvalidArgument` | `frames.rs` `frame_rejects_zero_area` |
| No code in the frame | `decode_frame` returns an empty vector, and a scan keeps going | `decode.rs` `decode_frame_without_codes_is_empty`; `decode.test.ts` `decode_frame_without_codes_is_empty` |
| Corrupt code (ECC failure) | Every grid found but none decodes is `DecodeFailed` | `decode.rs` `decode_corrupt_code_reports_decode_failed` |
| Multiple codes in one frame | Every decodable grid is returned; a partially decodable frame keeps the decodable ones | `decode.rs` `decode_finds_multiple_codes`; `decode.test.ts` `decode_finds_multiple_codes` |
| Rotation still decodes | A rotated code is found and its corners come back rotated | `decode.rs` `decode_rotated_still_decodes` |
| Blur and low light reduce decode rate | Decode only sees the frames it is given; the caller controls the rate. Empty and failed frames are ordinary frames, and the scan keeps going | `scanner.rs` `scan_returns_first_code` (blank frames before the code); `scanner.test.ts` `scan_returns_first_code` |
| Non-UTF-8 payload | The bytes are kept verbatim; the text is lossy | `decode.rs` `decode_non_utf8_keeps_bytes` |
| Camera busy | A second concurrent scan gets `AlreadyRunning` | `scanner.rs` `scan_second_call_is_already_running`; `scanner.test.ts` `second_call_is_already_running` |
| Permission denied | `Denied` answers through `PermissionState` on every platform bridge, with one code | `scanner.rs` `permission_states_round_trip`; `scanner.test.ts` `memory_backend_permission_round_trip` |
| Cancel | `stop()` resolves the running scan with `Cancelled` and reports that one was running | `scanner.rs` `stop_resolves_cancelled_and_returns_true`; `scanner.test.ts` `stop_resolves_cancelled` |
| Timeout | `ScanOptions.timeout` ends the scan with `Timeout` | `scanner.rs` `scan_timeout_resolves_timeout`; `scanner.test.ts` `timeout_resolves_timeout` |
| Closed frame source | A cancelled or drained source keeps answering `FrameSourceClosed`, and the scan ends with it | `scanner.rs` `closed_source_ends_scan_with_frame_source_closed`; `scanner.test.ts` `closed_source_ends_scan` |
| Double stop returns false | The first stop returns `true`; a stop with no scan running, including a second stop, returns `false` | `scanner.rs` `stop_without_scan_returns_false`; `scanner.test.ts` `stop_without_scan_returns_false` |
| Scan with no camera, web included | `frame_source()` answering `Ok(None)` makes the scan `Unsupported` | `scanner.rs` `scan_without_camera_is_unsupported` |
| Generation ECC picked by payload length | `ec_level: None` picks High up to 255 bytes, Medium up to 1071, Low above: smaller payloads get the strongest correction, larger ones still fit | `generate.rs` `generate_picks_ec_by_payload_length`; `generate.test.ts` `generate_picks_ec_by_payload_length` |
