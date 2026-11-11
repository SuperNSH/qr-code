import { expect, test } from "nsh/test";
import { decodeFrame, generate, type EccLevel } from "../src";
import { frameOf, rejectionOf } from "./helpers";

test("generate_rejects_empty_data", async () => {
  expect((await rejectionOf(() => generate(new Uint8Array(0)))).code).toBe("ERR_QR_INVALID_ARGUMENT");
  expect((await rejectionOf(() => generate(""))).code).toBe("ERR_QR_INVALID_ARGUMENT");
});

test("generate_rejects_payload_too_large", async () => {
  // 2953 bytes is the most any QR code carries (version 40 at Low); one more fits nowhere.
  expect((await rejectionOf(() => generate("x".repeat(2954)))).code).toBe("ERR_QR_PAYLOAD_TOO_LARGE");
  // High tops out at 1273 bytes on the same version-40 grid.
  expect((await rejectionOf(() => generate("x".repeat(1274), { ecLevel: "high" }))).code).toBe(
    "ERR_QR_PAYLOAD_TOO_LARGE",
  );
});

test("generate_picks_ec_by_payload_length", async () => {
  const cases: [number, EccLevel][] = [
    [1, "high"],
    [255, "high"],
    [256, "medium"],
    [1071, "medium"],
    [1072, "low"],
  ];
  for (const [length, ecLevel] of cases) {
    const matrix = await generate("x".repeat(length));
    expect(matrix.ecLevel).toBe(ecLevel);
    expect(matrix.modules.length).toBe(matrix.size * matrix.size);
    expect(matrix.version).toBeGreaterThanOrEqual(1);
  }
});

test("round_trip_generate_then_decode", async () => {
  const levels: EccLevel[] = ["low", "medium", "quartile", "high"];
  for (const ecLevel of levels) {
    const matrix = await generate("round trip", { ecLevel });
    expect(matrix.ecLevel).toBe(ecLevel);
    const decoded = await decodeFrame(frameOf(matrix, 4));
    expect(decoded.length).toBe(1);
    expect(decoded[0].text).toBe("round trip");
    expect([...decoded[0].bytes]).toEqual([..."round trip"].map((ch) => ch.charCodeAt(0)));
    expect(decoded[0].ecLevel).toBe(ecLevel);
    expect(decoded[0].version).toBe(matrix.version);
  }
});

test("to_svg_matches_matrix", async () => {
  // Without a quiet zone the drawing is exactly the module grid.
  const bare = await generate("svg", { quietZone: 0 });
  const bareSvg = bare.toSvg();
  expect(bareSvg.includes("<svg")).toBe(true);
  expect(bareSvg.includes("</svg>")).toBe(true);
  expect(bareSvg.includes(`viewBox="0 0 ${bare.size} ${bare.size}"`)).toBe(true);

  // The default adds four white modules on every side, and one path rectangle per dark module.
  const matrix = await generate("svg");
  const side = matrix.size + 8;
  const svg = matrix.toSvg();
  expect(svg.includes(`width="${side}"`)).toBe(true);
  expect(svg.includes(`height="${side}"`)).toBe(true);
  expect(svg.includes(`viewBox="0 0 ${side} ${side}"`)).toBe(true);
  // The crate's renderer draws each dark module as one `M<x> <y>h1v1H<x>` subpath, as in Rust.
  const dark = matrix.modules.filter(Boolean).length;
  expect(svg.split("h1v1H").length - 1).toBe(dark);
});
