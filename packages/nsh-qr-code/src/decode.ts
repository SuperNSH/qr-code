import { asQrError, qrError } from "./errors";
import type { Frame } from "./frames";
import type { EccLevel } from "./options";
import { rustDecodeImage } from "./rust";

/** A corner of a found code, in frame pixels. */
export interface QrPoint {
  x: number;
  y: number;
}

/** One QR code a frame held: its payload, and where it sat. */
export interface DecodedQr {
  /** The payload as text, lossy for non-UTF-8 bytes; `bytes` keeps those. */
  text: string;
  bytes: Uint8Array;
  ecLevel: EccLevel;
  version: number;
  /** Top-left, top-right, bottom-right, bottom-left. */
  corners: [QrPoint, QrPoint, QrPoint, QrPoint];
}

/**
 * Finds the QR codes in one frame. A frame with none resolves empty; codes too damaged to read
 * reject with `ERR_QR_DECODE_FAILED`; a frame whose size does not match its data with
 * `ERR_QR_INVALID_ARGUMENT`.
 */
export async function decodeFrame(frame: Frame): Promise<DecodedQr[]> {
  if (frame.width <= 0 || frame.height <= 0 || frame.luma.length !== frame.width * frame.height) {
    throw qrError("ERR_QR_INVALID_ARGUMENT");
  }
  try {
    return await rustDecodeImage(frame);
  } catch (error) {
    throw asQrError(error);
  }
}
