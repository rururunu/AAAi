import { listSkills } from "@/commands/skills";
import { warmInstallIcons, type IconWarmEntry } from "@/services/iconCache";
import type { McpServerConfig } from "@/types/setting";

/** Prefetch install-scoped MCP/Skill icons into memory (and backfill disk). */
export async function warmInstalledResourceIcons(
  mcpServers: readonly McpServerConfig[] = [],
): Promise<void> {
  const entries: IconWarmEntry[] = (mcpServers ?? []).map((server) => ({
    kind: "mcp",
    cacheKey: server.id,
    url: server.iconUrl,
  }));

  try {
    const skills = await listSkills();
    for (const skill of skills) {
      if (skill.source !== "user") continue;
      entries.push({
        kind: "skill",
        cacheKey: skill.name,
        url: skill.iconUrl,
      });
    }
  } catch {
    // Skills may be unavailable during early boot; MCP warm still helps.
  }

  await warmInstallIcons(entries);
}
