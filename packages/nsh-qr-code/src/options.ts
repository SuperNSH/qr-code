/** Where camera permission stands, as the platforms name it. */
export type PermissionState = "undetermined" | "granted" | "denied";

/** QR error correction: qrcode's L/M/Q/H, rqrr's levels 0/1/2/3. */
export type EccLevel = "low" | "medium" | "quartile" | "high";

/** The options of one scan. */
export interface ScanOptions {
  /** Milliseconds until the scan gives up with `ERR_QR_TIMEOUT`. Undefined waits for a code. */
  timeoutMs?: number;
}

/** The options of one generated code. */
export interface GenerateOptions {
  /** The error-correction level. Default is picked by payload length. */
  ecLevel?: EccLevel;
  /** The white border, in modules. Defaults to 4. */
  quietZone?: number;
}
