import type { FrameSource } from "./frames";
import type { PermissionState } from "./options";

/** What a `QrScanner` runs on: the Rust crate, or a memory backend in tests. */
export interface QrBackend {
  cameraPermission(): Promise<PermissionState>;
  requestCameraPermission(): Promise<PermissionState>;
  /** The live frame source, or null where no camera is registered yet. */
  frameSource(): Promise<FrameSource | null>;
}
