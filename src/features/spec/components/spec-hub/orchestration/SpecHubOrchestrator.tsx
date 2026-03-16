import { SpecHubPresentational, type SpecHubProps } from "../../SpecHub.presentational";
import { useSpecHubOrchestration } from "../hooks/useSpecHubOrchestration";

export function SpecHubOrchestrator(props: SpecHubProps) {
  const presentationalProps = useSpecHubOrchestration(props);
  return <SpecHubPresentational {...presentationalProps} />;
}
