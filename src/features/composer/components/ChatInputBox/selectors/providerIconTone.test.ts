import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("provider icon tones", () => {
  it("does not hardcode a purple OpenCode icon in selector components", () => {
    const providerSelect = readFileSync(
      resolve(
        process.cwd(),
        "src/features/composer/components/ChatInputBox/selectors/ProviderSelect.tsx",
      ),
      "utf8",
    );
    const configSelect = readFileSync(
      resolve(
        process.cwd(),
        "src/features/composer/components/ChatInputBox/selectors/ConfigSelect.tsx",
      ),
      "utf8",
    );

    expect(providerSelect).not.toContain("#6366f1");
    expect(configSelect).not.toContain("#6366f1");
  });
});
