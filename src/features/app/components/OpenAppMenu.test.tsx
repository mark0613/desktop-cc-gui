// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { OpenAppMenu } from "./OpenAppMenu";

vi.mock("@tauri-apps/plugin-opener", () => ({
  revealItemInDir: vi.fn(),
}));

vi.mock("../../../services/tauri", () => ({
  openWorkspaceIn: vi.fn(),
}));

vi.mock("../../../services/toasts", () => ({
  pushErrorToast: vi.fn(),
}));

vi.mock("../../../services/clientStorage", () => ({
  writeClientStoreValue: vi.fn(),
}));

describe("OpenAppMenu", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("shows a tooltip for the selected app trigger on hover", async () => {
    render(
      <OpenAppMenu
        path="/tmp/demo"
        openTargets={[
          {
            id: "vscode",
            label: "VS Code",
            kind: "app",
            appName: "Visual Studio Code",
            command: null,
            args: [],
          },
        ]}
        selectedOpenAppId="vscode"
        onSelectOpenAppId={vi.fn()}
        iconOnly
      />,
    );

    await act(async () => {
      fireEvent.mouseEnter(screen.getByRole("button", { name: "Open in VS Code" }));
      await vi.advanceTimersByTimeAsync(250);
    });

    expect(screen.getByRole("tooltip").textContent).toContain("Open in VS Code");
  });
});
