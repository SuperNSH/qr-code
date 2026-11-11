//! Generation rules: payload boundaries, the automatic ECC pick, round trips through decode, the SVG.

mod common;

use common::frame_of;
use nsh_qr_code::{EccLevel, ErrorKind, GenerateOptions, decode_frame, generate};

#[test]
fn generate_rejects_empty_data() {
    let error = generate(&[], &GenerateOptions::default()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidArgument);
    assert_eq!(error.code(), "ERR_QR_INVALID_ARGUMENT");
}

#[test]
// The options are non-exhaustive, so a test outside the crate sets its fields one by one.
#[allow(clippy::field_reassign_with_default)]
fn generate_rejects_payload_too_large() {
    // 2953 bytes is the most any QR code carries (version 40 at Low); one more fits nowhere.
    let error = generate(&[b'x'; 2954], &GenerateOptions::default()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::PayloadTooLarge);
    assert_eq!(error.code(), "ERR_QR_PAYLOAD_TOO_LARGE");

    // High tops out at 1273 bytes on the same version-40 grid.
    let mut high = GenerateOptions::default();
    high.ec_level = Some(EccLevel::High);
    let error = generate(&[b'x'; 1274], &high).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::PayloadTooLarge);
}

#[test]
fn generate_picks_ec_by_payload_length() {
    let cases = [
        (1usize, EccLevel::High),
        (255, EccLevel::High),
        (256, EccLevel::Medium),
        (1071, EccLevel::Medium),
        (1072, EccLevel::Low),
    ];
    for (length, expected) in cases {
        let data = vec![b'x'; length];
        let matrix =
            generate(&data, &GenerateOptions::default()).expect("every case fits at its own level");
        assert_eq!(matrix.ec_level, expected, "a payload of {length} bytes");
        assert_eq!(matrix.modules.len(), matrix.size * matrix.size);
        assert!(matrix.version >= 1);
    }
}

#[test]
// The options are non-exhaustive, so a test outside the crate sets its fields one by one.
#[allow(clippy::field_reassign_with_default)]
fn round_trip_each_ec_level() {
    for level in [
        EccLevel::Low,
        EccLevel::Medium,
        EccLevel::Quartile,
        EccLevel::High,
    ] {
        let mut options = GenerateOptions::default();
        options.ec_level = Some(level);
        let matrix = generate(b"round trip", &options).expect("the payload fits");
        assert_eq!(matrix.ec_level, level);

        let frame = frame_of(&matrix, 4);
        let decoded = decode_frame(&frame).expect("a generated code decodes");
        assert_eq!(decoded.len(), 1, "one code was drawn");
        let code = &decoded[0];
        assert_eq!(code.text, "round trip");
        assert_eq!(code.bytes, b"round trip");
        assert_eq!(code.ec_level, level);
        assert_eq!(code.version, matrix.version);
        for (x, y) in code.corners {
            assert!(
                x < frame.width && y < frame.height,
                "the corners sit inside the frame"
            );
        }
    }
}

#[test]
// The options are non-exhaustive, so a test outside the crate sets its fields one by one.
#[allow(clippy::field_reassign_with_default)]
fn svg_contains_quiet_zone_and_matrix() {
    // Without a quiet zone the drawing is exactly the module grid.
    let mut plain = GenerateOptions::default();
    plain.quiet_zone = 0;
    let bare = generate(b"svg", &plain).expect("the payload fits");
    let bare_svg = bare.to_svg();
    assert!(bare_svg.contains("<svg"));
    assert!(bare_svg.contains("</svg>"));
    assert!(bare_svg.contains(&format!("viewBox=\"0 0 {side} {side}\"", side = bare.size)));

    // The default adds four white modules on every side, and one unit square per dark module.
    let matrix = generate(b"svg", &GenerateOptions::default()).expect("the payload fits");
    let side = matrix.size + 2 * 4;
    let svg = matrix.to_svg();
    assert!(svg.contains(&format!("width=\"{side}\"")));
    assert!(svg.contains(&format!("height=\"{side}\"")));
    assert!(svg.contains(&format!("viewBox=\"0 0 {side} {side}\"")));
    // qrcode 0.14 draws one path rectangle per dark module, as `M<x> <y>h1v1H<x>`.
    let dark = matrix.modules.iter().filter(|module| **module).count();
    assert_eq!(svg.matches("h1v1H").count(), dark);
}
