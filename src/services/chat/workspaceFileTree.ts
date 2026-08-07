export type WorkspaceFileTreeNode = {
  name: string;
  /** Relative path from workspace root (`src/app.ts` or `src`). */
  path: string;
  kind: "file" | "dir";
  children?: WorkspaceFileTreeNode[];
};

/** Build a sorted directory tree from workspace-relative file paths. */
export function buildWorkspaceFileTree(paths: readonly string[]): WorkspaceFileTreeNode[] {
  type MutableNode = {
    name: string;
    path: string;
    kind: "file" | "dir";
    children?: Map<string, MutableNode>;
  };

  const root = new Map<string, MutableNode>();

  for (const raw of paths) {
    const normalized = raw.replace(/\\/g, "/").replace(/^\/+/, "").trim();
    if (!normalized) continue;
    const parts = normalized.split("/").filter(Boolean);
    if (parts.length === 0) continue;

    let cursor = root;
    let prefix = "";
    for (let i = 0; i < parts.length; i += 1) {
      const name = parts[i]!;
      prefix = prefix ? `${prefix}/${name}` : name;
      const isFile = i === parts.length - 1;
      let node = cursor.get(name);
      if (!node) {
        node = {
          name,
          path: prefix,
          kind: isFile ? "file" : "dir",
          children: isFile ? undefined : new Map(),
        };
        cursor.set(name, node);
      } else if (!isFile && node.kind === "file") {
        // Prefer directory when the same name appears as both (rare).
        node.kind = "dir";
        node.children = node.children ?? new Map();
      }
      if (!isFile) {
        node.children = node.children ?? new Map();
        cursor = node.children;
      }
    }
  }

  const toSorted = (map: Map<string, MutableNode>): WorkspaceFileTreeNode[] => {
    const nodes = [...map.values()].map((node) => {
      if (node.kind === "dir") {
        return {
          name: node.name,
          path: node.path,
          kind: "dir" as const,
          children: toSorted(node.children ?? new Map()),
        };
      }
      return {
        name: node.name,
        path: node.path,
        kind: "file" as const,
      };
    });
    nodes.sort((left, right) => {
      if (left.kind !== right.kind) return left.kind === "dir" ? -1 : 1;
      return left.name.localeCompare(right.name, undefined, { sensitivity: "base" });
    });
    return nodes;
  };

  return toSorted(root);
}
