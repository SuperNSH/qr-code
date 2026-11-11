//! Frames the tests build from generated matrices: luma, row-major, 0 black and 255 white.
#![allow(dead_code)] // each test binary links the whole module, not only the helpers it calls

use nsh_qr_code::{Frame, GenerateOptions, QrMatrix, generate};

/// How many white modules surround every code a test draws, matching the spec's default quiet zone.
const QUIET_ZONE: usize = 4;

/// Renders `matrix` at `scale` pixels per module inside a white quiet zone.
pub fn frame_of(matrix: &QrMatrix, scale: usize) -> Frame {
    let modules = matrix.size;
    let side = (modules + 2 * QUIET_ZONE) * scale;
    let mut luma = vec![255u8; side * side];
    draw(matrix, QUIET_ZONE, scale, side, &mut luma);
    Frame::new(side, side, luma).expect("the frame matches its own size")
}

/// Generates `data` and renders it, ready for `decode_frame`.
pub fn qr_frame(data: &[u8], options: &GenerateOptions, scale: usize) -> Frame {
    let matrix = generate(data, options).expect("the test payload fits");
    frame_of(&matrix, scale)
}

/// Two codes side by side in one frame, with a gap, so a decode has to find both.
pub fn two_code_frame(first: &QrMatrix, second: &QrMatrix, scale: usize) -> Frame {
    let cells = first.size.max(second.size);
    let gap = QUIET_ZONE;
    let side = (QUIET_ZONE + cells + gap + cells + QUIET_ZONE) * scale;
    let mut luma = vec![255u8; side * side];
    draw(first, QUIET_ZONE, scale, side, &mut luma);
    draw(second, QUIET_ZONE + cells + gap, scale, side, &mut luma);
    Frame::new(side, side, luma).expect("the frame matches its own size")
}

/// Writes one matrix's dark modules into `luma` at `offset_x` modules from the left edge.
fn draw(matrix: &QrMatrix, offset_x: usize, scale: usize, side: usize, luma: &mut [u8]) {
    for y in 0..matrix.size {
        for x in 0..matrix.size {
            if !matrix.modules[y * matrix.size + x] {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    let column = (offset_x + x) * scale + dx;
                    let row = (QUIET_ZONE + y) * scale + dy;
                    luma[row * side + column] = 0;
                }
            }
        }
    }
}
