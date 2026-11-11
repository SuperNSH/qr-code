import { expect, test } from "nsh/test";
import { QrError, asQrError, qrError, type QrErrorCode } from "../src";

/** The Rust crate's table of codes and messages, word for word. */
const CODES: [QrErrorCode, string][] = [
  ["ERR_QR_DECODE_FAILED", "A QR code was found but its payload could not be read. Try a clearer frame."],
  ["ERR_QR_PAYLOAD_TOO_LARGE", "The payload does not fit a QR code. Shorten it or lower the error correction."],
  ["ERR_QR_INVALID_ARGUMENT", "An argument is empty or not allowed, such as a frame whose size does not match its data."],
  ["ERR_QR_PERMISSION_DENIED", "Camera permission was denied. Grant it in the system settings."],
  ["ERR_QR_UNSUPPORTED", "No camera is registered on this platform yet; live frames come from the camera repo."],
  ["ERR_QR_ALREADY_RUNNING", "A scan is already running. Stop it before starting another."],
  ["ERR_QR_CANCELLED", "The scan was cancelled."],
  ["ERR_QR_TIMEOUT", "The scan timed out. Try again."],
  ["ERR_QR_FRAME_SOURCE_CLOSED", "The frame source stopped feeding frames."],
  ["ERR_QR_BACKEND", "The platform or frame source failed unexpectedly."],
];

test("codes_match_the_rust_table", () => {
  for (const [code, message] of CODES) {
    const error = qrError(code);
    expect(error instanceof QrError).toBe(true);
    expect(error instanceof Error).toBe(true);
    expect(error.code).toBe(code);
    expect(error.message).toBe(message);
  }
});

test("as_error_parses_host_prefix", () => {
  expect(asQrError({ code: "ERR_QR_DECODE_FAILED", message: "x" }).code).toBe("ERR_QR_DECODE_FAILED");
  const host = asQrError({ code: "ERR_NSH_HOST_ERROR", message: "ERR_QR_CANCELLED: text" });
  expect(host.code).toBe("ERR_QR_CANCELLED");
  expect(host.message).toBe("The scan was cancelled.");
});

test("unknown_becomes_backend", () => {
  expect(asQrError(new Error("boom")).code).toBe("ERR_QR_BACKEND");
  expect(asQrError({ code: "ERR_SOMETHING_ELSE", message: "x" }).code).toBe("ERR_QR_BACKEND");
  expect(asQrError({ code: "ERR_NSH_HOST_ERROR", message: "no code inside" }).code).toBe("ERR_QR_BACKEND");
  expect(asQrError(null).code).toBe("ERR_QR_BACKEND");
});
