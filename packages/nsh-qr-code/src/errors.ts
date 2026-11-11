/** The codes a QR rejection carries. Apps match on `code`. */
export type QrErrorCode =
  | "ERR_QR_DECODE_FAILED"
  | "ERR_QR_PAYLOAD_TOO_LARGE"
  | "ERR_QR_INVALID_ARGUMENT"
  | "ERR_QR_PERMISSION_DENIED"
  | "ERR_QR_UNSUPPORTED"
  | "ERR_QR_ALREADY_RUNNING"
  | "ERR_QR_CANCELLED"
  | "ERR_QR_TIMEOUT"
  | "ERR_QR_FRAME_SOURCE_CLOSED"
  | "ERR_QR_BACKEND";

/** Same wording as the Rust crate's messages. */
const MESSAGES: Record<QrErrorCode, string> = {
  ERR_QR_DECODE_FAILED: "A QR code was found but its payload could not be read. Try a clearer frame.",
  ERR_QR_PAYLOAD_TOO_LARGE: "The payload does not fit a QR code. Shorten it or lower the error correction.",
  ERR_QR_INVALID_ARGUMENT: "An argument is empty or not allowed, such as a frame whose size does not match its data.",
  ERR_QR_PERMISSION_DENIED: "Camera permission was denied. Grant it in the system settings.",
  ERR_QR_UNSUPPORTED: "No camera is registered on this platform yet; live frames come from the camera repo.",
  ERR_QR_ALREADY_RUNNING: "A scan is already running. Stop it before starting another.",
  ERR_QR_CANCELLED: "The scan was cancelled.",
  ERR_QR_TIMEOUT: "The scan timed out. Try again.",
  ERR_QR_FRAME_SOURCE_CLOSED: "The frame source stopped feeding frames.",
  ERR_QR_BACKEND: "The platform or frame source failed unexpectedly.",
};

const KNOWN = new Set<string>(Object.keys(MESSAGES));

/** A rejection of the QR API. `code` is the stable name; the message says what to do. */
export class QrError extends Error {
  readonly code: QrErrorCode;

  constructor(code: QrErrorCode) {
    super(MESSAGES[code]);
    this.name = "QrError";
    this.code = code;
  }
}

/** The error for a known code. */
export function qrError(code: QrErrorCode): QrError {
  return new QrError(code);
}

/**
 * Turns whatever the native side or a frame source threw into a coded `QrError`. A host's rejection
 * carries `ERR_NSH_HOST_ERROR`, and its message starts with the code, as `ERR_QR_TIMEOUT: text`.
 * Anything unknown is a backend failure.
 */
export function asQrError(error: unknown): QrError {
  if (error instanceof QrError) return error;
  const raw = (error ?? {}) as { code?: unknown; message?: unknown };
  const code = typeof raw.code === "string" ? raw.code : "";
  const message = typeof raw.message === "string" ? raw.message : "";
  if (code === "ERR_NSH_HOST_ERROR") {
    const named = /^(ERR_QR_[A-Z_]+): /.exec(message)?.[1];
    return named !== undefined && KNOWN.has(named)
      ? qrError(named as QrErrorCode)
      : qrError("ERR_QR_BACKEND");
  }
  return KNOWN.has(code) ? qrError(code as QrErrorCode) : qrError("ERR_QR_BACKEND");
}
