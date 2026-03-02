import { useCallback } from "react";
import type { Dispatch } from "react";
import type { RequestUserInputRequest } from "../../../types";
import type { ThreadAction } from "./useThreadsReducer";

type UseThreadUserInputEventsOptions = {
  dispatch: Dispatch<ThreadAction>;
};

export function useThreadUserInputEvents({ dispatch }: UseThreadUserInputEventsOptions) {
  return useCallback(
    (request: RequestUserInputRequest) => {
      if (request.params.completed === true) {
        dispatch({
          type: "removeUserInputRequest",
          requestId: request.request_id,
          workspaceId: request.workspace_id,
        });
        return;
      }
      dispatch({ type: "addUserInputRequest", request });
    },
    [dispatch],
  );
}
