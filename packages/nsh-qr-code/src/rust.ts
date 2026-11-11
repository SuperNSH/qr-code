import { NativeQrCode } from "nsh-qr-code/native";
import type { QrBackend } from "./backend";
import type { DecodedQr } from "./decode";
import { asQrError, qrError } from "./errors";
import type { Frame, FrameSource } from "./frames";
import type { EccLevel, PermissionState, ScanOptions } from "./options";

/** What the crate's decoding answers for one found code: corners flattened to x,y pairs. */
interface NativeDecoded {
  text: string;
  bytes: Uint8Array;
  ecLevel: string;
  version: number;
  corners: number[];
}

/** The crate's answer to one generation: the matrix facts, and its own SVG of them. */
export interface NativeQrMatrix {
  size: number;
  modules: boolean[];
  ecLevel: EccLevel;
  version: number;
  svg: string;
}

/**
 * The Rust crate as a backend: permissions live there, and so does its own scan loop. The app or
 * package must list the crate under `"rust"` in `nsh.json`; importing this module without it fails
 * the bundle.
 */
export interface RustQrBackend extends QrBackend {
  /** The crate's own scan over the platform's frame source, bypassing this package's loop. */
  scan(options?: ScanOptions): Promise<DecodedQr>;
  /** Stops the crate's running scan. Resolves whether one was running. */
  stop(): Promise<boolean>;
}

/** The Rust crate (`nsh-qr-code/native`) as a backend. */
export function rustBackend(): RustQrBackend {
  return new RustBackend();
}

/** Decodes one frame through the crate, whatever backend a scan runs on. */
export function rustDecodeImage(frame: Frame): Promise<DecodedQr[]> {
  return settle(async () => {
    const found = (await native().decodeImage(frame.width, frame.height, frame.luma)) as NativeDecoded[];
    return found.map(toDecoded);
  });
}

/** Generates a code through the crate; the error-correction level is already chosen. */
export function rustGenerate(
  data: Uint8Array,
  options: { ecLevel: EccLevel; quietZone: number },
): Promise<NativeQrMatrix> {
  return settle(async () => {
    const nativeOptions: { ecLevel: string; quietZone: number } = {
      ecLevel: options.ecLevel,
      quietZone: options.quietZone,
    };
    const svg = (await native().generate(data, nativeOptions)) as string;
    return matrixOf(svg, options);
  });
}

let shared: NativeQrCode | undefined;

// Made on first use, so importing the package starts nothing on the native side.
function native(): NativeQrCode {
  if (shared === undefined) shared = new NativeQrCode();
  return shared;
}

async function settle<T>(operation: () => Promise<T>): Promise<T> {
  try {
    return await operation();
  } catch (error) {
    throw asQrError(error);
  }
}

function toDecoded(found: NativeDecoded): DecodedQr {
  // The crate flattens the four corners to x,y pairs, top-left first.
  return {
    text: found.text,
    bytes: found.bytes,
    ecLevel: found.ecLevel as EccLevel,
    version: found.version,
    corners: [
      { x: found.corners[0], y: found.corners[1] },
      { x: found.corners[2], y: found.corners[3] },
      { x: found.corners[4], y: found.corners[5] },
      { x: found.corners[6], y: found.corners[7] },
    ],
  };
}

/**
 * Reads the crate's SVG back into a matrix: qrcode 0.14 draws each dark rect as one
 * `M{left} {top}h{width}v{height}H{left}V{top}` subpath in canvas units, and the crate renders
 * one unit per module, offset by the quiet zone, so the string carries the whole grid.
 */
function matrixOf(svg: string, options: { ecLevel: EccLevel; quietZone: number }): NativeQrMatrix {
  const total = Number(/viewBox="0 0 (\d+) /.exec(svg)?.[1]);
  const size = total - options.quietZone * 2;
  if (!Number.isInteger(size) || size < 21) {
    // Only a crate that stopped drawing its own grammar lands here.
    throw qrError("ERR_QR_BACKEND");
  }
  const modules = new Array<boolean>(size * size).fill(false);
  const subpath = /M(\d+) (\d+)h(\d+)v(\d+)H\d+V\d+/g;
  for (let found = subpath.exec(svg); found !== null; found = subpath.exec(svg)) {
    const left = Number(found[1]) - options.quietZone;
    const top = Number(found[2]) - options.quietZone;
    for (let dy = 0; dy < Number(found[4]); dy += 1) {
      for (let dx = 0; dx < Number(found[3]); dx += 1) {
        modules[(top + dy) * size + left + dx] = true;
      }
    }
  }
  // A normal QR side is 4 * version + 17 modules across.
  return { size, modules, ecLevel: options.ecLevel, version: (size - 17) / 4, svg };
}

class RustBackend implements RustQrBackend {
  cameraPermission(): Promise<PermissionState> {
    return settle(async () => (await native().cameraPermission()) as PermissionState);
  }

  requestCameraPermission(): Promise<PermissionState> {
    return settle(async () => (await native().requestCameraPermission()) as PermissionState);
  }

  frameSource(): Promise<FrameSource | null> {
    // No platform registers a camera yet; live frame sources arrive with the camera repo.
    return Promise.resolve(null);
  }

  scan(options: ScanOptions = {}): Promise<DecodedQr> {
    const nativeOptions: { timeoutMs?: number } = {};
    if (options.timeoutMs !== undefined) nativeOptions.timeoutMs = options.timeoutMs;
    return settle(async () => toDecoded((await native().scan(nativeOptions)) as NativeDecoded));
  }

  stop(): Promise<boolean> {
    return settle(() => native().stop());
  }
}
