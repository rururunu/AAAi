<template>
  <div class="flex h-full flex-col bg-background text-foreground">
    <header class="titlebar flex h-9 shrink-0 items-center justify-between border-b border-border bg-sidebar">
      <div 
        class="titlebar-drag flex h-full flex-1 items-center gap-2 pl-3" 
        data-tauri-drag-region
        @mousedown="onWindowDragMouseDown"
      >
        <Settings2 class="size-3.5 text-primary" />
        <span class="text-xs font-semibold" data-tauri-drag-region>{{ t.title }}</span>
      </div>

      <div class="flex h-full">
        <button
          type="button"
          class="titlebar-btn"
          :aria-label="t.minimize"
          @mousedown.prevent="minimize"
        >
          <Minus class="size-3.5" />
        </button>
        <button
          type="button"
          class="titlebar-btn close"
          :aria-label="t.close"
          @mousedown.prevent="close"
        >
          <X class="size-3.5" />
        </button>
      </div>
    </header>

    <div class="flex min-h-0 flex-1 overflow-hidden">
    <SidebarProvider class="h-full min-h-0 w-full [&_[data-slot=sidebar-wrapper]]:h-full [&_[data-slot=sidebar-wrapper]]:min-h-0">
      <Sidebar collapsible="none" class="settings-nav border-r">
        <SidebarContent>
          <SidebarGroup>
            <SidebarGroupLabel>{{ t.sidebarLabel }}</SidebarGroupLabel>
            <SidebarMenu>
              <SidebarMenuItem v-for="category in categories" :key="category.id">
                <SidebarMenuButton
                  class="settings-nav-item"
                  :is-active="activeCategory === category.id"
                  @click="activeCategory = category.id"
                >
                  <component :is="category.icon" class="size-4 shrink-0" />
                  <span class="settings-nav-label">{{ category.label }}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroup>
        </SidebarContent>
      </Sidebar>

      <SidebarInset class="flex min-h-0 flex-col">
        <div class="border-b border-border p-3">
          <div class="relative">
            <Search class="text-muted-foreground pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2" />
            <Input
              ref="searchRef"
              v-model="searchQuery"
              class="h-8 pl-8"
              :placeholder="searchPlaceholder"
            />
          </div>
        </div>

        <div class="flex-1 overflow-y-auto pr-1 peek-scrollbar">
          <Transition
            :css="false"
            mode="out-in"
            @enter="gsapSettingsPanelEnter"
            @leave="gsapSettingsPanelLeave"
          >
          <div :key="activeCategory" class="settings-panel p-1">
            <WorkspaceSettings
              v-if="activeCategory === 'workspace'"
              :query="searchQuery"
            />
            <McpSettings
              v-else-if="activeCategory === 'mcp'"
              :query="searchQuery"
            />
            <SkillsSettings
              v-else-if="activeCategory === 'skills'"
              :query="searchQuery"
            />
            <ProviderSettings
              v-else-if="activeCategory === 'provider'"
              :query="searchQuery"
            />
            <HistorySettings
              v-else-if="activeCategory === 'history'"
              :query="searchQuery"
              :expanded-history-groups="expandedHistoryGroups"
              @toggle-history-group="toggleHistoryGroup"
            />
            <SettingFieldList
              v-else
              :items="visibleItems"
              :empty-text="t.empty"
              v-model:api-key-draft="apiKeyDraft"
              v-model:mem0-api-key-draft="mem0ApiKeyDraft"
              v-model:mem0-user-id-draft="mem0UserIdDraft"
              v-model:mem0-base-url-draft="mem0BaseUrlDraft"
              v-model:serper-api-key-draft="serperApiKeyDraft"
              v-model:tavily-api-key-draft="tavilyApiKeyDraft"
              @toggle="onToggle"
              @slider-change="onSliderChange"
              @color-scheme-change="onColorSchemeChange"
              @language-change="onLanguageChange"
              @zoom-change="onZoomChange"
              @reasoning-effort-change="onReasoningEffortChange"
              @reasoning-language-change="onReasoningLanguageChange"
              @tool-approval-mode-change="onToolApprovalModeChange"
              @web-search-provider-change="onWebSearchProviderChange"
              @default-model-change="onDefaultModelChange"
              @multimodal-model-change="onMultimodalModelChange"
              @custom-accent-change="onCustomAccentChange"
              @reset-custom-accent="resetCustomAccent"
              @save-api-key="saveApiKey"
              @save-memory-settings="saveMemorySettings"
              @save-web-search-settings="saveWebSearchSettings"
            />
          </div>
          </Transition>
        </div>
      </SidebarInset>
    </SidebarProvider>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Bot, BrainCircuit, Plug, Shield, Folders, Globe2, History, Info, Minus, Palette, Search, Server, Settings2, Sparkles, X } from "@lucide/vue";
import WorkspaceSettings from "@/components/workspace/WorkspaceSettings.vue";
import McpSettings from "@/components/settings/McpSettings.vue";
import SkillsSettings from "@/components/settings/SkillsSettings.vue";
import HistorySettings from "@/components/settings/HistorySettings.vue";
import ProviderSettings from "@/components/settings/ProviderSettings.vue";
import SettingFieldList from "@/components/settings/SettingFieldList.vue";
import { onWindowDragMouseDown } from "@/services/overlay/windowDrag";
import {
  gsapSettingsNavMount,
  gsapSettingsPanelEnter,
  gsapSettingsPanelLeave,
} from "@/services/motion/gsapPresets";
import { getAppInfo } from "@/services/ipc";
import { Input } from "@/components/ui/input";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
} from "@/components/ui/sidebar";
import { useSettingStore } from "@/stores/setting";
import { useChatModelStore } from "@/stores/chatModel";
import { tr } from "@/services/i18n";
import { buildSettingDefinitions, type CategoryId, type SettingDefinition } from "@/pages/Settings/settingsDefinitions";
import type {
  AppLanguage,
  ColorScheme,
  ReasoningEffort,
  ReasoningLanguage,
  WebSearchProvider,
  ToolApprovalMode,
} from "@/types/setting";
import { DEFAULT_ACCENT_COLOR, normalizeAccentColor } from "@/types/setting";

const settingStore = useSettingStore();
const chatModelStore = useChatModelStore();
const appWindow = getCurrentWebviewWindow();

const SETTINGS_BASE_WIDTH = 700;
const SETTINGS_BASE_HEIGHT = 520;

async function resizeSettingsWindow() {
  const zoom = (settingStore.zoom || 100) / 100;
  const scaledWidth = SETTINGS_BASE_WIDTH * zoom;
  const scaledHeight = SETTINGS_BASE_HEIGHT * zoom;
  await appWindow.setSize(new LogicalSize(scaledWidth, scaledHeight));
}

const searchRef = ref<InstanceType<typeof Input> | null>(null);
const searchQuery = ref("");
const activeCategory = ref<CategoryId>("ai");

const appName = ref("-");
const appVersion = ref("-");
const appIdentifier = ref("-");
const apiKeyDraft = ref("");
const mem0ApiKeyDraft = ref("");
const mem0UserIdDraft = ref("");
const mem0BaseUrlDraft = ref("");
const serperApiKeyDraft = ref("");
const tavilyApiKeyDraft = ref("");
const expandedHistoryGroups = ref<Record<string, boolean>>({});

function isHistoryGroupExpanded(groupId: string) {
  return expandedHistoryGroups.value[groupId] !== false;
}

function toggleHistoryGroup(groupId: string) {
  expandedHistoryGroups.value = {
    ...expandedHistoryGroups.value,
    [groupId]: !isHistoryGroupExpanded(groupId),
  };
}

const searchPlaceholder = computed(() =>
  activeCategory.value === "history"
    ? tr(settingStore.language, "settings.history.search")
    : tr(settingStore.language, "settings.searchPlaceholder"),
);

const t = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "settings.title"),
    minimize: tr(language, "settings.minimize"),
    close: tr(language, "settings.close"),
    sidebarLabel: tr(language, "settings.sidebarLabel"),
    empty: tr(language, "settings.empty"),
    categories: {
      appearance: tr(language, "settings.categories.appearance"),
      ai: tr(language, "settings.categories.ai"),
      memory: tr(language, "settings.categories.memory"),
      search: tr(language, "settings.categories.search"),
      agent: tr(language, "settings.categories.agent"),
      mcp: tr(language, "settings.categories.mcp"),
      skills: tr(language, "settings.categories.skills"),
      workspace: tr(language, "settings.categories.workspace"),
      history: tr(language, "settings.categories.history"),
      about: tr(language, "settings.categories.about"),
      provider: tr(language, "settings.categories.provider"),
    },
  };
});

const categories = computed(() => [
  { id: "ai" as const, label: t.value.categories.ai, icon: Bot },
  { id: "provider" as const, label: t.value.categories.provider, icon: Server },
  { id: "workspace" as const, label: t.value.categories.workspace, icon: Folders },
  { id: "agent" as const, label: t.value.categories.agent, icon: Shield },
  { id: "history" as const, label: t.value.categories.history, icon: History },
  { id: "mcp" as const, label: t.value.categories.mcp, icon: Plug },
  { id: "skills" as const, label: t.value.categories.skills, icon: Sparkles },
  { id: "memory" as const, label: t.value.categories.memory, icon: BrainCircuit },
  { id: "search" as const, label: t.value.categories.search, icon: Globe2 },
  { id: "appearance" as const, label: t.value.categories.appearance, icon: Palette },
  { id: "about" as const, label: t.value.categories.about, icon: Info },
]);

const settingDefinitions = computed<SettingDefinition[]>(() =>
  buildSettingDefinitions(settingStore.language, {
    appName: appName.value,
    appVersion: appVersion.value,
    appIdentifier: appIdentifier.value,
  }),
);

const normalizedQuery = computed(() => searchQuery.value.trim().toLowerCase());

const visibleItems = computed(() =>
  settingDefinitions.value.filter((item) => {
    if (item.category !== activeCategory.value) {
      return false;
    }

    if (!normalizedQuery.value) {
      return true;
    }

    const haystack = [
      item.title,
      item.description,
      item.path,
      item.group,
      ...item.keywords,
    ]
      .join(" ")
      .toLowerCase();

    return haystack.includes(normalizedQuery.value);
  }),
);

function minimize() {
  void appWindow.minimize();
}

function close() {
  void appWindow.hide();
}

function onColorSchemeChange(value: unknown) {
  if (typeof value !== "string") {
    return;
  }
  void settingStore.update({ colorScheme: value as ColorScheme });
}

function onLanguageChange(value: unknown) {
  if (typeof value !== "string") {
    return;
  }
  void settingStore.update({ language: value as AppLanguage });
}

function onZoomChange(value: unknown) {
  if (typeof value !== "string") {
    return;
  }
  const zoomVal = parseFloat(value);
  if (isNaN(zoomVal)) {
    return;
  }
  void settingStore.update({ zoom: zoomVal });
}

function onReasoningEffortChange(value: unknown) {
  if (typeof value !== "string") {
    return;
  }
  void settingStore.update({ reasoningEffort: value as ReasoningEffort });
}

function onReasoningLanguageChange(value: unknown) {
  if (typeof value !== "string") {
    return;
  }
  void settingStore.update({ reasoningLanguage: value as ReasoningLanguage });
}

function onSliderChange(id: string, value: number) {
  if (id === "opacity") {
    void settingStore.update({ opacity: value });
  }
}

async function saveApiKey() {
  if (apiKeyDraft.value === settingStore.deepseekApiKey) {
    return;
  }
  await settingStore.update({ deepseekApiKey: apiKeyDraft.value.trim() });
  await chatModelStore.refresh();
}

function onCustomAccentChange(value: string) {
  void settingStore.update({ customAccentColor: normalizeAccentColor(value) });
}

function resetCustomAccent() {
  void settingStore.update({ customAccentColor: DEFAULT_ACCENT_COLOR });
}

function onDefaultModelChange(value: unknown) {
  if (typeof value !== "string" || !value.trim()) return;
  void settingStore.update({ chatModel: value });
}

function onMultimodalModelChange(value: unknown) {
  if (typeof value !== "string" || !value.trim()) return;
  void settingStore.update({ multimodalModel: value });
}

function onToggle(id: string) {
  if (id === "memoryEnabled") {
    void settingStore.update({ memoryEnabled: !settingStore.memoryEnabled });
  }
  if (id === "webSearchEnabled") {
    void settingStore.update({
      webSearchEnabled: !settingStore.webSearchEnabled,
      serperApiKey: serperApiKeyDraft.value.trim(),
      tavilyApiKey: tavilyApiKeyDraft.value.trim(),
    });
  }
  if (id === "lspEnabled") {
    void settingStore.update({ lspEnabled: !settingStore.lspEnabled });
  }
  if (id === "passToolReasoning") {
    void settingStore.update({ passToolReasoning: !settingStore.passToolReasoning });
  }
  if (id === "multimodalSplitAnalysis") {
    void settingStore.update({ multimodalSplitAnalysis: !settingStore.multimodalSplitAnalysis });
  }
  if (id === "largeContextEnabled") {
    void settingStore.update({ largeContextEnabled: !settingStore.largeContextEnabled });
  }
}

function onToolApprovalModeChange(value: unknown) {
  if (value !== "ask" && value !== "auto" && value !== "alwaysAllow") return;
  void settingStore.update({ toolApprovalMode: value as ToolApprovalMode });
}

function saveMemorySettings() {
  void settingStore.update({
    mem0ApiKey: mem0ApiKeyDraft.value.trim(),
    mem0UserId: mem0UserIdDraft.value.trim() || "peek-user",
    mem0BaseUrl: mem0BaseUrlDraft.value.trim() || "https://api.mem0.ai/v1",
  });
}

function saveWebSearchSettings() {
  void settingStore.update({
    serperApiKey: serperApiKeyDraft.value.trim(),
    tavilyApiKey: tavilyApiKeyDraft.value.trim(),
  });
}

function onWebSearchProviderChange(value: unknown) {
  if (value !== "serper" && value !== "tavily") return;
  void settingStore.update({
    webSearchProvider: value as WebSearchProvider,
    serperApiKey: serperApiKeyDraft.value.trim(),
    tavilyApiKey: tavilyApiKeyDraft.value.trim(),
  });
}

watch(activeCategory, () => {
  searchQuery.value = "";
});

onMounted(async () => {
  apiKeyDraft.value = settingStore.deepseekApiKey;
  mem0ApiKeyDraft.value = settingStore.mem0ApiKey;
  mem0UserIdDraft.value = settingStore.mem0UserId;
  mem0BaseUrlDraft.value = settingStore.mem0BaseUrl;
  serperApiKeyDraft.value = settingStore.serperApiKey;
  tavilyApiKeyDraft.value = settingStore.tavilyApiKey;
  void chatModelStore.fetch();

  const info = await getAppInfo();
  appName.value = info.name;
  appVersion.value = info.version;
  appIdentifier.value = info.identifier;

  await resizeSettingsWindow();

  await nextTick();
  searchRef.value?.$el?.focus?.();
  const navEl = document.querySelector(".settings-nav");
  if (navEl) gsapSettingsNavMount(navEl);
});

watch(
  () => settingStore.zoom,
  async () => {
    await resizeSettingsWindow();
  }
);
</script>

<style scoped>
.settings-nav {
  width: 8.75rem;
}

.settings-nav :deep([data-slot="sidebar-menu-button"]),
.settings-nav-item {
  gap: 0.55rem;
  font-size: 13px;
  letter-spacing: 0.02em;
}

.settings-nav-label {
  display: inline-block;
  min-width: 2.75em;
  font-variant-numeric: tabular-nums;
}

.settings-panel {
  min-height: 100%;
  will-change: opacity, transform;
}

.titlebar-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 46px;
  height: 100%;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: default;
}

.titlebar-btn:hover {
  background: var(--sidebar-accent);
}

.titlebar-btn.close:hover {
  background: #e81123;
  color: #fff;
}
</style>
