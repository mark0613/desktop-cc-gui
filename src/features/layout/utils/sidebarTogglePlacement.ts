type SidebarTogglePlacementArgs = {
  isCompact: boolean;
  isMacDesktop: boolean;
  isSoloMode: boolean;
  sidebarCollapsed: boolean;
};

export function shouldShowSidebarTopbarSidebarToggle({
  isCompact,
  isMacDesktop,
  isSoloMode,
  sidebarCollapsed,
}: SidebarTogglePlacementArgs): boolean {
  return !isCompact && isMacDesktop && !isSoloMode && !sidebarCollapsed;
}

export function shouldShowMainTopbarSidebarToggle({
  isCompact,
  isMacDesktop,
  isSoloMode,
  sidebarCollapsed,
}: SidebarTogglePlacementArgs): boolean {
  return !isCompact && !isSoloMode && (!isMacDesktop || sidebarCollapsed);
}

export function shouldShowFloatingTitlebarSidebarToggle({
  showHome,
  showMainTopbarSidebarToggle,
}: {
  showHome: boolean;
  showMainTopbarSidebarToggle: boolean;
}): boolean {
  return showHome && showMainTopbarSidebarToggle;
}
