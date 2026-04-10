// @vitest-environment jsdom
import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ContextBar } from "./ContextBar";

describe("ContextBar live canvas controls visibility", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    window.localStorage.removeItem("ccgui.messages.live.autoFollow");
    window.localStorage.removeItem("ccgui.messages.live.collapseMiddleSteps");
  });

  it("shows output collapse controls in history mode when there are messages", () => {
    const { container } = render(
      <ContextBar
        isLoading={false}
        hasMessages
        showStatusPanelToggle
      />,
    );

    expect(container.querySelector(".context-live-canvas-controls")).toBeTruthy();
    expect(container.querySelector(".context-live-canvas-btn--focus-follow")).toBeNull();
    expect(container.querySelector(".context-live-canvas-btn")).toBeTruthy();
  });

  it("hides output collapse controls when idle and no messages", () => {
    const { container } = render(
      <ContextBar
        isLoading={false}
        hasMessages={false}
        showStatusPanelToggle
      />,
    );

    expect(container.querySelector(".context-live-canvas-controls")).toBeNull();
  });
});
