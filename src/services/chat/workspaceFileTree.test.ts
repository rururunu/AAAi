import { describe, expect, it } from "vitest";
import { buildWorkspaceFileTree } from "./workspaceFileTree";

describe("buildWorkspaceFileTree", () => {
  it("groups files under directories and sorts dirs first", () => {
    const tree = buildWorkspaceFileTree([
      "readme.md",
      "src/main.ts",
      "src/util/a.ts",
      "src/util/b.ts",
      "docs/guide.md",
    ]);
    expect(tree.map((node) => node.name)).toEqual(["docs", "src", "readme.md"]);
    expect(tree[1]?.children?.map((node) => node.name)).toEqual(["util", "main.ts"]);
    expect(tree[1]?.children?.[0]?.children?.map((node) => node.path)).toEqual([
      "src/util/a.ts",
      "src/util/b.ts",
    ]);
  });
});
