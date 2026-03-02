/** @vitest-environment jsdom */
import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import type { ComposerEditorSettings } from "../../../types";
import { Composer } from "./Composer";

vi.mock("../../../services/dragDrop", () => ({
  subscribeWindowDragDrop: vi.fn(() => () => {}),
}));

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `tauri://${path}`,
  invoke: vi.fn(async () => null),
}));

vi.mock("../../engine/components/EngineSelector", () => ({
  EngineSelector: () => null,
}));

vi.mock("../../opencode/components/OpenCodeControlPanel", () => ({
  OpenCodeControlPanel: ({ visible }: { visible: boolean }) =>
    visible ? <div data-testid="opencode-control-panel" /> : null,
}));

vi.mock("./ChatInputBox/ChatInputBoxAdapter", () => ({
  ChatInputBoxAdapter: ({
    text,
    onTextChange,
    onSend,
  }: {
    text: string;
    onTextChange: (next: string, cursor: number | null) => void;
    onSend: () => void;
  }) => (
    <textarea
      value={text}
      onChange={(event) =>
        onTextChange(event.currentTarget.value, event.currentTarget.value.length)
      }
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          onSend();
        }
      }}
    />
  ),
}));

type HarnessProps = {
  initialText?: string;
  selectedEngine?: "claude" | "codex" | "opencode";
  commands?: { name: string; path: string; content: string }[];
  onSend?: (text: string, images: string[], options?: { selectedMemoryIds?: string[] }) => void;
};

function ComposerHarness({
  initialText = "",
  selectedEngine = "claude",
  commands = [],
  onSend = () => {},
}: HarnessProps) {
  const editorSettings: ComposerEditorSettings = {
    preset: "smart",
    expandFenceOnSpace: true,
    expandFenceOnEnter: false,
    fenceLanguageTags: true,
    fenceWrapSelection: true,
    autoWrapPasteMultiline: true,
    autoWrapPasteCodeLike: true,
    continueListOnShiftEnter: true,
  };

  return (
    <Composer
      onSend={onSend}
      onQueue={() => {}}
      onStop={() => {}}
      canStop={false}
      isProcessing={false}
      steerEnabled={false}
      collaborationModes={[]}
      collaborationModesEnabled={true}
      selectedCollaborationModeId={null}
      onSelectCollaborationMode={() => {}}
      selectedEngine={selectedEngine}
      models={[]}
      selectedModelId={null}
      onSelectModel={() => {}}
      reasoningOptions={[]}
      selectedEffort={null}
      onSelectEffort={() => {}}
      reasoningSupported={false}
      accessMode="current"
      onSelectAccessMode={() => {}}
      skills={[]}
      prompts={[]}
      commands={commands}
      files={[]}
      draftText={initialText}
      onDraftChange={() => {}}
      dictationEnabled={false}
      editorSettings={editorSettings}
      activeWorkspaceId="ws-1"
      activeThreadId="thread-1"
    />
  );
}

type RenderedHarness = {
  container: HTMLDivElement;
  unmount: () => void;
};

function renderComposerHarness(props: HarnessProps): RenderedHarness {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);

  act(() => {
    root.render(<ComposerHarness {...props} />);
  });

  return {
    container,
    unmount: () => {
      act(() => {
        root.unmount();
      });
      container.remove();
    },
  };
}

function getTextarea(container: HTMLElement) {
  const textarea = container.querySelector("textarea");
  if (!textarea) {
    throw new Error("Textarea not found");
  }
  return textarea;
}

describe("Composer editor helpers", () => {
  it("sends selected opencode direct command chip on Enter without chat text", async () => {
    const onSend = vi.fn();
    const harness = renderComposerHarness({
      initialText: "/export",
      selectedEngine: "opencode",
      commands: [{ name: "export", path: "", content: "" }],
      onSend,
    });
    const textarea = getTextarea(harness.container);

    await act(async () => {
      textarea.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
      );
    });

    expect(onSend).toHaveBeenCalledWith("/export", []);
    harness.unmount();
  });

});
