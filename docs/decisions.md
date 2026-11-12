# Decisions

Each entry says what was chosen and why. `docs/goals.md` lists the acceptance criteria and the tests behind them.

## Open

None.

## Decided

### Library choice: rqrr 0.11 to decode, qrcode 0.14 to generate

rqrr 0.11 is pure Rust and takes luma through `PreparedImage::prepare_from_greyscale`, so the `img` feature (and the `image` crate) stays off. qrcode 0.14 generates with the `std` and `svg` features only; its defaults pull `image`, which this library does not need.

### NSH version: 0.5.0

`nsh-core = "0.5.0"`, and CI installs the matching CLI (`NSH_VERSION: v0.5.0`). The pin moved from 0.4.0 on 2026-11-12 with no source change.

### Platforms in scope: Apple, Android, Windows, Linux, web

Every platform gets its camera-permission bridge now; live frame sources arrive with the camera repo, through the `FrameSource` trait this repo defines.

## Reference: where vision-camera's code scanner lives

react-native-vision-camera's code scanner moved into its `react-native-vision-camera-barcode-scanner` package inside the same monorepo, cloned read-only at `.refs/react-native-vision-camera/packages/react-native-vision-camera-barcode-scanner`; `mrousavy/react-native-data-scanner` is the one-shot Apple alternative. That package path is the canonical location.
