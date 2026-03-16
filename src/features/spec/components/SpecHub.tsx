import { SpecHubOrchestrator } from "./spec-hub/orchestration/SpecHubOrchestrator";
import type { SpecHubProps } from "./SpecHub.presentational";

export function SpecHub(props: SpecHubProps) {
  return <SpecHubOrchestrator {...props} />;
}
