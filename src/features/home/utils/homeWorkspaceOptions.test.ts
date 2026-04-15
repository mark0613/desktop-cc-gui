import { describe, expect, it } from "vitest";
import { getHomeWorkspaceOptions, resolveHomeWorkspaceId } from "./homeWorkspaceOptions";

describe("homeWorkspaceOptions", () => {
  it("prefers grouped workspace order when available", () => {
    const options = getHomeWorkspaceOptions(
      [
        {
          id: "group-1",
          name: "Main",
          workspaces: [
            { id: "ws-2", name: "beta" },
            { id: "ws-1", name: "alpha" },
          ],
        },
      ],
      [{ id: "fallback-1", name: "fallback" }],
    );

    expect(options.map((option) => option.id)).toEqual(["ws-2", "ws-1"]);
  });

  it("falls back to raw workspaces when grouped sections are empty", () => {
    const options = getHomeWorkspaceOptions([], [
      { id: "ws-1", name: "alpha" },
      { id: "ws-2", name: "beta" },
    ]);

    expect(options.map((option) => option.id)).toEqual(["ws-1", "ws-2"]);
  });

  it("keeps the selected workspace when it still exists", () => {
    const selectedWorkspaceId = resolveHomeWorkspaceId("ws-2", [
      { id: "ws-1", name: "alpha" },
      { id: "ws-2", name: "beta" },
    ]);

    expect(selectedWorkspaceId).toBe("ws-2");
  });

  it("falls back to the first workspace when the selected one is missing", () => {
    const selectedWorkspaceId = resolveHomeWorkspaceId("missing", [
      { id: "ws-1", name: "alpha" },
      { id: "ws-2", name: "beta" },
    ]);

    expect(selectedWorkspaceId).toBe("ws-1");
  });
});
