import { asQrError, generate, type Frame, type QrError, type QrMatrix } from "../src";

/** How many white modules surround every code a test draws, matching the spec's default quiet zone. */
const quietZone = 4;

/** Renders a matrix at `scale` pixels per module inside a white quiet zone, ready for decodeFrame. */
export function frameOf(matrix: QrMatrix, scale: number): Frame {
  const side = (matrix.size + 2 * quietZone) * scale;
  const luma = new Uint8Array(side * side).fill(255);
  draw(matrix, quietZone, scale, side, luma);
  return { width: side, height: side, luma };
}

/** Two codes side by side in one frame, with a gap, so a decode has to find both. */
export function twoCodeFrame(first: QrMatrix, second: QrMatrix, scale: number): Frame {
  const cells = Math.max(first.size, second.size);
  const gap = quietZone;
  const side = (quietZone + cells + gap + cells + quietZone) * scale;
  const luma = new Uint8Array(side * side).fill(255);
  draw(first, quietZone, scale, side, luma);
  draw(second, quietZone + cells + gap, scale, side, luma);
  return { width: side, height: side, luma };
}

/** The coded error a throwing or rejecting call produced. Fails the test when it settled fine. */
export async function rejectionOf(run: () => unknown): Promise<QrError> {
  try {
    await run();
  } catch (error) {
    return asQrError(error);
  }
  throw new Error("the call was expected to reject");
}

/** A frame carrying one generated code, the common input of the decode and scanner tests. */
export async function codeFrame(text: string): Promise<Frame> {
  return frameOf(await generate(text), 4);
}

/** Writes one matrix's dark modules into `luma` at `offsetX` modules from the left edge. */
function draw(matrix: QrMatrix, offsetX: number, scale: number, side: number, luma: Uint8Array): void {
  for (let y = 0; y < matrix.size; y += 1) {
    for (let x = 0; x < matrix.size; x += 1) {
      if (!matrix.modules[y * matrix.size + x]) continue;
      for (let dy = 0; dy < scale; dy += 1) {
        for (let dx = 0; dx < scale; dx += 1) {
          const column = (offsetX + x) * scale + dx;
          const row = (quietZone + y) * scale + dy;
          luma[row * side + column] = 0;
        }
      }
    }
  }
}
