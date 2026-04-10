const DEFAULT_WORKSPACE_SUFFIXES = [
  "/.ccgui/workspace",
  "/.mossx/workspace",
  "/.codemoss/workspace",
  "/com.zhukunpenglinyutong.ccgui/workspace",
  "/com.zhukunpenglinyutong.mossx/workspace",
  "/com.zhukunpenglinyutong.codemoss/workspace",
];

export function normalizeWorkspacePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

export function isDefaultWorkspacePath(path: string): boolean {
  const normalized = normalizeWorkspacePath(path);
  return DEFAULT_WORKSPACE_SUFFIXES.some((suffix) => normalized.endsWith(suffix));
}
