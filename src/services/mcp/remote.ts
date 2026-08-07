/**
 * Shared helpers for mcp-remote (Smithery / hosted HTTP MCP via stdio bridge).
 *
 * Keep `MCP_REMOTE_PINNED_VERSION` in sync with
 * `src-tauri/src/core/mcp/remote_auth.rs`.
 */

import type { McpServerConfig } from "@/types/setting";

/** Pinned npm version so OAuth token dirs stay stable across launches. */
export const MCP_REMOTE_PINNED_VERSION = "0.1.38";

export const MCP_REMOTE_PACKAGE = `mcp-remote@${MCP_REMOTE_PINNED_VERSION}`;

export type McpServerRuntimeState =
  "disabled" | "connected" | "authenticated" | "needs_auth" | "local";

export type McpServerRuntimeStatus = {
  id: string;
  enabled: boolean;
  usesRemoteAuth: boolean;
  connected: boolean;
  hasSavedCredentials: boolean;
  state: McpServerRuntimeState | string;
};

export type McpConnectResult = {
  serverId: string;
  toolCount: number;
  status: McpServerRuntimeStatus;
};

/** True when args launch the mcp-remote OAuth bridge. */
export function isMcpRemoteServer(server: Pick<McpServerConfig, "args">): boolean {
  return (server.args ?? []).some((arg) => {
    const base = arg.split("@")[0] ?? arg;
    return base.toLowerCase() === "mcp-remote";
  });
}

/** Hosted HTTP URL passed to mcp-remote (first http(s) arg after the package). */
export function mcpRemoteServerUrl(server: Pick<McpServerConfig, "args">): string | undefined {
  const args = server.args ?? [];
  const idx = args.findIndex((arg) => {
    const base = arg.split("@")[0] ?? arg;
    return base.toLowerCase() === "mcp-remote";
  });
  if (idx < 0) return undefined;
  return args.slice(idx + 1).find((arg) => /^https?:\/\//i.test(arg));
}

/** Strip a query-string `api_key` param, e.g. before sending a URL to a management API. */
export function withoutApiKeyParam(url: string): string {
  try {
    const parsed = new URL(url);
    parsed.searchParams.delete("api_key");
    return parsed.toString();
  } catch {
    return url.replace(/([?&])api_key=[^&]*&?/i, "$1").replace(/[?&]$/, "");
  }
}

/** Ensure install args use the pinned package (idempotent). */
export function withPinnedMcpRemote(args: string[]): string[] {
  const next = [...args];
  const idx = next.findIndex((arg) => {
    const base = arg.split("@")[0] ?? arg;
    return base.toLowerCase() === "mcp-remote";
  });
  if (idx < 0) return next;
  next[idx] = MCP_REMOTE_PACKAGE;
  return next;
}

/** Sanitize a registry identity into a filesystem / tool-safe install id. */
export function sanitizeMcpInstallId(raw: string): string {
  return (
    raw
      .trim()
      .replace(/[^A-Za-z0-9_.-]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 64)
      .toLowerCase() || "mcp"
  );
}

/**
 * Stable install id from Smithery identity.
 * Prefer qualifiedName (e.g. `gmail` / `org/slug`) — no synthetic `sm-` prefix.
 */
export function mcpInstallId(server: {
  id?: string;
  qualifiedName?: string;
  slug?: string;
  displayName?: string;
}): string {
  const qn = server.qualifiedName?.trim();
  if (qn) return sanitizeMcpInstallId(qn);
  const registryId = server.id?.trim();
  if (registryId && registryId.includes("/")) {
    return sanitizeMcpInstallId(registryId);
  }
  if (registryId && registryId.length >= 8) {
    return sanitizeMcpInstallId(`smid-${registryId}`);
  }
  const slug = server.slug?.trim();
  if (slug) return sanitizeMcpInstallId(slug);
  return sanitizeMcpInstallId(server.displayName || "mcp");
}

/** Legacy install ids used `sm-${qualifiedName}` — rewrite when safe. */
export function migrateLegacySmitheryInstallId(
  server: Pick<McpServerConfig, "id" | "qualifiedName">,
  occupiedIds: ReadonlySet<string>,
): string | null {
  const qn = server.qualifiedName?.trim();
  if (!qn) return null;
  const preferred = sanitizeMcpInstallId(qn);
  if (!preferred || server.id === preferred) return null;
  const legacy = sanitizeMcpInstallId(`sm-${qn}`);
  if (server.id !== legacy) return null;
  if (occupiedIds.has(preferred)) return null;
  return preferred;
}

/** Match an installed server to a catalog/Smithery candidate by identity, not title. */
export function isSameMcpInstall(
  installed: Pick<McpServerConfig, "id" | "qualifiedName" | "registryId">,
  candidate: {
    id?: string;
    qualifiedName?: string;
    installId?: string;
  },
): boolean {
  const qn = candidate.qualifiedName?.trim();
  if (qn && installed.qualifiedName?.trim() === qn) return true;
  const rid = candidate.id?.trim();
  if (rid && installed.registryId?.trim() === rid) return true;
  const installId = candidate.installId?.trim() || (qn || rid ? mcpInstallId(candidate) : "");
  if (installId && installed.id === installId) return true;
  return false;
}
