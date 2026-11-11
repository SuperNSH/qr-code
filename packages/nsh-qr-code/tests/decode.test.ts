import { expect, test } from "nsh/test";
import { decodeFrame, generate, type Frame } from "../src";
import { twoCodeFrame } from "./helpers";

/** A frame of one plain shade: no finder patterns, no codes. */
function blank(shade: number): Frame {
  const luma = new Uint8Array(64 * 64).fill(shade);
  return { width: 64, height: 64, luma };
}

test("decode_frame_without_codes_is_empty", async () => {
  for (const shade of [255, 128, 0]) {
    expect((await decodeFrame(blank(shade))).length).toBe(0);
  }
});

test("decode_finds_multiple_codes", async () => {
  const first = await generate("first code");
  const second = await generate("second code");
  const decoded = await decodeFrame(twoCodeFrame(first, second, 4));
  const texts = decoded.map((code) => code.text).sort();
  expect(texts).toEqual(["first code", "second code"]);
});
