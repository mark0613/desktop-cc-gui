import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const variablesBridgeCss = readFileSync(
  fileURLToPath(new URL("./variables-bridge.css", import.meta.url)),
  "utf8",
);

describe("chat input variables bridge", () => {
  it("defines monochrome codex context accents for shared composers", () => {
    expect(variablesBridgeCss).toContain("--codex-context-accent:");
    expect(variablesBridgeCss).toContain("--codex-context-accent-track:");
    expect(variablesBridgeCss).toContain("var(--text-primary, #e6e7ea) 86%");
    expect(variablesBridgeCss).toContain("var(--text-primary, #333333) 84%");
  });
});
