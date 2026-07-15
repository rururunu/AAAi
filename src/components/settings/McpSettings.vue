<template>
  <section class="mcp-settings">
    <AppConfirmDialog ref="confirmDialogRef" />

    <header>
      <div>
        <h2>{{ copy.title }}</h2>
        <p>{{ copy.description }}</p>
      </div>
      <Button
        v-if="tab === 'installed'"
        size="sm"
        class="h-8 gap-1.5"
        :disabled="Boolean(editor)"
        @click="startCreate"
      >
        <Plus class="size-3.5" />
        {{ copy.add }}
      </Button>
    </header>

    <div class="tabs" role="tablist">
      <button
        type="button"
        role="tab"
        class="tab"
        :class="{ active: tab === 'installed' }"
        :aria-selected="tab === 'installed'"
        @click="tab = 'installed'"
      >
        {{ copy.tabInstalled }}
      </button>
      <button
        type="button"
        role="tab"
        class="tab"
        :class="{ active: tab === 'catalog' }"
        :aria-selected="tab === 'catalog'"
        @click="openCatalog"
      >
        {{ copy.tabCatalog }}
      </button>
    </div>

    <p v-if="error" class="form-error">{{ error }}</p>

    <McpServerEditor
      v-if="editor"
      :editor="editor"
      :saving="saving"
      :copy="copy"
      :meta-labels="metaLabels"
      @update:editor="(value) => (editor = value)"
      @cancel="cancelEdit"
      @save="saveEditor"
    />

    <McpInstalledList
      v-if="tab === 'installed'"
      :servers="filtered"
      :disabled-actions="Boolean(editor)"
      :copy="copy"
      @toggle="toggleEnabled"
      @edit="startEdit"
      @remove="remove"
    />

    <McpCatalogPanel
      v-else
      v-model:query="catalogQuery"
      :loading="catalogLoading"
      :error="catalogError"
      :runtime-hint="runtimeHint"
      :show-curated="showCurated"
      :curated-entries="curatedEntries"
      :registry-entries="visibleRegistryEntries"
      :registry-meta="registryMeta"
      :next-cursor="catalogNextCursor"
      :saving="saving"
      :is-installed="isInstalled"
      :copy="copy"
      @search="runCatalogSearch"
      @install="addFromCatalog"
      @load-more="loadMoreCatalog"
    />
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Plus } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { invoke } from "@tauri-apps/api/core";
import {
  type CatalogEntry,
  type McpRuntimeSupport,
  filterCurated,
  filterInstallable,
  searchMcpRegistry,
} from "@/services/mcp/registry";
import { tr } from "@/services/i18n";
import type { McpI18nKey } from "@/services/locales/mcp";
import { useSettingStore } from "@/stores/setting";
import type { McpServerConfig } from "@/types/setting";
import McpCatalogPanel from "./mcp/McpCatalogPanel.vue";
import McpInstalledList from "./mcp/McpInstalledList.vue";
import McpServerEditor from "./mcp/McpServerEditor.vue";

const props = defineProps<{ query?: string }>();
const settingStore = useSettingStore();
const saving = ref(false);
const error = ref("");
const tab = ref<"installed" | "catalog">("installed");
const catalogQuery = ref("");
const catalogLoading = ref(false);
const catalogError = ref("");
const registryEntries = ref<CatalogEntry[]>([]);
const catalogNextCursor = ref<string | undefined>();
const catalogLoaded = ref(false);
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const runtimeSupport = ref<McpRuntimeSupport>({ npm: true, pypi: true });

type EditorState = {
  mode: "create" | "edit";
  id: string;
  title: string;
  description: string;
  command: string;
  argsText: string;
  envText: string;
  enabled: boolean;
};

const editor = ref<EditorState | null>(null);

const lang = computed(() => settingStore.language);
const t = (key: McpI18nKey, values?: Record<string, string | number>) =>
  tr(lang.value, key, values ?? {});

const metaLabels = computed(() => ({
  displayName: t("mcp.displayName"),
  displayNamePlaceholder: t("mcp.displayNamePlaceholder"),
  blurb: t("mcp.blurb"),
  blurbPlaceholder: t("mcp.blurbPlaceholder"),
}));

const copy = computed(() => ({
  title: t("mcp.title"),
  description: t("mcp.description"),
  add: t("mcp.add"),
  empty: t("mcp.empty"),
  id: t("mcp.id"),
  idPlaceholder: t("mcp.idPlaceholder"),
  command: t("mcp.command"),
  commandPlaceholder: t("mcp.commandPlaceholder"),
  args: t("mcp.args"),
  argsPlaceholder: t("mcp.argsPlaceholder"),
  env: t("mcp.env"),
  envPlaceholder: t("mcp.envPlaceholder"),
  envCount: (count: number) => t("mcp.envCount", { count }),
  enabled: t("mcp.enabled"),
  disabled: t("mcp.disabled"),
  edit: t("mcp.edit"),
  remove: t("mcp.remove"),
  cancel: t("mcp.cancel"),
  save: t("mcp.save"),
  deleteTitle: t("mcp.deleteTitle"),
  deleteDesc: (name: string) => t("mcp.deleteDesc", { name }),
  deleteConfirm: t("mcp.deleteConfirm"),
  idRequired: t("mcp.idRequired"),
  idExists: t("mcp.idExists"),
  commandRequired: t("mcp.commandRequired"),
  tabInstalled: t("mcp.tabInstalled"),
  tabCatalog: t("mcp.tabCatalog"),
  catalogSearch: t("mcp.catalogSearch"),
  search: t("mcp.search"),
  searching: t("mcp.searching"),
  catalogHint: t("mcp.catalogHint"),
  curatedTitle: t("mcp.curatedTitle"),
  curatedBadge: t("mcp.curatedBadge"),
  registryTitle: t("mcp.registryTitle"),
  catalogEmpty: t("mcp.catalogEmpty"),
  install: t("mcp.install"),
  added: t("mcp.added"),
  needsEnv: (names: string) => t("mcp.needsEnv", { names }),
  loadMore: t("mcp.loadMore"),
  resultCount: (count: number) => t("mcp.resultCount", { count }),
}));
const servers = computed(() => settingStore.mcpServers ?? []);
const curatedEntries = computed(() =>
  filterInstallable(
    filterCurated(catalogQuery.value || props.query || ""),
    runtimeSupport.value,
  ),
);
const visibleRegistryEntries = computed(() =>
  filterInstallable(registryEntries.value, runtimeSupport.value),
);
const showCurated = computed(() => curatedEntries.value.length > 0);
const registryMeta = computed(() => {
  if (!catalogLoaded.value || catalogLoading.value) return "";
  if (!visibleRegistryEntries.value.length) return "";
  return copy.value.resultCount(visibleRegistryEntries.value.length);
});
const runtimeHint = computed(() => {
  const support = runtimeSupport.value;
  if (support.npm && support.pypi) return "";
  const zh = settingStore.language.startsWith("zh");
  if (!support.npm && !support.pypi) {
    return zh
      ? "未检测到 Node/npx 或 uvx，目录暂无可安装项。请先安装 Node.js，或在「已安装」中手动添加。"
      : "No Node/npx or uvx detected — catalog is empty. Install Node.js, or add a server manually.";
  }
  if (!support.npm) {
    return zh
      ? "未检测到 Node/npx，已隐藏 npm 类 MCP。"
      : "Node/npx not found — npm MCP packages are hidden.";
  }
  return zh
    ? "未检测到 uvx，已隐藏 PyPI 类 MCP。"
    : "uvx not found — PyPI MCP packages are hidden.";
});

async function refreshRuntimeSupport() {
  try {
    runtimeSupport.value = await invoke<McpRuntimeSupport>("get_mcp_runtime_support");
  } catch {
    // Fail open so catalog still works if IPC unavailable during HMR.
    runtimeSupport.value = { npm: true, pypi: true };
  }
}

const filtered = computed(() => {
  const query = props.query?.trim().toLowerCase() ?? "";
  if (!query) return servers.value;
  return servers.value.filter((server) => {
    const haystack = [
      server.id,
      server.title ?? "",
      server.description ?? "",
      server.command,
      ...(server.args ?? []),
      ...(server.env ?? []).flatMap(([k, v]) => [k, v]),
    ]
      .join(" ")
      .toLowerCase();
    return haystack.includes(query);
  });
});

function parseArgs(text: string) {
  return text
    .trim()
    .split(/\s+/)
    .map((part) => part.trim())
    .filter(Boolean);
}

function parseEnv(text: string): Array<[string, string]> {
  const result: Array<[string, string]> = [];
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq <= 0) continue;
    result.push([trimmed.slice(0, eq).trim(), trimmed.slice(eq + 1)]);
  }
  return result;
}

function formatEnv(env?: Array<[string, string]>) {
  return (env ?? []).map(([k, v]) => `${k}=${v}`).join("\n");
}

function isInstalled(id: string) {
  return servers.value.some((server) => server.id === id);
}

function serverTitle(server: McpServerConfig) {
  return server.title?.trim() || server.id;
}

function startCreate() {
  error.value = "";
  editor.value = {
    mode: "create",
    id: "",
    title: "",
    description: "",
    command: "",
    argsText: "",
    envText: "",
    enabled: true,
  };
}

function startEdit(server: McpServerConfig) {
  error.value = "";
  editor.value = {
    mode: "edit",
    id: server.id,
    title: server.title ?? "",
    description: server.description ?? "",
    command: server.command,
    argsText: (server.args ?? []).join(" "),
    envText: formatEnv(server.env),
    enabled: server.enabled !== false,
  };
}

function cancelEdit() {
  editor.value = null;
  error.value = "";
}

async function persist(next: McpServerConfig[]) {
  saving.value = true;
  error.value = "";
  try {
    await settingStore.update({ mcpServers: next });
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    saving.value = false;
  }
}

async function saveEditor() {
  const draft = editor.value;
  if (!draft) return;
  const id = draft.id.trim();
  const command = draft.command.trim();
  if (!id) {
    error.value = copy.value.idRequired;
    return;
  }
  if (!command) {
    error.value = copy.value.commandRequired;
    return;
  }
  if (draft.mode === "create" && servers.value.some((server) => server.id === id)) {
    error.value = copy.value.idExists;
    return;
  }

  const title = draft.title.trim();
  const description = draft.description.trim();
  const nextServer: McpServerConfig = {
    id,
    ...(title ? { title } : {}),
    ...(description ? { description } : {}),
    command,
    args: parseArgs(draft.argsText),
    env: parseEnv(draft.envText),
    enabled: draft.enabled,
  };
  const next =
    draft.mode === "create"
      ? [...servers.value, nextServer]
      : servers.value.map((server) => (server.id === id ? nextServer : server));
  await persist(next);
  editor.value = null;
}

async function toggleEnabled(server: McpServerConfig) {
  const next = servers.value.map((item) =>
    item.id === server.id ? { ...item, enabled: item.enabled === false } : item,
  );
  await persist(next);
}

async function remove(server: McpServerConfig) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: copy.value.deleteTitle,
    description: copy.value.deleteDesc(serverTitle(server)),
    confirmLabel: copy.value.deleteConfirm,
    cancelLabel: copy.value.cancel,
  });
  if (!confirmed) return;
  await persist(servers.value.filter((item) => item.id !== server.id));
  if (editor.value?.id === server.id) editor.value = null;
}

function toPlainInstall(entry: CatalogEntry): McpServerConfig {
  const title = (entry.install.title ?? entry.title ?? "").trim();
  const description = (entry.install.description ?? entry.description ?? "").trim();
  return {
    id: String(entry.install.id ?? "").trim(),
    ...(title ? { title } : {}),
    ...(description ? { description } : {}),
    command: String(entry.install.command ?? "").trim(),
    args: [...(entry.install.args ?? [])].map(String),
    env: (entry.install.env ?? []).map(([k, v]) => [String(k), String(v)] as [string, string]),
    enabled: entry.install.enabled !== false,
  };
}

async function addFromCatalog(entry: CatalogEntry) {
  if (isInstalled(entry.install.id)) return;
  const install = toPlainInstall(entry);
  if (!install.id || !install.command) {
    error.value = copy.value.commandRequired;
    return;
  }
  const requiredEnv = entry.requiredEnv ?? [];
  if (requiredEnv.length) {
    const envLines = [
      ...(install.env ?? []).map(([k, v]) => `${k}=${v}`),
      ...requiredEnv.map((item) => `${item.name}=`),
    ].filter(Boolean);
    editor.value = {
      mode: "create",
      id: install.id,
      title: install.title ?? "",
      description: install.description ?? "",
      command: install.command,
      argsText: (install.args ?? []).join(" "),
      envText: envLines.join("\n"),
      enabled: true,
    };
    error.value = copy.value.needsEnv(requiredEnv.map((item) => item.name).join(", "));
    return;
  }
  try {
    await persist([...servers.value, install]);
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  }
}

async function runCatalogSearch() {
  catalogLoading.value = true;
  catalogError.value = "";
  catalogNextCursor.value = undefined;
  try {
    await refreshRuntimeSupport();
    const result = await searchMcpRegistry(catalogQuery.value || props.query || "", {
      desired: 60,
      maxPages: 12,
    });
    registryEntries.value = filterInstallable(result.entries, runtimeSupport.value);
    catalogNextCursor.value = result.nextCursor;
    catalogLoaded.value = true;
  } catch (err) {
    catalogError.value = err instanceof Error ? err.message : String(err);
    registryEntries.value = [];
    catalogNextCursor.value = undefined;
  } finally {
    catalogLoading.value = false;
  }
}

async function loadMoreCatalog() {
  if (!catalogNextCursor.value || catalogLoading.value) return;
  catalogLoading.value = true;
  catalogError.value = "";
  try {
    const result = await searchMcpRegistry(catalogQuery.value || props.query || "", {
      desired: 40,
      maxPages: 8,
      cursor: catalogNextCursor.value,
    });
    const seen = new Set(registryEntries.value.map((entry) => entry.name));
    for (const entry of filterInstallable(result.entries, runtimeSupport.value)) {
      if (seen.has(entry.name)) continue;
      seen.add(entry.name);
      registryEntries.value.push(entry);
    }
    catalogNextCursor.value = result.nextCursor;
  } catch (err) {
    catalogError.value = err instanceof Error ? err.message : String(err);
  } finally {
    catalogLoading.value = false;
  }
}

function openCatalog() {
  tab.value = "catalog";
  if (!catalogLoaded.value) void runCatalogSearch();
}

watch(
  () => props.query,
  (value) => {
    if (tab.value === "catalog" && value != null) {
      catalogQuery.value = value;
      void runCatalogSearch();
    }
  },
);

onMounted(() => {
  if (props.query?.trim()) catalogQuery.value = props.query;
  void refreshRuntimeSupport();
});
</script>

<style scoped>
.mcp-settings {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 12px 16px 20px;
}

header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

header h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

header p {
  margin: 4px 0 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
  max-width: 52ch;
}

.tabs {
  display: flex;
  gap: 4px;
  padding: 3px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--muted-foreground) 10%, transparent);
  width: fit-content;
}

.tab {
  border: 0;
  background: transparent;
  color: var(--muted-foreground);
  font-size: 12px;
  padding: 6px 12px;
  border-radius: 8px;
  cursor: pointer;
}

.tab.active {
  background: var(--background);
  color: var(--foreground);
}

.form-error {
  margin: 0;
  color: #ef4444;
  font-size: 12px;
  line-height: 1.5;
}
</style>
