// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LaunchScriptButton } from "./LaunchScriptButton";

describe("LaunchScriptButton", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("does not close editor when interacting inside popover", () => {
    const onCloseEditor = vi.fn();
    render(
      <LaunchScriptButton
        launchScript={null}
        editorOpen
        draftScript=""
        isSaving={false}
        error={null}
        onRun={vi.fn()}
        onOpenEditor={vi.fn()}
        onCloseEditor={onCloseEditor}
        onDraftChange={vi.fn()}
        onSave={vi.fn()}
      />,
    );

    fireEvent.pointerDown(screen.getByPlaceholderText("例如 npm run dev"));

    expect(onCloseEditor).not.toHaveBeenCalled();
  });

  it("ignores pointer events whose target is not a Node", () => {
    const onCloseEditor = vi.fn();
    render(
      <LaunchScriptButton
        launchScript={null}
        editorOpen
        draftScript=""
        isSaving={false}
        error={null}
        onRun={vi.fn()}
        onOpenEditor={vi.fn()}
        onCloseEditor={onCloseEditor}
        onDraftChange={vi.fn()}
        onSave={vi.fn()}
      />,
    );

    window.dispatchEvent(new PointerEvent("pointerdown"));

    expect(onCloseEditor).not.toHaveBeenCalled();
  });

  it("shows a tooltip for the run button on hover", async () => {
    render(
      <LaunchScriptButton
        launchScript="npm run dev"
        editorOpen={false}
        draftScript=""
        isSaving={false}
        error={null}
        onRun={vi.fn()}
        onOpenEditor={vi.fn()}
        onCloseEditor={vi.fn()}
        onDraftChange={vi.fn()}
        onSave={vi.fn()}
      />,
    );

    await act(async () => {
      fireEvent.mouseEnter(screen.getByRole("button", { name: "composer.runLaunchScript" }));
      await vi.advanceTimersByTimeAsync(250);
    });

    expect(screen.getByRole("tooltip").textContent).toContain("composer.runLaunchScript");
  });
});
