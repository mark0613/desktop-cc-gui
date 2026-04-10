export const MESSAGES_LIVE_AUTO_FOLLOW_FLAG_KEY = "ccgui.messages.live.autoFollow";
export const MESSAGES_LIVE_COLLAPSE_MIDDLE_STEPS_FLAG_KEY =
  "ccgui.messages.live.collapseMiddleSteps";
export const MESSAGES_LIVE_CONTROLS_UPDATED_EVENT = "ccgui:messages-live-controls-updated";

export function readLocalBooleanFlag(storageKey: string, fallbackValue: boolean): boolean {
  if (typeof window === "undefined") {
    return fallbackValue;
  }
  try {
    const value = window.localStorage.getItem(storageKey);
    if (!value) {
      return fallbackValue;
    }
    const normalized = value.trim().toLowerCase();
    if (normalized === "1" || normalized === "true" || normalized === "on") {
      return true;
    }
    if (normalized === "0" || normalized === "false" || normalized === "off") {
      return false;
    }
    return fallbackValue;
  } catch {
    return fallbackValue;
  }
}

export function writeLocalBooleanFlag(storageKey: string, enabled: boolean): void {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(storageKey, enabled ? "1" : "0");
  } catch {
    // Ignore storage write errors in restricted contexts.
  }
}
