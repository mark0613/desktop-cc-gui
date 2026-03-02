import { isTauri } from "@tauri-apps/api/core";

let cachedIsWindows: boolean | null = null;

export function isWindowsPlatform(): boolean {
  if (cachedIsWindows !== null) {
    return cachedIsWindows;
  }
  try {
    if (!isTauri() || typeof navigator === "undefined") {
      cachedIsWindows = false;
      return false;
    }
    const platform =
      (
        navigator as Navigator & {
          userAgentData?: { platform?: string };
        }
      ).userAgentData?.platform ??
      navigator.platform ??
      "";
    cachedIsWindows = platform.toLowerCase().includes("win");
  } catch {
    cachedIsWindows = false;
  }
  return cachedIsWindows;
}
