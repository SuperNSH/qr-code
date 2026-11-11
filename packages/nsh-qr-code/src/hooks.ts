import { cleanup, effect, signal } from "nsh";
import type { DecodedQr } from "./decode";
import { asQrError, type QrError } from "./errors";
import type { ScanOptions } from "./options";
import { QrScanner } from "./scanner";

/** Where the hook's scan stands. */
export type ScanState =
  | { status: "idle" }
  | { status: "scanning" }
  | { status: "failed"; error: QrError };

export interface UseQrScannerOptions {
  /** Called once for each QR code found; QR codes are all this hook scans. */
  onCodeScanned: (code: DecodedQr) => void;
  /** The scanner to run. Defaults to a new one on the Rust crate. */
  scanner?: QrScanner;
}

/** A reactive scan: start and stop it, read where it stands. */
export interface QrScannerHandle {
  state(): ScanState;
  start(options?: ScanOptions): void;
  stop(): void;
}

/**
 * Scans for QR codes while started, reporting each through `onCodeScanned`. A binding on `state()`
 * runs again only when the scan's standing changes. The scan ends when the component goes.
 */
export function useQrScanner(options: UseQrScannerOptions): QrScannerHandle {
  const scanner = options.scanner ?? new QrScanner();
  const state = signal<ScanState>({ status: "idle" });
  const wanted = signal(false);
  let scanOptions: ScanOptions = {};
  let gone = false;
  let attempt = 0;

  let shown = "idle";
  const show = (next: ScanState) => {
    // One standing, one signal write: a rescan cycle must not rerun the binding for nothing.
    const key = next.status === "failed" ? `failed ${next.error.code}` : next.status;
    if (key === shown) return;
    shown = key;
    state.set(next);
  };

  const run = () => {
    // A stop-then-start lets a cancelled scan's rejection land after the next one began; only the
    // current attempt's completion may touch anything, or it would idle out the running scan.
    const current = ++attempt;
    show({ status: "scanning" });
    scanner.scan(scanOptions).then(
      (code) => {
        // The handle can outlive its component, and a code then has no owner to land in.
        if (gone || current !== attempt) return;
        options.onCodeScanned(code);
        if (wanted()) run();
      },
      (error: unknown) => {
        if (gone || current !== attempt) return;
        wanted.set(false);
        const coded = asQrError(error);
        show(coded.code === "ERR_QR_CANCELLED" ? { status: "idle" } : { status: "failed", error: coded });
      },
    );
  };

  effect(() => {
    // Reads `wanted` only: a flip starts the loop, and `run` reruns it per code found.
    if (wanted()) run();
  });
  cleanup(() => {
    gone = true;
    attempt += 1;
    wanted.set(false);
    void scanner.stop();
  });
  return {
    state: () => state(),
    start: (next?: ScanOptions) => {
      scanOptions = next ?? {};
      wanted.set(true);
    },
    stop: () => {
      wanted.set(false);
      void scanner.stop();
    },
  };
}
