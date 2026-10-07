import react from "@vitejs/plugin-react";
import { expect, it } from "vitest";
import configText from "../../src-tauri/tauri.conf.json?raw";

it("permits only the locked React development preamble without weakening production scripts", async () => {
  const config = JSON.parse(configText) as {
    app: { security: { csp: string; devCsp: string } };
  };
  const { csp, devCsp } = config.app.security;
  const scriptPolicy = (policy: string) =>
    policy
      .split(";")
      .map((part) => part.trim())
      .find((part) => part.startsWith("script-src "));
  const preamble = react.preambleCode.replace("__BASE__", "/");
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(preamble),
  );
  const hash = btoa(String.fromCharCode(...new Uint8Array(digest)));

  expect(scriptPolicy(csp)).toBe("script-src 'self'");
  expect(scriptPolicy(devCsp)).toBe(`script-src 'self' 'sha256-${hash}'`);
  expect(csp).not.toContain("unsafe-eval");
  expect(devCsp).not.toContain("unsafe-eval");
});
