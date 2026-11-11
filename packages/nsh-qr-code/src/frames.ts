/** One video frame: grayscale luma, row-major, 0 black and 255 white. */
export interface Frame {
  width: number;
  height: number;
  luma: Uint8Array;
}

/**
 * The seam the camera repo (SuperNSH/nsh-camera) implements for live capture. Each `nextFrame`
 * call resolves exactly once; a cancelled or ended source keeps rejecting with
 * `ERR_QR_FRAME_SOURCE_CLOSED`.
 */
export interface FrameSource {
  nextFrame(): Promise<Frame>;
  cancel(): void;
}
