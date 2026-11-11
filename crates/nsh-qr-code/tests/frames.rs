//! `Frame` owns its size rule: the luma must cover exactly width * height, and the area must not be zero.

use nsh_qr_code::{ErrorKind, Frame};

#[test]
fn frame_rejects_size_mismatch() {
    let frame = Frame::new(3, 2, vec![128; 6]).expect("six samples cover three by two");
    assert_eq!((frame.width, frame.height), (3, 2));
    assert_eq!(frame.luma.len(), 6);

    for luma in [Vec::new(), vec![0; 5], vec![0; 7]] {
        let error = Frame::new(3, 2, luma).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
        assert_eq!(error.code(), "ERR_QR_INVALID_ARGUMENT");
    }
}

#[test]
fn frame_rejects_zero_area() {
    for (width, height) in [(0, 4), (4, 0), (0, 0)] {
        let error = Frame::new(width, height, vec![255; width * height]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
        assert_eq!(error.code(), "ERR_QR_INVALID_ARGUMENT");
    }
}
