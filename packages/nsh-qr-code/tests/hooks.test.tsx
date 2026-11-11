import { open, test } from "nsh/test";

test("use_qr_scanner_reports_codes", async () => {
  const app = await open("./fixtures/scanner.tsx", { clock: {} });
  await app.press("id:start");
  await app.waitFor("text:first-code");
});
