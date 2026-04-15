// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { LaunchScriptEntry } from "../../../types";
import { LaunchScriptEntryButton } from "./LaunchScriptEntryButton";

const entry: LaunchScriptEntry = {
  id: "entry-1",
  script: "npm run dev",
  icon: "play",
  label: "Dev",
};

describe("LaunchScriptEntryButton", () => {
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
      <LaunchScriptEntryButton
        entry={entry}
        editorOpen
        draftScript={entry.script}
        draftIcon={entry.icon}
        draftLabel={entry.label ?? ""}
        isSaving={false}
        error={null}
        onRun={vi.fn()}
        onOpenEditor={vi.fn()}
        onCloseEditor={onCloseEditor}
        onDraftChange={vi.fn()}
        onDraftIconChange={vi.fn()}
        onDraftLabelChange={vi.fn()}
        onSave={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    fireEvent.pointerDown(screen.getByPlaceholderText("例如 npm run dev"));

    expect(onCloseEditor).not.toHaveBeenCalled();
  });

  it("shows the launch label in a tooltip on hover", async () => {
    render(
      <LaunchScriptEntryButton
        entry={entry}
        editorOpen={false}
        draftScript={entry.script}
        draftIcon={entry.icon}
        draftLabel={entry.label ?? ""}
        isSaving={false}
        error={null}
        onRun={vi.fn()}
        onOpenEditor={vi.fn()}
        onCloseEditor={vi.fn()}
        onDraftChange={vi.fn()}
        onDraftIconChange={vi.fn()}
        onDraftLabelChange={vi.fn()}
        onSave={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    await act(async () => {
      fireEvent.mouseEnter(screen.getAllByRole("button", { name: "Dev" })[0]);
      await vi.advanceTimersByTimeAsync(250);
    });

    expect(screen.getByRole("tooltip").textContent).toContain("Dev");
  });
});
