# nsh-qr-code

Decode and generate QR codes for [Native Shell](https://nsh.dev) apps, in Rust and in TSX, with stable error codes an app can match on.

Part of the Native Shell (NSH) capability set. Same layout as the other SuperNSH repos: a Rust crate, a TypeScript package, docs in `docs/`, CI on GitHub Actions, and a web status page on GitHub Pages at https://supernsh.github.io/qr-code/.

What it does:

- Decodes every QR code in a frame: bytes, lossy text, error-correction level, version and corners. A still image decodes the same way once it is a frame; the native module's `decodeImage` does that for TSX apps.
- Generates QR codes, picking the error-correction level by payload length when the caller does not (High up to 255 bytes, Medium up to 1071, Low above), and renders an SVG with a quiet zone.
- Scans one code at a time over a `FrameSource`: `stop()` resolves the scan with `ERR_QR_CANCELLED`, `timeout` ends it with `ERR_QR_TIMEOUT`, a second concurrent scan gets `ERR_QR_ALREADY_RUNNING`. Each call resolves exactly once.
- Bridges camera permission on Apple, Android, Windows, Linux and the web.

No code in this repo captures frames. The `FrameSource` trait is defined here as the seam, and the camera repo ([SuperNSH/nsh-camera](https://github.com/SuperNSH/nsh-camera), a different repo) implements it for live capture. Until a backend registers a frame source, a scan answers `ERR_QR_UNSUPPORTED`.

## Install

In the app's directory:

```sh
nsh add nsh-qr-code
```

`nsh add` needs NSH main or the nightly: the 0.6.0 and 0.6.1 releases do not have it. In a Rust app, `cargo add nsh-qr-code` works with any of them.

TSX code imports from `nsh-qr-code`.

## Status

**Implemented, verified on CI.** CI run [44376377286](https://github.com/SuperNSH/qr-code/actions/runs/44376377286) on `master` (NSH 0.6.0) passed every job: format, `rust` on ubuntu, macOS and Windows (clippy `-D warnings` and `cargo test`), `check` for `aarch64-apple-ios`, `aarch64-linux-android` and `wasm32-unknown-unknown`, and `typescript` (`nsh check`, `nsh build` with no warnings, `nsh test`). Each edge-case test was also seen failing: run [44401068264](https://github.com/SuperNSH/qr-code/actions/runs/44401068264) broke the scan gate, the ECC pick and the timeout on purpose and went red in the Rust unit tests and the TS suite. Not verified: no live scan on any device, because the camera repo supplies frames; the tests scan fake frame sources.

## Layout

- `crates/nsh-qr-code/`: the Rust crate (library `nsh_qr_code`).
- `packages/nsh-qr-code/`: the TypeScript package.
- `docs/`: `brief.md` (research and options), `goals.md` (goals, acceptance criteria, errors, edge cases and their tests), `decisions.md`, `parity.md`.
- `site/`: static status page, published by `.github/workflows/pages.yml`.
- `.zcode/workflow.ts`: the phases this area runs through.
- `.refs/`: cloned reference repos, read-only, ignored by git and by the build.

## Checks

Nothing builds on a developer machine. GitHub Actions runs the gates (`.github/workflows/ci.yml`):

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`
- `nsh check`
- `nsh test`

## Licence

MIT. See [LICENSE](LICENSE).

Repository: https://github.com/SuperNSH/qr-code
