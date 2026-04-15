import { describe, expect, it } from "vitest";
import { shouldHideHomeOnThreadActivation } from "./homeVisibility";

describe("shouldHideHomeOnThreadActivation", () => {
  it("returns true when a thread becomes active while home is open", () => {
    expect(
      shouldHideHomeOnThreadActivation({
        homeOpen: true,
        activeThreadId: "thread-1",
      }),
    ).toBe(true);
  });

  it("returns false when home is already closed", () => {
    expect(
      shouldHideHomeOnThreadActivation({
        homeOpen: false,
        activeThreadId: "thread-1",
      }),
    ).toBe(false);
  });

  it("returns false when there is no active thread yet", () => {
    expect(
      shouldHideHomeOnThreadActivation({
        homeOpen: true,
        activeThreadId: null,
      }),
    ).toBe(false);
  });
});
