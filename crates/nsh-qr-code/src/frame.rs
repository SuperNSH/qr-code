//! A frame of grayscale pixels, and the source the camera repo feeds them through.

use std::{future::Future, pin::Pin};

use crate::error::{ErrorKind, QrError};

/// A future a frame source returns. Each call's future borrows the source.
pub type FrameFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// One camera frame: grayscale luma, row-major, 0 black, 255 white.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// The frame's width in pixels.
    pub width: usize,
    /// The frame's height in pixels.
    pub height: usize,
    /// One luma value per pixel, `width * height` of them.
    pub luma: Vec<u8>,
}

impl Frame {
    /// A frame of `width` times `height` pixels.
    ///
    /// Refuses a `luma` whose length is not the pixel count, and a zero-sized frame, with
    /// `ERR_QR_INVALID_ARGUMENT`.
    pub fn new(width: usize, height: usize, luma: Vec<u8>) -> Result<Self, QrError> {
        let invalid = || QrError::new(ErrorKind::InvalidArgument);
        let area = width.checked_mul(height).ok_or_else(invalid)?;
        if area == 0 || luma.len() != area {
            return Err(invalid());
        }
        Ok(Self {
            width,
            height,
            luma,
        })
    }
}

/// Where frames come from. This is the seam the camera repo (SuperNSH/nsh-camera) implements
/// for live capture.
///
/// Each `next_frame` call resolves exactly once. A cancelled or ended source keeps answering
/// `ERR_QR_FRAME_SOURCE_CLOSED`. Implementations never wait unboundedly.
pub trait FrameSource: Send + Sync {
    /// The next frame, when it arrives.
    fn next_frame(&self) -> FrameFuture<'_, Result<Frame, QrError>>;

    /// Asks the source to stop feeding frames; pending `next_frame` calls answer closed.
    fn cancel(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_needs_one_luma_per_pixel() {
        let frame = Frame::new(2, 3, vec![7; 6]).expect("2*3 pixels");
        assert_eq!((frame.width, frame.height, frame.luma.len()), (2, 3, 6));

        let short = Frame::new(2, 3, vec![0; 5]).unwrap_err();
        assert_eq!(short.kind(), ErrorKind::InvalidArgument);
        let long = Frame::new(2, 3, vec![0; 7]).unwrap_err();
        assert_eq!(long.kind(), ErrorKind::InvalidArgument);
    }

    #[test]
    fn a_zero_area_frame_is_refused() {
        assert_eq!(
            Frame::new(0, 3, Vec::new()).unwrap_err().kind(),
            ErrorKind::InvalidArgument
        );
        assert_eq!(
            Frame::new(2, 0, Vec::new()).unwrap_err().kind(),
            ErrorKind::InvalidArgument
        );
    }

    #[test]
    fn dimensions_that_overflow_are_refused_not_panicked() {
        let error = Frame::new(usize::MAX, usize::MAX, Vec::new()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
    }
}
