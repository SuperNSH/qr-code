import type { QrBackend } from "./backend";
import { decodeFrame, type DecodedQr } from "./decode";
import { asQrError, qrError, type QrError } from "./errors";
import type { Frame, FrameSource } from "./frames";
import type { PermissionState, ScanOptions } from "./options";
import { rustBackend } from "./rust";

type Outcome = { ok: true; value: DecodedQr } | { ok: false; error: QrError };

/**
 * A scan over a frame source, as the Rust crate names it. It runs on the Rust crate unless it is
 * given another backend. One scan runs at a time, and each call resolves exactly once.
 */
export class QrScanner {
  private readonly backend: QrBackend;
  private busy = false;
  private waiting: ((outcome: Outcome) => void) | null = null;

  constructor(options: { backend?: QrBackend } = {}) {
    this.backend = options.backend ?? rustBackend();
  }

  cameraPermission(): Promise<PermissionState> {
    return this.backend.cameraPermission();
  }

  requestCameraPermission(): Promise<PermissionState> {
    return this.backend.requestCameraPermission();
  }

  /**
   * Scans until a frame holds a readable QR code and resolves it. Empty and undecodable frames are
   * ordinary frames and keep the scan going; a source that stopped feeding frames ends it with
   * `ERR_QR_FRAME_SOURCE_CLOSED`. A second call while one runs rejects with
   * `ERR_QR_ALREADY_RUNNING`, a timeout with `ERR_QR_TIMEOUT`, and a zero or invalid `timeoutMs`
   * with `ERR_QR_INVALID_ARGUMENT`.
   */
  scan(options: ScanOptions = {}): Promise<DecodedQr> {
    if (options.timeoutMs !== undefined && !(options.timeoutMs > 0)) {
      return Promise.reject(qrError("ERR_QR_INVALID_ARGUMENT"));
    }
    if (this.busy) {
      return Promise.reject(qrError("ERR_QR_ALREADY_RUNNING"));
    }
    this.busy = true;
    return new Promise<DecodedQr>((resolve, reject) => {
      let settled = false;
      let source: FrameSource | null = null;
      let timer: ReturnType<typeof setTimeout> | null = null;
      const stopSource = () => {
        const open = source;
        source = null;
        if (open === null) return;
        // The scan is over: the source must stop feeding frames nobody reads.
        try {
          open.cancel();
        } catch {
          // A cancel that throws is the source's own failure to stop; the scan is settled either way.
        }
      };
      const settle = (outcome: Outcome) => {
        if (settled) return;
        settled = true;
        if (timer !== null) clearTimeout(timer);
        if (this.waiting === settle) {
          this.waiting = null;
          this.busy = false;
        }
        stopSource();
        if (outcome.ok) resolve(outcome.value);
        else reject(outcome.error);
      };
      this.waiting = settle;
      if (options.timeoutMs !== undefined) {
        timer = setTimeout(() => {
          settle({ ok: false, error: qrError("ERR_QR_TIMEOUT") });
        }, options.timeoutMs);
      }
      const hunt = async (): Promise<void> => {
        let open: FrameSource | null;
        try {
          open = await this.backend.frameSource();
        } catch (error: unknown) {
          settle({ ok: false, error: asQrError(error) });
          return;
        }
        if (open === null) {
          settle({ ok: false, error: qrError("ERR_QR_UNSUPPORTED") });
          return;
        }
        source = open;
        if (settled) {
          // A timeout or a stop won before the source opened; it still has to be told to stop.
          stopSource();
          return;
        }
        for (;;) {
          let frame: Frame;
          try {
            frame = await open.nextFrame();
          } catch (error: unknown) {
            settle({ ok: false, error: asQrError(error) });
            return;
          }
          if (settled) return;
          let found: DecodedQr[];
          try {
            found = await decodeFrame(frame);
          } catch (error: unknown) {
            const coded = asQrError(error);
            // Blur, rotation and low light are ordinary frames: a code that will not read is tried again on the next frame.
            if (coded.code === "ERR_QR_DECODE_FAILED") continue;
            settle({ ok: false, error: coded });
            return;
          }
          if (found.length > 0) {
            settle({ ok: true, value: found[0] });
            return;
          }
        }
      };
      void hunt();
    });
  }

  /**
   * Cancels the running scan. The pending `scan` rejects with `ERR_QR_CANCELLED`. Resolves `true`
   * when a scan was running; a second stop resolves `false`.
   */
  async stop(): Promise<boolean> {
    const busy = this.busy;
    const waiting = this.waiting;
    // The gate opens at once: the cancelled scan's loop is dead and cannot settle again.
    this.waiting = null;
    this.busy = false;
    waiting?.({ ok: false, error: qrError("ERR_QR_CANCELLED") });
    return busy;
  }
}
