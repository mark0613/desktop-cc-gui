import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { EngineIcon } from "./EngineIcon";

describe("EngineIcon", () => {
  it("renders the Codex icon as a monochrome svg glyph", () => {
    const markup = renderToStaticMarkup(<EngineIcon engine="codex" size={16} />);

    expect(markup).toContain("<svg");
    expect(markup).toContain("fill=\"currentColor\"");
    expect(markup).not.toContain("<img");
  });

  it("keeps Claude as an image asset", () => {
    const markup = renderToStaticMarkup(<EngineIcon engine="claude" size={16} />);

    expect(markup).toContain("<img");
  });

  it("renders the OpenCode icon as a monochrome svg glyph", () => {
    const markup = renderToStaticMarkup(<EngineIcon engine="opencode" size={16} />);

    expect(markup).toContain("<svg");
    expect(markup).toContain("stroke=\"currentColor\"");
    expect(markup).not.toContain("<img");
  });
});
