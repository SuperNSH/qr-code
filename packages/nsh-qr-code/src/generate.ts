import { qrError } from "./errors";
import type { EccLevel, GenerateOptions } from "./options";
import { rustGenerate } from "./rust";

/** A generated code: `modules` is row-major, `modules[y * size + x]`. */
export interface QrMatrix {
  size: number;
  modules: boolean[];
  ecLevel: EccLevel;
  version: number;
  /** The code as an SVG: dark modules on white, with the quiet-zone border. */
  toSvg(): string;
}

/** The most bytes a version-40 code carries, per error-correction level. */
const CAPACITY: Record<EccLevel, number> = {
  low: 2953,
  medium: 2331,
  quartile: 1663,
  high: 1273,
};

// Smaller payloads get the strongest correction: the code stays small anyway.
function levelFor(byteLength: number): EccLevel {
  if (byteLength <= 255) return "high";
  if (byteLength <= 1071) return "medium";
  return "low";
}

/**
 * Generates the QR code for `data`. Without `ecLevel` the level is picked by payload length. Empty
 * data rejects with `ERR_QR_INVALID_ARGUMENT`, and a payload no code carries with
 * `ERR_QR_PAYLOAD_TOO_LARGE`.
 */
export async function generate(data: Uint8Array | string, options: GenerateOptions = {}): Promise<QrMatrix> {
  const bytes = typeof data === "string" ? new TextEncoder().encode(data) : data;
  const quietZone = options.quietZone ?? 4;
  // The crate's quiet zone is a u32; anything past that is not an argument it can take.
  if (bytes.length === 0 || !Number.isSafeInteger(quietZone) || quietZone < 0 || quietZone > 4294967295) {
    throw qrError("ERR_QR_INVALID_ARGUMENT");
  }
  const ecLevel = options.ecLevel ?? levelFor(bytes.length);
  if (bytes.length > CAPACITY[ecLevel]) {
    throw qrError("ERR_QR_PAYLOAD_TOO_LARGE");
  }
  const matrix = await rustGenerate(bytes, { ecLevel, quietZone });
  return {
    size: matrix.size,
    modules: matrix.modules,
    ecLevel: matrix.ecLevel,
    version: matrix.version,
    // The crate's own SVG of the same matrix, so both sides draw the same picture.
    toSvg: () => matrix.svg,
  };
}
