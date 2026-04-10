import { useEffect } from "react";
import { isMacPlatform, matchesShortcut } from "../../../utils/shortcuts";

type UsePrimaryModeShortcutsOptions = {
  isEnabled: boolean;
  onOpenChat: () => void;
  onOpenKanban: () => void;
};

function isEditableTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  if (target.isContentEditable) {
    return true;
  }
  return Boolean(
    target.closest(
      'input, textarea, select, [contenteditable=""], [contenteditable="true"], [role="textbox"]',
    ),
  );
}

export function usePrimaryModeShortcuts({
  isEnabled,
  onOpenChat,
  onOpenKanban,
}: UsePrimaryModeShortcutsOptions) {
  useEffect(() => {
    if (!isEnabled) {
      return;
    }
    const isMac = isMacPlatform();
    const chatShortcut = isMac ? "cmd+j" : "ctrl+j";
    const kanbanShortcut = isMac ? "cmd+k" : "ctrl+k";

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.repeat) {
        return;
      }
      if (isEditableTarget(event.target)) {
        return;
      }
      const matchesChatShortcut = matchesShortcut(event, chatShortcut);
      const matchesKanbanShortcut = matchesShortcut(event, kanbanShortcut);
      if (!matchesChatShortcut && !matchesKanbanShortcut) {
        return;
      }
      event.preventDefault();
      if (matchesChatShortcut) {
        onOpenChat();
        return;
      }
      onOpenKanban();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isEnabled, onOpenChat, onOpenKanban]);
}
