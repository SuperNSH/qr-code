//! `decode_frame`: frames without codes, payloads destroyed past ECC, several codes, rotation, raw bytes.

mod common;

use common::{qr_frame, two_code_frame};
use nsh_qr_code::{ErrorKind, Frame, GenerateOptions, decode_frame, generate};

#[test]
fn decode_frame_without_codes_is_empty() {
    for shade in [255u8, 128, 0] {
        let frame = Frame::new(64, 64, vec![shade; 64 * 64]).expect("the frame matches its size");
        let decoded = decode_frame(&frame).expect("a frame without codes is not an error");
        assert!(decoded.is_empty(), "a plain {shade} frame holds no code");
    }
}

#[test]
fn decode_corrupt_code_reports_decode_failed() {
    let scale = 4;
    let quiet_zone = 4;
    let options = GenerateOptions::default(); // one byte picks High and a version-1 grid
    let matrix = generate(b"z", &options).expect("the payload fits");
    let mut frame = qr_frame(b"z", &options, scale);

    // Flip every module between the finder patterns: the grid is still found, its payload is gone.
    for y in 0..matrix.size {
        for x in 8..matrix.size - 8 {
            for dy in 0..scale {
                for dx in 0..scale {
                    let column = (quiet_zone + x) * scale + dx;
                    let row = (quiet_zone + y) * scale + dy;
                    let at = row * frame.width + column;
                    frame.luma[at] = 255 - frame.luma[at];
                }
            }
        }
    }

    let error = decode_frame(&frame).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::DecodeFailed);
    assert_eq!(error.code(), "ERR_QR_DECODE_FAILED");
}

#[test]
fn decode_finds_multiple_codes() {
    let options = GenerateOptions::default();
    let first = generate(b"first code", &options).expect("the payload fits");
    let second = generate(b"second code", &options).expect("the payload fits");
    let frame = two_code_frame(&first, &second, 4);

    let decoded = decode_frame(&frame).expect("both codes decode");
    let mut texts: Vec<&str> = decoded.iter().map(|code| code.text.as_str()).collect();
    texts.sort();
    assert_eq!(texts, ["first code", "second code"]);
}

#[test]
fn decode_rotated_still_decodes() {
    let frame = qr_frame(b"rotated still decodes", &GenerateOptions::default(), 4);
    let rotated = rotate_quarter_turn(&frame);
    let decoded = decode_frame(&rotated).expect("rotation does not hide a code");
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].text, "rotated still decodes");
}

#[test]
fn decode_non_utf8_keeps_bytes() {
    let payload = [0xFFu8, 0xFE, 0x00, 0x41, 0xC3, 0x28];
    let frame = qr_frame(&payload, &GenerateOptions::default(), 4);
    let decoded = decode_frame(&frame).expect("the code decodes");
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].bytes, payload);
    // The text is lossy where the bytes are not UTF-8, and the raw bytes survive alongside it.
    assert!(decoded[0].text.contains('\u{FFFD}'));
    assert_eq!(decoded[0].text, String::from_utf8_lossy(&payload));
}

/// The frame turned a quarter turn clockwise: every row becomes a column.
fn rotate_quarter_turn(frame: &Frame) -> Frame {
    let (width, height) = (frame.width, frame.height);
    let mut luma = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            let (column, row) = (height - 1 - y, x);
            luma[row * height + column] = frame.luma[y * width + x];
        }
    }
    Frame::new(height, width, luma).expect("rotation keeps the sample count")
}
