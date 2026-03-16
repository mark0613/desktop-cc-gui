import type { ReactNode } from "react";
import type { GitCommitFileChange } from "../../../../../types";

const FILE_TREE_ROOT_PATH = "__repo_root__";

type FileTreeNode = {
  name: string;
  path: string;
  dirs: Map<string, FileTreeNode>;
  files: GitCommitFileChange[];
};

type FileTreeItem =
  | {
      id: string;
      type: "dir";
      label: string;
      path: string;
      depth: number;
      expanded: boolean;
    }
  | {
      id: string;
      type: "file";
      label: string;
      path: string;
      depth: number;
      change: GitCommitFileChange;
    };

export function isRepositoryUnavailableError(error: unknown): boolean {
  const message =
    error instanceof Error ? error.message : typeof error === "string" ? error : "";
  const normalized = message.toLowerCase();
  return (
    normalized.includes("could not find repository") ||
    normalized.includes("not a git repository") ||
    normalized.includes("codenotfound") ||
    normalized.includes("class=repository")
  );
}

export function formatRelativeTime(
  timestampSec: number,
  translate: (key: string, options?: Record<string, unknown>) => string,
): string {
  const now = Date.now();
  const target = timestampSec * 1000;
  const delta = Math.floor((now - target) / 1000);
  if (delta < 60) return translate("git.historyTimeJustNow");
  if (delta < 3600) {
    return translate("git.historyTimeMinutesAgo", { count: Math.floor(delta / 60) });
  }
  if (delta < 86400) {
    return translate("git.historyTimeHoursAgo", { count: Math.floor(delta / 3600) });
  }
  if (delta < 604800) {
    return translate("git.historyTimeDaysAgo", { count: Math.floor(delta / 86400) });
  }
  return new Date(target).toLocaleDateString();
}

export function statusLabel(change: GitCommitFileChange): string {
  const oldPath = change.oldPath?.trim();
  if (change.status === "R" && oldPath && oldPath !== change.path) {
    return `${oldPath} -> ${change.path}`;
  }
  return change.path;
}

export function buildFileKey(change: GitCommitFileChange): string {
  return `${change.path}::${change.status}::${change.oldPath ?? ""}`;
}

export function getTreeLineOpacity(depth: number): string {
  if (depth <= 0) {
    return "0";
  }
  const opacity = Math.max(0.34, 1 - (depth - 1) * 0.14);
  return opacity.toFixed(2);
}

export function renderChangedFilesSummary(
  translate: (key: string, options?: Record<string, unknown>) => string,
  count: number,
  additions: number,
  deletions: number,
): ReactNode {
  const addToken = "__MOSS_HISTORY_ADD__";
  const delToken = "__MOSS_HISTORY_DEL__";
  const template = translate("git.historyChangedFilesSummary", {
    count,
    additions: addToken,
    deletions: delToken,
  });
  const addIndex = template.indexOf(addToken);
  const delIndex = template.indexOf(delToken);
  if (addIndex < 0 || delIndex < 0) {
    return template;
  }
  const firstToken =
    addIndex <= delIndex
      ? { type: "add" as const, index: addIndex, token: addToken }
      : { type: "del" as const, index: delIndex, token: delToken };
  const secondToken =
    firstToken.type === "add"
      ? { type: "del" as const, index: delIndex, token: delToken }
      : { type: "add" as const, index: addIndex, token: addToken };
  const beforeFirst = template.slice(0, firstToken.index);
  const between = template.slice(firstToken.index + firstToken.token.length, secondToken.index);
  const afterSecond = template.slice(secondToken.index + secondToken.token.length);
  const renderToken = (type: "add" | "del") =>
    type === "add" ? (
      <span className="git-history-diff-add">+{additions}</span>
    ) : (
      <span className="git-history-diff-del">-{deletions}</span>
    );
  return (
    <>
      {beforeFirst}
      {renderToken(firstToken.type)}
      {between}
      {renderToken(secondToken.type)}
      {afterSecond}
    </>
  );
}

export function getPathLeafName(path: string | null | undefined): string {
  if (!path) {
    return "";
  }
  const normalized = path.replace(/\\/g, "/").replace(/\/+$/, "");
  if (!normalized) {
    return "";
  }
  const parts = normalized.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? "";
}

export function collectDirPaths(files: GitCommitFileChange[]): Set<string> {
  const paths = new Set<string>([FILE_TREE_ROOT_PATH]);
  for (const file of files) {
    const parts = file.path.split("/").filter(Boolean);
    let current = "";
    for (let index = 0; index < parts.length - 1; index += 1) {
      current = current ? `${current}/${parts[index]}` : parts[index];
      paths.add(current);
    }
  }
  return paths;
}

export function pickSelectedFileKey(
  previousKey: string | null,
  files: GitCommitFileChange[],
): string | null {
  if (!files.length) {
    return null;
  }
  if (previousKey) {
    const exists = files.some((entry) => buildFileKey(entry) === previousKey);
    if (exists) {
      return previousKey;
    }
  }
  return buildFileKey(files[0]);
}

export function buildFileTreeItems(
  files: GitCommitFileChange[],
  expandedDirs: Set<string>,
  rootLabel?: string,
): FileTreeItem[] {
  const root: FileTreeNode = {
    name: "",
    path: "",
    dirs: new Map<string, FileTreeNode>(),
    files: [],
  };

  for (const file of files) {
    const parts = file.path.split("/").filter(Boolean);
    if (!parts.length) {
      root.files.push(file);
      continue;
    }

    let node = root;
    let currentPath = "";
    for (let index = 0; index < parts.length - 1; index += 1) {
      const part = parts[index];
      currentPath = currentPath ? `${currentPath}/${part}` : part;
      let child = node.dirs.get(part);
      if (!child) {
        child = {
          name: part,
          path: currentPath,
          dirs: new Map<string, FileTreeNode>(),
          files: [],
        };
        node.dirs.set(part, child);
      }
      node = child;
    }
    node.files.push(file);
  }

  const items: FileTreeItem[] = [];

  const collapseDirChain = (
    start: FileTreeNode,
  ): { node: FileTreeNode; label: string; path: string } => {
    return {
      node: start,
      label: start.name,
      path: start.path,
    };
  };

  const walk = (node: FileTreeNode, depth: number) => {
    const dirs = Array.from(node.dirs.values()).sort((a, b) =>
      a.name.localeCompare(b.name),
    );
    for (const dir of dirs) {
      const collapsed = collapseDirChain(dir);
      const expanded = expandedDirs.has(collapsed.path);
      items.push({
        id: `dir:${collapsed.path}`,
        type: "dir",
        label: collapsed.label,
        path: collapsed.path,
        depth,
        expanded,
      });
      if (expanded) {
        walk(collapsed.node, depth + 1);
      }
    }

    const leafFiles = node.files.slice().sort((a, b) =>
      a.path.localeCompare(b.path),
    );
    for (const file of leafFiles) {
      const segments = file.path.split("/").filter(Boolean);
      const label = segments[segments.length - 1] ?? file.path;
      items.push({
        id: `file:${buildFileKey(file)}`,
        type: "file",
        label,
        path: file.path,
        depth,
        change: file,
      });
    }
  };

  if (rootLabel && rootLabel.trim()) {
    const rootExpanded = expandedDirs.has(FILE_TREE_ROOT_PATH);
    items.push({
      id: `dir:${FILE_TREE_ROOT_PATH}`,
      type: "dir",
      label: rootLabel,
      path: FILE_TREE_ROOT_PATH,
      depth: 0,
      expanded: rootExpanded,
    });
    if (rootExpanded) {
      walk(root, 1);
    }
    return items;
  }

  walk(root, 0);
  return items;
}

export function getBranchScope(name: string): string {
  const slashIndex = name.indexOf("/");
  if (slashIndex <= 0) {
    return "__root__";
  }
  return name.slice(0, slashIndex);
}

export function getBranchLeafName(name: string): string {
  const slashIndex = name.indexOf("/");
  if (slashIndex <= 0) {
    return name;
  }
  return name.slice(slashIndex + 1);
}

export function trimRemotePrefix(name: string, remote: string): string {
  const prefix = `${remote}/`;
  if (!name.startsWith(prefix)) {
    return name;
  }
  return name.slice(prefix.length);
}

export function getSpecialBranchBadges(
  branchName: string,
  t: (key: string, options?: Record<string, unknown>) => string,
): string[] {
  const normalized = getBranchLeafName(branchName).toLowerCase();
  const badges: string[] = [];
  if (normalized === "main" || normalized === "master") {
    badges.push(t("git.historyBranchBadgeMain"));
  }
  if (normalized === "zh") {
    badges.push(t("git.historyBranchBadgeZh"));
  }
  return badges;
}
