import type { QrBackend } from "./backend";
import { qrError } from "./errors";
import type { Frame, FrameSource } from "./frames";
import type { PermissionState } from "./options";

/** The answers a memory backend gives. Each call answers what the config says. */
export interface MemoryQrConfig {
  /** What `cameraPermission` reports. Default "granted". */
  cameraPermission?: PermissionState;
  /** The source `frameSource` hands out. Default none. */
  frameSource?: FrameSource | null;
  /** Called on each `cameraPermission` read, so a test can count them. */
  onPermissionRead?: () => void;
}

/** A backend the test drives: it states the permission and hands out its source. */
export interface MemoryQrBackend extends QrBackend {
  setPermission(value: PermissionState): void;
  setFrameSource(source: FrameSource | null): void;
  permissionReads(): number;
  permissionRequests(): number;
  frameSourceCalls(): number;
}

/** A memory backend, for tests and for apps that run without a camera. */
export function memoryBackend(config: MemoryQrConfig = {}): MemoryQrBackend {
  let permission = config.cameraPermission ?? "granted";
  let source: FrameSource | null = config.frameSource ?? null;
  let reads = 0;
  let requests = 0;
  let sourceCalls = 0;

  return {
    cameraPermission: async () => {
      reads += 1;
      config.onPermissionRead?.();
      return permission;
    },
    requestCameraPermission: async () => {
      requests += 1;
      return permission;
    },
    frameSource: async () => {
      sourceCalls += 1;
      return source;
    },
    setPermission: (value) => {
      permission = value;
    },
    setFrameSource: (next) => {
      source = next;
    },
    permissionReads: () => reads,
    permissionRequests: () => requests,
    frameSourceCalls: () => sourceCalls,
  };
}

/** A frame source a test reads: it counts its calls and reports when it closed. */
export interface FakeFrameSource extends FrameSource {
  nextFrameCalls(): number;
  cancels(): number;
  closed(): boolean;
}

/**
 * Feeds the given frames in order, then keeps rejecting with `ERR_QR_FRAME_SOURCE_CLOSED`.
 * `cancel` closes it at once, queued frames or not.
 */
export function fakeFrameSource(frames: Frame[]): FakeFrameSource {
  const queue = [...frames];
  let calls = 0;
  let cancelCount = 0;
  let closed = false;

  return {
    nextFrame: () => {
      calls += 1;
      const frame = closed ? undefined : queue.shift();
      return frame === undefined
        ? Promise.reject(qrError("ERR_QR_FRAME_SOURCE_CLOSED"))
        : Promise.resolve(frame);
    },
    cancel: () => {
      closed = true;
      cancelCount += 1;
    },
    nextFrameCalls: () => calls,
    cancels: () => cancelCount,
    closed: () => closed,
  };
}
