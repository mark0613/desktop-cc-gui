// @vitest-environment jsdom
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { WorkspaceInfo } from "../../../types";
import { MainHeader } from "./MainHeader";

vi.mock("@tauri-apps/plugin-opener", () => ({
  revealItemInDir: vi.fn(),
}));

const workspace: WorkspaceInfo = {
  id: "workspace-1",
  name: "desktop-cc-gui",
  path: "/tmp/desktop-cc-gui",
  connected: true,
  settings: {
    sidebarCollapsed: false,
  },
};

const groupedWorkspaces = [
  {
    id: "group-1",
    name: "Main",
    workspaces: [workspace],
  },
];

function renderHeader() {
  return render(
    <MainHeader
      workspace={workspace}
      openTargets={[]}
      openAppIconById={{}}
      selectedOpenAppId=""
      onSelectOpenAppId={() => {}}
      branchName="feature/branch-menu"
      branches={[
        { name: "feature/branch-menu", lastCommit: Date.now() },
        { name: "main", lastCommit: Date.now() - 1_000 },
      ]}
      onCheckoutBranch={() => {}}
      onCreateBranch={() => {}}
      groupedWorkspaces={groupedWorkspaces}
      activeWorkspaceId={workspace.id}
      onSelectWorkspace={() => {}}
    />,
  );
}

describe("MainHeader branch reveal behavior", () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("keeps branch controls visible while the pointer stays within the title row", () => {
    vi.useFakeTimers();
    const { container } = renderHeader();
    const titleLine = container.querySelector(".workspace-title-line");
    const projectMenu = container.querySelector(".workspace-project-menu");

    expect(titleLine).toBeTruthy();
    expect(projectMenu).toBeTruthy();
    if (!titleLine || !projectMenu) {
      throw new Error("Expected workspace title line with project menu");
    }

    fireEvent.mouseEnter(titleLine);
    fireEvent.mouseEnter(projectMenu);
    expect(titleLine.className).toContain("is-project-detail-visible");

    fireEvent.mouseLeave(projectMenu);
    act(() => {
      vi.advanceTimersByTime(250);
    });

    expect(titleLine.className).toContain("is-project-detail-visible");
  });

  it("waits briefly after leaving the whole title row before hiding branch controls", () => {
    vi.useFakeTimers();
    const { container } = renderHeader();
    const titleLine = container.querySelector(".workspace-title-line");
    const projectMenu = container.querySelector(".workspace-project-menu");

    expect(titleLine).toBeTruthy();
    expect(projectMenu).toBeTruthy();
    if (!titleLine || !projectMenu) {
      throw new Error("Expected workspace title line with project menu");
    }

    fireEvent.mouseEnter(titleLine);
    fireEvent.mouseEnter(projectMenu);
    expect(titleLine.className).toContain("is-project-detail-visible");

    fireEvent.mouseLeave(titleLine);
    act(() => {
      vi.advanceTimersByTime(250);
    });
    expect(titleLine.className).toContain("is-project-detail-visible");

    act(() => {
      vi.advanceTimersByTime(500);
    });
    expect(titleLine.className).not.toContain("is-project-detail-visible");
  });
});
