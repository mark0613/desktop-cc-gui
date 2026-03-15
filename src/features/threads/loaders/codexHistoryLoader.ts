import { buildItemsFromThread, mergeThreadItems } from "../../../utils/threadItems";
import type { HistoryLoader } from "../contracts/conversationCurtainContracts";
import { normalizeHistorySnapshot } from "../contracts/conversationCurtainContracts";
import { parseCodexSessionHistory } from "./codexSessionHistory";
import { asRecord } from "./historyLoaderUtils";
import { extractLatestTurnPlan } from "./historyLoaderUtils";
import { extractUserInputQueueFromThread } from "./historyLoaderUtils";

type CodexHistoryLoaderOptions = {
  workspaceId: string;
  resumeThread: (
    workspaceId: string,
    threadId: string,
  ) => Promise<Record<string, unknown> | null>;
  loadCodexSession?: (workspaceId: string, threadId: string) => Promise<unknown>;
};

export function createCodexHistoryLoader({
  workspaceId,
  resumeThread,
  loadCodexSession,
}: CodexHistoryLoaderOptions): HistoryLoader {
  return {
    engine: "codex",
    async load(threadId: string) {
      const response = await resumeThread(workspaceId, threadId);
      const result = asRecord(response?.result ?? response);
      const thread = asRecord(result.thread ?? response?.thread);
      const hasThread = Object.keys(thread).length > 0;
      const historyItems = hasThread ? buildItemsFromThread(thread) : [];
      let items = historyItems;

      if (loadCodexSession) {
        try {
          const fallbackHistory = await loadCodexSession(workspaceId, threadId);
          const fallbackItems = parseCodexSessionHistory(fallbackHistory);
          const structuredFallback = historyItems.length > 0
            ? fallbackItems.filter((item) => item.kind !== "message")
            : fallbackItems;
          items = mergeThreadItems(historyItems, structuredFallback);
        } catch (error) {
          console.warn("Failed to load Codex local history fallback", {
            workspaceId,
            threadId,
            error,
          });
        }
      }

      return normalizeHistorySnapshot({
        engine: "codex",
        workspaceId,
        threadId,
        items,
        plan: hasThread ? extractLatestTurnPlan(thread) : undefined,
        userInputQueue: hasThread
          ? extractUserInputQueueFromThread(thread, workspaceId, threadId)
          : [],
        meta: {
          workspaceId,
          threadId,
          engine: "codex",
          activeTurnId: null,
          isThinking: false,
          heartbeatPulse: null,
          historyRestoredAtMs: Date.now(),
        },
      });
    },
  };
}
