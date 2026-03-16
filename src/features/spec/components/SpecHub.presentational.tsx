import { SpecHubPresentational as SpecHubPresentationalImpl } from "./spec-hub/presentational/SpecHubPresentationalImpl";

export type SpecHubProps = {
  workspaceId: string | null;
  workspaceName: string | null;
  files: string[];
  directories: string[];
  onBackToChat: () => void;
};

export function SpecHubPresentational(props: SpecHubProps) {
  return <SpecHubPresentationalImpl {...props} />;
}
