type HomeWorkspaceOption = {
  id: string;
  name: string;
};

type HomeWorkspaceGroup = {
  id?: string | null;
  name?: string;
  workspaces: HomeWorkspaceOption[];
};

export function getHomeWorkspaceOptions(
  groupedWorkspaces: HomeWorkspaceGroup[],
  workspaces: HomeWorkspaceOption[],
): HomeWorkspaceOption[] {
  const groupedOptions = groupedWorkspaces.flatMap((group) => group.workspaces ?? []);
  return groupedOptions.length > 0 ? groupedOptions : workspaces;
}

export function resolveHomeWorkspaceId(
  selectedWorkspaceId: string | null | undefined,
  workspaces: HomeWorkspaceOption[],
): string | null {
  if (
    selectedWorkspaceId &&
    workspaces.some((workspace) => workspace.id === selectedWorkspaceId)
  ) {
    return selectedWorkspaceId;
  }

  return workspaces[0]?.id ?? null;
}
