import GitBranch from "lucide-react/dist/esm/icons/git-branch";
import type { MouseEvent } from "react";

import type { WorkspaceInfo } from "../../../types";

type WorktreeCardProps = {
  worktree: WorkspaceInfo;
  isActive: boolean;
  hasPrimaryActiveThread: boolean;
  hasRunningSession?: boolean;
  threadCount: number;
  hasThreadCursor: boolean;
  isDeleting?: boolean;
  onShowWorktreeMenu: (event: MouseEvent, workspaceId: string) => void;
  onToggleWorkspaceCollapse: (workspaceId: string, collapsed: boolean) => void;
  onConnectWorkspace: (workspace: WorkspaceInfo) => void;
  children?: React.ReactNode;
};

type ParsedWorktreeName = {
  prefix: string | null;
  leaf: string;
};

function parseWorktreeName(rawName: string): ParsedWorktreeName {
  const normalized = rawName.trim();
  if (!normalized) {
    return { prefix: null, leaf: rawName };
  }
  const slashIndex = normalized.lastIndexOf("/");
  if (slashIndex <= 0 || slashIndex >= normalized.length - 1) {
    return { prefix: null, leaf: normalized };
  }
  return {
    prefix: normalized.slice(0, slashIndex),
    leaf: normalized.slice(slashIndex + 1),
  };
}

export function WorktreeCard({
  worktree,
  isActive,
  hasPrimaryActiveThread,
  hasRunningSession = false,
  threadCount,
  hasThreadCursor,
  isDeleting = false,
  onShowWorktreeMenu,
  onToggleWorkspaceCollapse,
  onConnectWorkspace,
  children,
}: WorktreeCardProps) {
  const worktreeCollapsed = worktree.settings.sidebarCollapsed;
  const worktreeBranch = worktree.worktree?.branch ?? "";
  const displayName = worktreeBranch || worktree.name;
  const parsedName = parseWorktreeName(displayName);
  const handleToggleCollapse = () => {
    onToggleWorkspaceCollapse(worktree.id, !worktreeCollapsed);
  };

  return (
    <div className={`worktree-card${isDeleting ? " deleting" : ""}`}>
      <div
        className={`worktree-row ${
          isActive
            ? hasPrimaryActiveThread
              ? "context-active"
              : "active"
            : ""
        }${isDeleting ? " deleting" : ""}`}
        role="button"
        tabIndex={isDeleting ? -1 : 0}
        aria-disabled={isDeleting}
        aria-expanded={!worktreeCollapsed}
        onClick={(event) => {
          if (!isDeleting && event.detail <= 1) {
            handleToggleCollapse();
          }
        }}
        onContextMenu={(event) => {
          if (!isDeleting) {
            onShowWorktreeMenu(event, worktree.id);
          }
        }}
        title={displayName}
        onKeyDown={(event) => {
          if (isDeleting) {
            return;
          }
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            handleToggleCollapse();
          }
        }}
      >
        <GitBranch
          className={`worktree-node-icon${hasRunningSession ? " is-session-running" : ""}`}
          aria-hidden
        />
        <div className="worktree-label-wrap">
          {parsedName.prefix ? (
            <span className="worktree-label-prefix">{parsedName.prefix}</span>
          ) : null}
          <div className="worktree-label">{parsedName.leaf}</div>
        </div>
        <div className="worktree-actions">
          {isDeleting ? (
            <div className="worktree-deleting" role="status" aria-live="polite">
              <span className="worktree-deleting-spinner" aria-hidden />
              <span className="worktree-deleting-label">Deleting</span>
            </div>
          ) : (
            <>
              {(threadCount > 0 || hasThreadCursor) && (
                <span className="worktree-thread-count" aria-label={`Threads: ${threadCount}`}>
                  {threadCount > 0 ? threadCount : "0"}
                  {hasThreadCursor ? "+" : ""}
                </span>
              )}
              <button
                className={`worktree-toggle ${worktreeCollapsed ? "" : "expanded"}`}
                onClick={(event) => {
                  event.stopPropagation();
                  handleToggleCollapse();
                }}
                onDoubleClick={(event) => {
                  event.stopPropagation();
                }}
                data-tauri-drag-region="false"
                aria-label={worktreeCollapsed ? "Show agents" : "Hide agents"}
                aria-expanded={!worktreeCollapsed}
              >
                <span className="worktree-toggle-icon">›</span>
              </button>
              {!worktree.connected && (
                <span
                  className="connect"
                  onClick={(event) => {
                    event.stopPropagation();
                    onConnectWorkspace(worktree);
                  }}
                  onDoubleClick={(event) => {
                    event.stopPropagation();
                  }}
                >
                  connect
                </span>
              )}
            </>
          )}
        </div>
      </div>
      {children}
    </div>
  );
}
