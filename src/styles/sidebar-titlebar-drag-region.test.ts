import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const sidebarCss = readFileSync(
  fileURLToPath(new URL("./sidebar.css", import.meta.url)),
  "utf8",
);

function getCssRuleBlock(css: string, selector: string): string {
  const escapedSelector = selector
    .replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
    .replace(/\s+/g, "\\s+");
  const match = css.match(new RegExp(`${escapedSelector}\\s*\\{([^}]*)\\}`));
  return match?.[1] ?? "";
}

describe("sidebar titlebar drag region", () => {
  it("keeps the shell draggable while isolating the sidebar toggle", () => {
    const contentRule = getCssRuleBlock(sidebarCss, ".sidebar-topbar-content");
    const toggleRule = getCssRuleBlock(sidebarCss, ".sidebar-titlebar-toggle");
    const swappedRule = getCssRuleBlock(sidebarCss, ".sidebar-titlebar-toggle.is-layout-swapped");

    expect(contentRule).toContain("justify-content: flex-end;");
    expect(contentRule).toContain("-webkit-app-region: drag;");
    expect(toggleRule).toContain("width: auto;");
    expect(toggleRule).toContain("margin-left: auto;");
    expect(toggleRule).toContain("-webkit-app-region: no-drag;");
    expect(swappedRule).toContain("margin-right: auto;");
  });
});
