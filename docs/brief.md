# QR codes: brief

Decode and generate QR codes for NSH apps.

## Options

- rqrr: pure Rust decoding from grayscale frames (decode only)
- A QR generator crate: not yet chosen; verify before adopting
- Frames come from the camera, which NSH core does not provide; see the camera repo
- Reference only: mobile_scanner (Flutter)

## Recommended path

rqrr for decoding in the Rust core. Generation via a separate crate. Scanning waits on the camera repo.

## Edge cases to design for

- Camera busy or permission denied: report it, do not hang
- Blur, rotation and low light reduce decode rate
- Decoding only sees the frames it is given; the caller controls the rate
- Generation: pick the error-correction level by payload length

## Research status

Research done 2026-11-09 by web search. Versions and licences are unverified until checked on the registry before pinning. Full catalogue: the research notes for the SuperNSH library set.
