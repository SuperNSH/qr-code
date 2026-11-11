import { expect, test } from "nsh/test";
import {
  QrScanner,
  fakeFrameSource,
  memoryBackend,
  qrError,
  type FrameSource,
  type PermissionState,
  type QrBackend,
} from "../src";
import { codeFrame } from "./helpers";

/** A source whose frame never arrives: a scan parks on it until stopped or timed out. */
function stuckSource(): FrameSource {
  return {
    nextFrame: () => new Promise(() => {}),
    cancel: () => {},
  };
}

test("scan_returns_first_code", async () => {
  const scanner = new QrScanner({
    backend: memoryBackend({
      frameSource: fakeFrameSource([await codeFrame("first"), await codeFrame("second")]),
    }),
  });
  const code = await scanner.scan();
  expect(code.text).toBe("first");
});

test("second_call_is_already_running", async () => {
  const scanner = new QrScanner({ backend: memoryBackend({ frameSource: stuckSource() }) });
  const first = scanner.scan();
  await expect(scanner.scan()).rejects.toMatchObject({ code: "ERR_QR_ALREADY_RUNNING" });
  expect(await scanner.stop()).toBe(true);
  await expect(first).rejects.toMatchObject({ code: "ERR_QR_CANCELLED" });
});

test("stop_resolves_cancelled", async () => {
  const scanner = new QrScanner({ backend: memoryBackend({ frameSource: stuckSource() }) });
  const first = scanner.scan();
  expect(await scanner.stop()).toBe(true);
  await expect(first).rejects.toMatchObject({ code: "ERR_QR_CANCELLED" });
  // The stopped scan's slot is free again, and the second stop has nothing to stop.
  expect(await scanner.stop()).toBe(false);
});

test("stop_without_scan_returns_false", async () => {
  const scanner = new QrScanner({ backend: memoryBackend({ frameSource: fakeFrameSource([]) }) });
  expect(await scanner.stop()).toBe(false);
});

test("timeout_resolves_timeout", async () => {
  const scanner = new QrScanner({ backend: memoryBackend({ frameSource: stuckSource() }) });
  await expect(scanner.scan({ timeoutMs: 5 })).rejects.toMatchObject({ code: "ERR_QR_TIMEOUT" });
  // The timeout freed the gate: another scan runs, and stop ends it as cancelled.
  const next = scanner.scan();
  await expect(scanner.scan()).rejects.toMatchObject({ code: "ERR_QR_ALREADY_RUNNING" });
  expect(await scanner.stop()).toBe(true);
  await expect(next).rejects.toMatchObject({ code: "ERR_QR_CANCELLED" });
});

test("closed_source_ends_scan", async () => {
  const scanner = new QrScanner({ backend: memoryBackend({ frameSource: fakeFrameSource([]) }) });
  await expect(scanner.scan()).rejects.toMatchObject({ code: "ERR_QR_FRAME_SOURCE_CLOSED" });
});

test("memory_backend_permission_round_trip", async () => {
  const states: PermissionState[] = ["undetermined", "granted", "denied"];
  for (const cameraPermission of states) {
    const backend = memoryBackend({ cameraPermission });
    expect(backend.permissionReads()).toBe(0);
    const scanner = new QrScanner({ backend });
    expect(await scanner.cameraPermission()).toBe(cameraPermission);
    expect(await scanner.requestCameraPermission()).toBe(cameraPermission);
    expect(backend.permissionReads()).toBe(1);
    expect(backend.permissionRequests()).toBe(1);
  }

  // A backend failure keeps its code on the way out.
  const failing: QrBackend = {
    cameraPermission: async () => {
      throw qrError("ERR_QR_PERMISSION_DENIED");
    },
    requestCameraPermission: async () => "denied",
    frameSource: async () => null,
  };
  const scanner = new QrScanner({ backend: failing });
  await expect(scanner.cameraPermission()).rejects.toMatchObject({
    code: "ERR_QR_PERMISSION_DENIED",
  });
});
