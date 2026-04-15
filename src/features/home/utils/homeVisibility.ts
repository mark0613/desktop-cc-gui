export function shouldHideHomeOnThreadActivation(params: {
  homeOpen: boolean;
  activeThreadId: string | null | undefined;
}): boolean {
  return Boolean(params.homeOpen && params.activeThreadId);
}
