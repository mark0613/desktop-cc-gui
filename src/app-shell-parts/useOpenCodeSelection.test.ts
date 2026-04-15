// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { OpenCodeAgentOption } from "../types";
import {
  migratePendingOpenCodeThreadSelection,
  normalizeOpenCodeAgentsResponse,
  normalizeOpenCodeSelectionValue,
  resolveOpenCodeThreadSelection,
  useOpenCodeSelection,
} from "./useOpenCodeSelection";

describe("normalizeOpenCodeAgentsResponse", () => {
  it("normalizes wrapped responses, trims ids, filters blanks, and sorts by id", () => {
    const normalized = normalizeOpenCodeAgentsResponse({
      result: [
        { id: " zebra ", description: 123, is_primary: true },
        { id: "alpha", description: "first", isPrimary: false },
        { id: "   " },
      ],
    });

    expect(normalized).toEqual<OpenCodeAgentOption[]>([
      { id: "alpha", description: "first", isPrimary: false },
      { id: "zebra", description: "123", isPrimary: true },
    ]);
  });
});

describe("resolveOpenCodeThreadSelection", () => {
  it("prefers thread-specific selection over workspace default", () => {
    expect(
      resolveOpenCodeThreadSelection({
        activeWorkspaceId: "ws-1",
        threadId: "thread-1",
        byThreadId: { "thread-1": "agent-b" },
        defaultsByWorkspace: { "ws-1": "agent-a" },
      }),
    ).toBe("agent-b");
  });

  it("falls back to workspace default when thread has no override", () => {
    expect(
      resolveOpenCodeThreadSelection({
        activeWorkspaceId: "ws-1",
        threadId: "thread-2",
        byThreadId: {},
        defaultsByWorkspace: { "ws-1": "agent-a" },
      }),
    ).toBe("agent-a");
  });

  it("returns null without an active workspace", () => {
    expect(
      resolveOpenCodeThreadSelection({
        activeWorkspaceId: null,
        threadId: "thread-2",
        byThreadId: { "thread-2": "agent-a" },
        defaultsByWorkspace: { "ws-1": "agent-b" },
      }),
    ).toBeNull();
  });
});

describe("normalizeOpenCodeSelectionValue", () => {
  it("trims non-empty values and collapses blank values to null", () => {
    expect(normalizeOpenCodeSelectionValue("  high  ")).toBe("high");
    expect(normalizeOpenCodeSelectionValue("   ")).toBeNull();
    expect(normalizeOpenCodeSelectionValue(null)).toBeNull();
  });
});

describe("migratePendingOpenCodeThreadSelection", () => {
  it("copies a pending thread selection to the finalized opencode thread", () => {
    expect(
      migratePendingOpenCodeThreadSelection({
        selectionsByThreadId: { "opencode-pending-1": "agent-a" },
        previousThreadId: "opencode-pending-1",
        activeThreadId: "opencode:session-1",
      }),
    ).toEqual({
      "opencode-pending-1": "agent-a",
      "opencode:session-1": "agent-a",
    });
  });

  it("does not overwrite an existing finalized thread selection", () => {
    expect(
      migratePendingOpenCodeThreadSelection({
        selectionsByThreadId: {
          "opencode-pending-1": "agent-a",
          "opencode:session-1": "agent-b",
        },
        previousThreadId: "opencode-pending-1",
        activeThreadId: "opencode:session-1",
      }),
    ).toEqual({
      "opencode-pending-1": "agent-a",
      "opencode:session-1": "agent-b",
    });
  });
});

describe("useOpenCodeSelection", () => {
  it("supports syncing the active thread after the hook initializes", () => {
    const { result } = renderHook(() =>
      useOpenCodeSelection({
        activeEngine: "claude",
        activeWorkspaceId: "ws-1",
      }),
    );

    act(() => {
      result.current.selectOpenCodeAgentForThread("opencode-pending-1", "agent-a");
      result.current.syncActiveOpenCodeThread("opencode-pending-1");
      result.current.syncActiveOpenCodeThread("opencode:session-1");
    });

    expect(result.current.resolveOpenCodeAgentForThread("opencode:session-1")).toBe(
      "agent-a",
    );
  });
});
