//! Reading QR codes out of frames, through `rqrr`.

use rqrr::DeQRError;

use crate::{
    error::{ErrorKind, QrError},
    frame::Frame,
    options::{DecodedQr, EccLevel},
};

/// The QR codes in one frame.
///
/// No codes found is an empty vec. Codes found whose grids all fail to decode is
/// `ERR_QR_DECODE_FAILED` (or `ERR_QR_PAYLOAD_TOO_LARGE` / `ERR_QR_BACKEND` for the grid that
/// says so). When some decode and some do not, the decodable ones come back.
pub fn decode_frame(frame: &Frame) -> Result<Vec<DecodedQr>, QrError> {
    // Frame's fields are public, so a hand-built frame can lie about its shape; refuse it
    // rather than index out of bounds.
    match frame.width.checked_mul(frame.height) {
        Some(area) if area > 0 && frame.luma.len() == area => {}
        _ => return Err(QrError::new(ErrorKind::InvalidArgument)),
    }
    let mut image =
        rqrr::PreparedImage::prepare_from_greyscale(frame.width, frame.height, |x, y| {
            frame.luma[y * frame.width + x]
        });
    let grids = image.detect_grids();
    let mut found = Vec::new();
    let mut failure: Option<QrError> = None;
    for grid in grids {
        let mut bytes = Vec::new();
        match grid.decode_to(&mut bytes) {
            Ok(meta) => match EccLevel::from_rqrr(meta.ecc_level) {
                Some(ec_level) => found.push(DecodedQr {
                    text: String::from_utf8_lossy(&bytes).into_owned(),
                    bytes,
                    ec_level,
                    version: meta.version.0 as u8,
                    corners: [
                        corner(grid.bounds[0]),
                        corner(grid.bounds[1]),
                        corner(grid.bounds[2]),
                        corner(grid.bounds[3]),
                    ],
                }),
                None => remember(&mut failure, || QrError::new(ErrorKind::DecodeFailed)),
            },
            // The first failure decides the code the whole call reports.
            Err(error) => remember(&mut failure, || grid_error(error)),
        }
    }
    if found.is_empty() {
        failure.map_or_else(|| Ok(found), Err)
    } else {
        Ok(found)
    }
}

/// Keeps the first failure of the frame; later ones change nothing.
fn remember(failure: &mut Option<QrError>, make: impl FnOnce() -> QrError) {
    if failure.is_none() {
        *failure = Some(make());
    }
}

/// What a grid that did not decode reports. The bytes go to a `Vec`, so `IoError` cannot
/// happen in practice; it maps to `ERR_QR_BACKEND` for completeness.
fn grid_error(error: DeQRError) -> QrError {
    match error {
        DeQRError::DataOverflow => QrError::new(ErrorKind::PayloadTooLarge),
        DeQRError::IoError => QrError::new(ErrorKind::Backend),
        _ => QrError::new(ErrorKind::DecodeFailed),
    }
}

/// A corner as frame pixels. rqrr maps skewed corners, so a code partly outside the frame
/// reads negative there; those clamp to the frame's edge.
fn corner(point: rqrr::Point) -> (usize, usize) {
    (point.x.max(0) as usize, point.y.max(0) as usize)
}

#[cfg(test)]
mod tests {
    use std::future::Future;

    use super::*;
    use crate::{
        fake::FakeFrameSource,
        frame::{Frame, FrameSource},
        generate::generate,
        options::{EccLevel, GenerateOptions},
    };

    fn run<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
    }

    /// A code drawn into a frame, white around it.
    fn frame_of(matrix: &crate::options::QrMatrix, border: usize) -> Frame {
        let size = matrix.size + 2 * border;
        let mut luma = vec![255u8; size * size];
        for y in 0..matrix.size {
            for x in 0..matrix.size {
                let dark = matrix.modules[y * matrix.size + x];
                luma[(y + border) * size + x + border] = u8::from(!dark) * 255;
            }
        }
        Frame::new(size, size, luma).expect("one luma per pixel")
    }

    #[test]
    fn a_generated_code_round_trips_through_a_frame_source() {
        let matrix = generate(b"Hello NSH", &GenerateOptions::default()).expect("encodes");
        let source = FakeFrameSource::new(vec![frame_of(&matrix, 4)]);
        run(async {
            let frame = source.next_frame().await.expect("one frame");
            let found = decode_frame(&frame).expect("decodes");
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].text, "Hello NSH");
            assert_eq!(found[0].bytes, b"Hello NSH");
            assert_eq!(found[0].ec_level, EccLevel::High, "9 bytes pick High");
            assert_eq!(found[0].version, matrix.version);
            let [top_left, _, bottom_right, _] = found[0].corners;
            assert!(top_left.0 <= bottom_right.0 && top_left.1 <= bottom_right.1);
            assert_eq!(source.next_frame_calls(), 1);
        });
    }

    #[test]
    fn a_frame_without_codes_decodes_to_nothing() {
        let blank = Frame::new(8, 8, vec![128; 64]).expect("8*8 pixels");
        assert!(decode_frame(&blank).expect("no codes is fine").is_empty());
    }

    #[test]
    fn a_frame_that_lies_about_its_shape_is_refused_not_panicked() {
        // The fields are public, so a caller can build one Frame::new would refuse.
        let short = Frame {
            width: 4,
            height: 4,
            luma: vec![0; 5],
        };
        assert_eq!(
            decode_frame(&short).unwrap_err().kind(),
            ErrorKind::InvalidArgument
        );
        let zero_area = Frame {
            width: 0,
            height: 4,
            luma: Vec::new(),
        };
        assert_eq!(
            decode_frame(&zero_area).unwrap_err().kind(),
            ErrorKind::InvalidArgument
        );
    }

    #[test]
    fn a_code_past_correction_reads_as_decode_failed() {
        // Every data module inverted, the patterns kept: the grid is found, the payload is not.
        let code =
            qrcode::QrCode::with_error_correction_level(b"corrupt me", qrcode::EcLevel::H).unwrap();
        let size = code.width();
        let mut luma = vec![255u8; size * size];
        for y in 0..size {
            for x in 0..size {
                let dark = code[(x, y)] == qrcode::Color::Dark;
                let module_dark = if code.is_functional(x, y) {
                    dark
                } else {
                    !dark
                };
                luma[y * size + x] = u8::from(!module_dark) * 255;
            }
        }
        let frame = Frame::new(size, size, luma).expect("one luma per pixel");
        let error = decode_frame(&frame).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::DecodeFailed);
    }

    #[test]
    fn non_utf8_payloads_keep_their_bytes() {
        let payload = [0xff, 0xfe, 0x00, 0x80];
        let matrix = generate(&payload, &GenerateOptions::default()).expect("encodes");
        let source = FakeFrameSource::new(vec![frame_of(&matrix, 4)]);
        run(async {
            let frame = source.next_frame().await.expect("one frame");
            let found = decode_frame(&frame).expect("decodes");
            assert_eq!(found[0].bytes, payload);
            assert_eq!(found[0].text, String::from_utf8_lossy(&payload));
        });
    }
}
