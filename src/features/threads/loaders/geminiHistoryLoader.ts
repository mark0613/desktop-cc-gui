import type { HistoryLoader } from "../contracts/conversationCurtainContracts";
import { normalizeHistorySnapshot } from "../contracts/conversationCurtainContracts";
import { parseClaudeHistoryMessages } from "./claudeHistoryLoader";

type GeminiHistoryLoaderOptions = {
  workspaceId: string;
  workspacePath: string | null;
  loadGeminiSession: (
    workspacePath: string,
    sessionId: string,
  ) => Promise<unknown>;
};

export function createGeminiHistoryLoader({
  workspaceId,
  workspacePath,
  loadGeminiSession,
}: GeminiHistoryLoaderOptions): HistoryLoader {
  return {
    engine: "gemini",
    async load(threadId: string) {
      const sessionId = threadId.startsWith("gemini:")
        ? threadId.slice("gemini:".length)
        : threadId;
      if (!workspacePath) {
        return normalizeHistorySnapshot({
          engine: "gemini",
          workspaceId,
          threadId,
          meta: {
            workspaceId,
            threadId,
            engine: "gemini",
            activeTurnId: null,
            isThinking: false,
            heartbeatPulse: null,
            historyRestoredAtMs: Date.now(),
          },
        });
      }

      const result = await loadGeminiSession(workspacePath, sessionId);
      const record = result as { messages?: unknown };
      const messagesData = record.messages ?? result;
      const items = parseClaudeHistoryMessages(messagesData);

      return normalizeHistorySnapshot({
        engine: "gemini",
        workspaceId,
        threadId,
        items,
        plan: null,
        userInputQueue: [],
        meta: {
          workspaceId,
          threadId,
          engine: "gemini",
          activeTurnId: null,
          isThinking: false,
          heartbeatPulse: null,
          historyRestoredAtMs: Date.now(),
        },
      });
    },
  };
}
