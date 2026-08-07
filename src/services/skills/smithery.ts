/**
 * Smithery Skills Registry client (https://registry.smithery.ai).
 * Browse/search prompt skills and install them into the local AAAi skills dir.
 */

const REGISTRY_BASE = "https://registry.smithery.ai";

/** Official Smithery skill categories (exact API `category` values). */
export const SMITHERY_SKILL_CATEGORIES = [
  "Research",
  "Coding",
  "Writing",
  "Data & Analytics",
  "Design",
  "Planning",
  "Communication",
  "Productivity",
  "DevOps",
  "AI & ML",
  "Security",
  "Business",
] as const;

export type SmitherySkillCategory = (typeof SMITHERY_SKILL_CATEGORIES)[number];

export type SmitherySkillSummary = {
  id: string;
  namespace: string;
  slug: string;
  qualifiedName?: string;
  displayName: string;
  description: string;
  prompt?: string | null;
  gitUrl?: string | null;
  verified?: boolean;
  categories?: string[];
  totalActivations?: number;
  uniqueUsers?: number;
  qualityScore?: number;
  externalStars?: number;
};

export type SmitherySkillsPage = {
  skills: SmitherySkillSummary[];
  pagination: {
    currentPage: number;
    pageSize: number;
    totalPages: number;
    totalCount: number;
  };
};

export function skillInstallName(
  skill: Pick<SmitherySkillSummary, "namespace" | "slug" | "id"> & {
    displayName?: string;
  },
): string {
  const ns = (skill.namespace || "").trim();
  const slug = (skill.slug || "").trim();
  // Prefer namespace.slug so two skills with the same slug never collide.
  const raw = ns && slug ? `${ns}.${slug}` : slug || skill.id || skill.displayName || "skill";
  return (
    raw
      .replace(/[^A-Za-z0-9_.-]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 96)
      .toLowerCase() || "skill"
  );
}

export type SkillInstallMeta = {
  id: string;
  namespace: string;
  slug: string;
  qualifiedName: string;
  displayName: string;
  description: string;
  iconUrl?: string | null;
  gitUrl?: string | null;
  verified?: boolean;
  categories?: string[];
  source: "smithery";
  registryId: string;
};

export function buildSkillInstallMeta(skill: SmitherySkillSummary): SkillInstallMeta {
  const qualifiedName =
    skill.qualifiedName?.trim() || `${skill.namespace}/${skill.slug}`.replace(/\/+/g, "/");
  return {
    id: skillInstallName(skill),
    namespace: skill.namespace,
    slug: skill.slug,
    qualifiedName,
    displayName: skill.displayName || skill.slug,
    description: skill.description || "",
    iconUrl: smitherySkillIconUrl(skill),
    gitUrl: skill.gitUrl ?? null,
    verified: skill.verified,
    categories: skill.categories,
    source: "smithery",
    registryId: skill.id,
  };
}

/** Match installed skill to a Smithery entry by registry identity. */
export function isSameSkillInstall(
  installed: {
    name: string;
    qualifiedName?: string | null;
    registryId?: string | null;
    namespace?: string | null;
    slug?: string | null;
  },
  candidate: Pick<SmitherySkillSummary, "id" | "namespace" | "slug" | "qualifiedName">,
): boolean {
  if (candidate.id && installed.registryId === candidate.id) return true;
  const qn = candidate.qualifiedName?.trim() || `${candidate.namespace}/${candidate.slug}`;
  if (installed.qualifiedName && installed.qualifiedName === qn) return true;
  if (
    installed.namespace &&
    installed.slug &&
    installed.namespace === candidate.namespace &&
    installed.slug === candidate.slug
  ) {
    return true;
  }
  return installed.name === skillInstallName(candidate);
}

/** Format star counts like Smithery UI: 63171 → "63.17k stars". */
export function formatSmitheryStars(count: number): string {
  const n = Math.max(0, Math.floor(count));
  let compact: string;
  if (n >= 1_000_000) {
    const m = n / 1_000_000;
    compact = `${m >= 10 ? m.toFixed(1) : m.toFixed(2).replace(/0+$/, "").replace(/\.$/, "")}M`;
  } else if (n >= 1_000) {
    const k = n / 1_000;
    compact = `${k >= 100 ? k.toFixed(0) : k.toFixed(2).replace(/0+$/, "").replace(/\.$/, "")}k`;
  } else {
    compact = String(n);
  }
  return `${compact} stars`;
}

/** Format activation counts like Smithery UI: 63171 → "63.17k uses". */
export function formatSmitheryUses(count: number): string {
  const n = Math.max(0, Math.floor(count));
  let compact: string;
  if (n >= 1_000_000) {
    const m = n / 1_000_000;
    compact = `${m >= 10 ? m.toFixed(1) : m.toFixed(2).replace(/0+$/, "").replace(/\.$/, "")}M`;
  } else if (n >= 1_000) {
    const k = n / 1_000;
    compact = `${k >= 100 ? k.toFixed(0) : k.toFixed(2).replace(/0+$/, "").replace(/\.$/, "")}k`;
  } else {
    compact = String(n);
  }
  return `${compact} uses`;
}

export async function searchSmitherySkills(
  query: string,
  options: { page?: number; pageSize?: number; category?: string | null } = {},
): Promise<SmitherySkillsPage> {
  const params = new URLSearchParams();
  const q = query.trim();
  if (q) params.set("q", q);
  const category = options.category?.trim();
  if (category) params.set("category", category);
  params.set("page", String(options.page ?? 1));
  params.set("pageSize", String(options.pageSize ?? 20));

  const response = await fetch(`${REGISTRY_BASE}/skills?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Smithery registry error (${response.status})`);
  }
  const data = (await response.json()) as SmitherySkillsPage;
  const skills = Array.isArray(data.skills) ? data.skills : [];
  return {
    // Prefer GitHub stars, then activations.
    skills: sortSmitherySkillsByStars(skills),
    pagination: data.pagination ?? {
      currentPage: options.page ?? 1,
      pageSize: options.pageSize ?? 20,
      totalPages: 1,
      totalCount: 0,
    },
  };
}

export function sortSmitherySkillsByStars(skills: SmitherySkillSummary[]): SmitherySkillSummary[] {
  return [...skills].sort((a, b) => {
    const byStars = (b.externalStars ?? 0) - (a.externalStars ?? 0);
    if (byStars !== 0) return byStars;
    const byActivations = (b.totalActivations ?? 0) - (a.totalActivations ?? 0);
    if (byActivations !== 0) return byActivations;
    return (b.uniqueUsers ?? 0) - (a.uniqueUsers ?? 0);
  });
}

/** Skills API has no iconUrl — use the GitHub org/user avatar for the namespace. */
export function smitherySkillIconUrl(
  skill: Pick<SmitherySkillSummary, "namespace" | "gitUrl">,
): string | null {
  const fromGit = skill.gitUrl?.match(/^https?:\/\/github\.com\/([^/]+)(?:\/|$)/i)?.[1];
  const owner = (fromGit || skill.namespace || "").trim();
  if (!owner || owner === "." || owner === "..") return null;
  return `https://github.com/${encodeURIComponent(owner)}.png?size=80`;
}

export async function getSmitherySkill(
  namespace: string,
  slug: string,
): Promise<SmitherySkillSummary> {
  const response = await fetch(
    `${REGISTRY_BASE}/skills/${encodeURIComponent(namespace)}/${encodeURIComponent(slug)}`,
  );
  if (!response.ok) {
    throw new Error(`Smithery skill not found (${response.status})`);
  }
  return (await response.json()) as SmitherySkillSummary;
}

/** Convert a GitHub tree URL into candidate raw SKILL.md URLs. */
export function githubRawSkillCandidates(gitUrl: string): string[] {
  const trimmed = gitUrl.trim();
  const treeMatch = trimmed.match(
    /^https?:\/\/github\.com\/([^/]+)\/([^/]+)\/tree\/([^/]+)\/(.+?)\/?$/i,
  );
  if (treeMatch) {
    const [, owner, repo, branch, path] = treeMatch;
    const base = `https://raw.githubusercontent.com/${owner}/${repo}/${branch}/${path.replace(/\/$/, "")}`;
    return [`${base}/SKILL.md`, `${base}/skill.md`, `${base}/README.md`];
  }
  const blobMatch = trimmed.match(
    /^https?:\/\/github\.com\/([^/]+)\/([^/]+)\/blob\/([^/]+)\/(.+\.md)$/i,
  );
  if (blobMatch) {
    const [, owner, repo, branch, path] = blobMatch;
    return [`https://raw.githubusercontent.com/${owner}/${repo}/${branch}/${path}`];
  }
  return [];
}

async function fetchText(url: string): Promise<string | null> {
  try {
    const response = await fetch(url);
    if (!response.ok) return null;
    const text = await response.text();
    return text.trim() ? text : null;
  } catch {
    return null;
  }
}

/**
 * Resolve installable markdown for a Smithery skill.
 * Prefers SKILL.md from gitUrl; falls back to the registry `prompt` field.
 */
export async function resolveSmitherySkillMarkdown(skill: SmitherySkillSummary): Promise<string> {
  const detail =
    skill.gitUrl || skill.prompt ? skill : await getSmitherySkill(skill.namespace, skill.slug);

  if (detail.gitUrl) {
    for (const url of githubRawSkillCandidates(detail.gitUrl)) {
      const body = await fetchText(url);
      if (body) return body;
    }
  }

  const prompt = (detail.prompt ?? skill.prompt ?? "").trim();
  if (prompt.length >= 40) {
    return [
      "---",
      `name: ${skillInstallName(skill)}`,
      `description: ${detail.description || skill.description || skill.displayName}`,
      `qualifiedName: ${detail.qualifiedName || `${skill.namespace}/${skill.slug}`}`,
      `namespace: ${skill.namespace}`,
      `slug: ${skill.slug}`,
      `registryId: ${skill.id}`,
      "source: smithery",
      "---",
      "",
      prompt,
      "",
    ].join("\n");
  }

  throw new Error("Could not download SKILL.md from Smithery (missing git source or prompt).");
}
