import { describe, expect, it } from "vitest";
import { githubRawSkillCandidates, skillInstallName } from "./smithery";

describe("smithery skill helpers", () => {
  it("builds raw SKILL.md candidates from a GitHub tree URL", () => {
    expect(
      githubRawSkillCandidates(
        "https://github.com/affaan-m/everything-claude-code/tree/main/skills/frontend-patterns",
      ),
    ).toEqual([
      "https://raw.githubusercontent.com/affaan-m/everything-claude-code/main/skills/frontend-patterns/SKILL.md",
      "https://raw.githubusercontent.com/affaan-m/everything-claude-code/main/skills/frontend-patterns/skill.md",
      "https://raw.githubusercontent.com/affaan-m/everything-claude-code/main/skills/frontend-patterns/README.md",
    ]);
  });

  it("sanitizes install names", () => {
    expect(
      skillInstallName({
        id: "1",
        namespace: "ns",
        slug: "Frontend Design!",
        displayName: "Frontend Design",
      }),
    ).toBe("ns.frontend-design");

    expect(
      skillInstallName({
        id: "",
        namespace: "",
        slug: "",
        displayName: "Only Display Name!",
      }),
    ).toBe("only-display-name");
  });
});
