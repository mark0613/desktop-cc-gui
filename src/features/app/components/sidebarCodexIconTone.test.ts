import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("sidebar codex icon tone", () => {
  it("keeps sidebar codex icons monochrome instead of hardcoding a brand color", () => {
    const sidebarComponent = readFileSync(
      resolve(process.cwd(), "src/features/app/components/Sidebar.tsx"),
      "utf8",
    );
    const sidebarStyles = readFileSync(
      resolve(process.cwd(), "src/styles/sidebar.css"),
      "utf8",
    );

    expect(sidebarComponent).not.toContain('engine="codex" size={14} style={{ color: "#10a37f" }}');
    expect(sidebarStyles).toContain(".thread-engine-badge.thread-engine-codex");
    expect(sidebarStyles).toContain("color: var(--text-strong);");
  });
});
