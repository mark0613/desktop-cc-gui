export type KanbanContextMode = "new" | "inherit";
export type KanbanTaskEngine = "claude" | "codex" | "gemini" | "opencode";

type ResolveKanbanThreadCreationStrategyInput = {
  mode: KanbanContextMode;
  engine: KanbanTaskEngine;
  activeThreadId: string | null;
  activeThreadEngine?: KanbanTaskEngine | null;
  activeWorkspaceId: string | null;
  targetWorkspaceId: string;
  isActiveThreadInWorkspace: boolean;
};

function inferKanbanThreadEngine(
  threadId: string | null | undefined,
): KanbanTaskEngine | null {
  const normalized = threadId?.trim().toLowerCase() ?? "";
  if (!normalized) {
    return null;
  }
  if (
    normalized.startsWith("claude:") ||
    normalized.startsWith("claude-pending-")
  ) {
    return "claude";
  }
  if (
    normalized.startsWith("gemini:") ||
    normalized.startsWith("gemini-pending-")
  ) {
    return "gemini";
  }
  if (
    normalized.startsWith("opencode:") ||
    normalized.startsWith("opencode-pending-")
  ) {
    return "opencode";
  }
  if (
    normalized.startsWith("codex:") ||
    normalized.startsWith("codex-pending-")
  ) {
    return "codex";
  }
  return null;
}

export function isKanbanThreadCompatibleWithEngine(input: {
  engine: KanbanTaskEngine;
  threadId: string | null | undefined;
  threadEngine?: KanbanTaskEngine | null;
}): boolean {
  if (!input.threadId) {
    return false;
  }
  const resolvedThreadEngine =
    input.threadEngine ?? inferKanbanThreadEngine(input.threadId);
  if (resolvedThreadEngine) {
    return resolvedThreadEngine === input.engine;
  }
  return input.engine === "codex";
}

export function resolveKanbanThreadCreationStrategy(
  input: ResolveKanbanThreadCreationStrategyInput,
): "new" | "inherit" {
  if (input.mode !== "inherit") {
    return "new";
  }
  if (!input.activeThreadId) {
    return "new";
  }
  if (!input.activeWorkspaceId || input.activeWorkspaceId !== input.targetWorkspaceId) {
    return "new";
  }
  if (!input.isActiveThreadInWorkspace) {
    return "new";
  }
  if (
    !isKanbanThreadCompatibleWithEngine({
      engine: input.engine,
      threadId: input.activeThreadId,
      threadEngine: input.activeThreadEngine ?? null,
    })
  ) {
    return "new";
  }
  return "inherit";
}
