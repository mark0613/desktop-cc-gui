import type { RealtimeAdapter } from "../contracts/conversationCurtainContracts";
import { mapCommonRealtimeEvent } from "./sharedRealtimeAdapter";

export const geminiRealtimeAdapter: RealtimeAdapter = {
  engine: "gemini",
  mapEvent(input: unknown) {
    return mapCommonRealtimeEvent("gemini", input, {
      allowTextDeltaAlias: true,
    });
  },
};
