import { describe, expect, it } from "vitest";
import type { ThreadSummary } from "../../../types";
import { initialState, threadReducer } from "./useThreadsReducer";

describe("threadReducer engine/source behavior", () => {
  it("resolves raw session id to existing prefixed engine thread", () => {
    const next = threadReducer(
      {
        ...initialState,
        threadsByWorkspace: {
          "ws-1": [
            {
              id: "opencode:session-xyz",
              name: "Agent 1",
              updatedAt: 1,
              engineSource: "opencode",
            },
          ],
        },
      },
      {
        type: "ensureThread",
        workspaceId: "ws-1",
        threadId: "session-xyz",
      },
    );

    expect(next.threadsByWorkspace["ws-1"]).toHaveLength(1);
    expect(next.threadsByWorkspace["ws-1"]?.[0]?.id).toBe("opencode:session-xyz");
    expect(next.threadStatusById["session-xyz"]).toBeUndefined();
  });

  it("updates thread engine source when requested", () => {
    const threads: ThreadSummary[] = [
      { id: "thread-1", name: "Agent 1", updatedAt: 1, engineSource: "codex" },
    ];
    const next = threadReducer(
      {
        ...initialState,
        threadsByWorkspace: { "ws-1": threads },
      },
      {
        type: "setThreadEngine",
        workspaceId: "ws-1",
        threadId: "thread-1",
        engine: "claude",
      },
    );
    expect(next.threadsByWorkspace["ws-1"]?.[0]?.engineSource).toBe("claude");
  });
});
