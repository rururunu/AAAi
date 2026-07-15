<template>
  <AppConfirmDialog ref="confirmDialogRef" />
  <div class="py-2 select-none">
    <!-- Group Title -->
    <div class="flex items-center justify-between px-4 py-2 select-none">
      <h2 class="text-muted-foreground text-[11px] font-semibold tracking-wider uppercase">
        {{ historyText.title }}
      </h2>

      <div class="flex items-center gap-2">
        <Button
          variant="ghost"
          size="sm"
          class="h-7 text-xs text-muted-foreground hover:text-foreground flex items-center gap-1.5"
          @click="toggleSelectAll"
        >
          <input
            type="checkbox"
            :checked="isAllSelected"
            class="appearance-none size-3.5 rounded border border-input bg-background checked:bg-primary checked:border-primary cursor-pointer pointer-events-none transition-all relative flex items-center justify-center after:content-[''] after:hidden checked:after:block after:w-1.5 after:h-1 after:border-l-2 after:border-b-2 after:border-primary-foreground after:rotate-[-45deg] after:translate-y-[-0.5px]"
          />
          <span>{{ historyText.selectAll }}</span>
        </Button>

        <Button
          v-if="selectedSessionIds.length > 0"
          variant="destructive"
          size="sm"
          class="h-7 text-xs flex items-center gap-1.5"
          @click="deleteSelectedSessions"
        >
          <Trash2 class="size-3" />
          <span>{{ historyText.deleteSelected.replace("{count}", String(selectedSessionIds.length)) }}</span>
        </Button>

        <Button
          variant="ghost"
          size="sm"
          class="h-7 text-xs text-destructive hover:bg-destructive/10 hover:text-destructive flex items-center gap-1.5"
          @click="clearAllSessions"
        >
          <AlertTriangle class="size-3" />
          <span>{{ historyText.clearAll }}</span>
        </Button>
      </div>
    </div>

    <div class="border-t border-border">
      <div v-if="historyGroups.length === 0" class="text-muted-foreground px-4 py-8 text-center text-sm">
        {{ historyText.empty }}
      </div>

      <section v-for="group in historyGroups" :key="group.id" class="border-b border-border">
        <div class="flex items-center gap-1 px-3 py-1.5 hover:bg-muted/30">
          <button
            type="button"
            class="flex min-w-0 flex-1 items-center gap-2 py-1 text-left"
            @click="toggleHistoryGroup(group.id)"
          >
            <ChevronDown v-if="isHistoryGroupExpanded(group.id)" class="size-3.5 text-muted-foreground" />
            <ChevronRight v-else class="size-3.5 text-muted-foreground" />
            <Globe2 v-if="group.public" class="size-4 text-muted-foreground" />
            <Folder v-else class="size-4 text-primary" />
            <span class="min-w-0 flex-1">
              <strong class="block truncate text-xs font-semibold">{{ group.name }}</strong>
              <small v-if="group.root" class="block truncate text-[10px] text-muted-foreground">{{ group.root }}</small>
            </span>
          </button>
          <span class="text-[11px] tabular-nums text-muted-foreground">{{ group.sessions.length }}</span>
          <Button
            variant="ghost"
            size="sm"
            class="h-7 gap-1.5 px-2 text-xs text-muted-foreground hover:text-foreground"
            :disabled="group.sessions.length === 0"
            @click="toggleHistoryGroupSelection(group)"
          >
            <input
              type="checkbox"
              :checked="isHistoryGroupSelected(group)"
              :indeterminate="isHistoryGroupPartiallySelected(group)"
              class="pointer-events-none relative flex size-3.5 cursor-pointer appearance-none items-center justify-center rounded border border-input bg-background transition-all checked:border-primary checked:bg-primary after:hidden after:h-1 after:w-1.5 after:translate-y-[-0.5px] after:rotate-[-45deg] after:border-b-2 after:border-l-2 after:border-primary-foreground after:content-[''] checked:after:block"
            />
            {{ historyText.selectAll }}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="size-7 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
            :disabled="group.sessions.length === 0"
            :title="historyText.deleteGroup"
            :aria-label="historyText.deleteGroup"
            @click="deleteHistoryGroup(group)"
          >
            <Trash2 class="size-3.5" />
          </Button>
        </div>

        <article
          v-for="session in isHistoryGroupExpanded(group.id) ? group.sessions : []"
          :key="session.sessionId"
          class="grid grid-cols-[minmax(0,1fr)_150px] items-center gap-4 border-t border-border/70 py-3 pr-4 pl-10 hover:bg-muted/20"
        >
          <div class="flex min-w-0 items-start gap-3">
            <input
              v-model="selectedSessionIds"
              type="checkbox"
              :value="session.sessionId"
              class="appearance-none size-4 rounded border border-input bg-background checked:bg-primary checked:border-primary cursor-pointer mt-1 transition-all relative flex items-center justify-center after:content-[''] after:hidden checked:after:block after:w-2 after:h-1 after:border-l-2 after:border-b-2 after:border-primary-foreground after:rotate-[-45deg] after:translate-y-[-1px]"
            />
            <div class="min-w-0 flex-1 space-y-1">
              <p class="text-[11px] text-muted-foreground">{{ formatTime(session.updatedAt) }}</p>
              <h3 class="truncate text-sm font-medium cursor-pointer hover:text-primary" @click="openSession(session)">
                {{ session.preview }}
              </h3>
              <p class="text-xs text-muted-foreground">
                {{ historyText.messages.replace("{count}", String(session.messageCount)) }}
              </p>
            </div>
          </div>
          <div class="flex justify-end gap-2">
            <Button variant="outline" size="sm" class="h-8 gap-1.5 text-xs" @click="openSession(session)">
              <FolderOpen class="size-3.5" />
              {{ historyText.open }}
            </Button>
            <Button variant="ghost" size="icon" class="size-8 text-muted-foreground hover:text-destructive" @click="deleteSingleSession(session.sessionId)">
              <Trash2 class="size-3.5" />
            </Button>
          </div>
        </article>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { emit as tauriEmit } from "@tauri-apps/api/event";
import { AlertTriangle, ChevronDown, ChevronRight, Folder, FolderOpen, Globe2, Trash2 } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { listWorkspaces, switchWorkspace, type Workspace } from "@/commands/workspace";
import { listChatSessions, deleteChatSession, clearAllChatSessions, openSessionInOverlay } from "@/services/ipc";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import type { SettingsI18nKey } from "@/services/locales/settings";
import type { ChatSessionSummary } from "@/types/chat";

const props = defineProps<{
  query?: string;
  expandedHistoryGroups: Record<string, boolean>;
}>();

const emit = defineEmits<{
  "toggle-history-group": [groupId: string];
}>();

const settingStore = useSettingStore();

const historyText = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "settings.history.title"),
    selectAll: tr(language, "settings.history.selectAll"),
    deleteSelected: tr(language, "settings.history.deleteSelected"),
    clearAll: tr(language, "settings.history.clearAll"),
    empty: tr(language, "settings.history.empty"),
    deleteGroup: tr(language, "settings.history.deleteGroup"),
    messages: tr(language, "settings.history.messages"),
    open: tr(language, "settings.history.open"),
    publicGroup: tr(language, "settings.history.publicGroup"),
    yesterday: tr(language, "settings.history.yesterday"),
    cancel: tr(language, "settings.history.cancel"),
    deleteLabel: tr(language, "settings.history.deleteLabel"),
  };
});

function historyConfirm(key: Extract<SettingsI18nKey, `settings.historyConfirm.${string}`>, values: Record<string, string | number> = {}) {
  return tr(settingStore.language, key, values);
}

const PUBLIC_HISTORY_GROUP = "__public__";
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const historySessions = ref<ChatSessionSummary[]>([]);
const historyWorkspaces = ref<Workspace[]>([]);
const selectedSessionIds = ref<string[]>([]);

interface HistoryGroup {
  id: string;
  name: string;
  root?: string;
  public: boolean;
  sessions: ChatSessionSummary[];
}

function workspaceNameFromId(id: string) {
  const parts = id.replace(/\\/g, "/").split("/").filter(Boolean);
  return parts[parts.length - 1] ?? id;
}

const historyGroups = computed<HistoryGroup[]>(() => {
  const query = props.query?.trim().toLowerCase() ?? "";
  const groups = new Map<string, HistoryGroup>();
  for (const workspace of historyWorkspaces.value) {
    groups.set(workspace.id, {
      id: workspace.id,
      name: workspace.name,
      root: workspace.root,
      public: false,
      sessions: [],
    });
  }
  groups.set(PUBLIC_HISTORY_GROUP, {
    id: PUBLIC_HISTORY_GROUP,
    name: historyText.value.publicGroup,
    public: true,
    sessions: [],
  });

  for (const session of historySessions.value) {
    const groupId = session.workspaceId ?? PUBLIC_HISTORY_GROUP;
    if (!groups.has(groupId)) {
      groups.set(groupId, {
        id: groupId,
        name: workspaceNameFromId(groupId),
        root: groupId,
        public: false,
        sessions: [],
      });
    }
    groups.get(groupId)?.sessions.push(session);
  }

  return [...groups.values()]
    .map((group) => {
      const groupMatches = query && `${group.name} ${group.root ?? ""}`.toLowerCase().includes(query);
      return {
        ...group,
        sessions: !query || groupMatches
          ? group.sessions
          : group.sessions.filter((session) => session.preview.toLowerCase().includes(query)),
      };
    })
    .filter((group) => !query || group.sessions.length > 0)
    .sort((left, right) => Number(left.public) - Number(right.public));
});

const filteredHistorySessions = computed(() =>
  historyGroups.value.flatMap((group) => group.sessions),
);

function isHistoryGroupExpanded(groupId: string) {
  return props.expandedHistoryGroups[groupId] !== false;
}

function toggleHistoryGroup(groupId: string) {
  emit("toggle-history-group", groupId);
}

function historyGroupSessionIds(group: HistoryGroup) {
  return historySessions.value
    .filter((session) => (session.workspaceId ?? PUBLIC_HISTORY_GROUP) === group.id)
    .map((session) => session.sessionId);
}

function isHistoryGroupSelected(group: HistoryGroup) {
  const groupIds = historyGroupSessionIds(group);
  return groupIds.length > 0 && groupIds.every((id) => selectedSessionIds.value.includes(id));
}

function isHistoryGroupPartiallySelected(group: HistoryGroup) {
  const selectedCount = historyGroupSessionIds(group)
    .filter((id) => selectedSessionIds.value.includes(id)).length;
  return selectedCount > 0 && selectedCount < historyGroupSessionIds(group).length;
}

function toggleHistoryGroupSelection(group: HistoryGroup) {
  const groupIds = historyGroupSessionIds(group);
  if (isHistoryGroupSelected(group)) {
    const ids = new Set(groupIds);
    selectedSessionIds.value = selectedSessionIds.value.filter((id) => !ids.has(id));
    return;
  }
  selectedSessionIds.value = [...new Set([...selectedSessionIds.value, ...groupIds])];
}

const isAllSelected = computed(() => {
  return filteredHistorySessions.value.length > 0 &&
    filteredHistorySessions.value.every((s) => selectedSessionIds.value.includes(s.sessionId));
});

function toggleSelectAll() {
  if (isAllSelected.value) {
    selectedSessionIds.value = [];
  } else {
    selectedSessionIds.value = filteredHistorySessions.value.map((s) => s.sessionId);
  }
}

async function loadSessions() {
  const [sessionsList, workspaces] = await Promise.all([
    listChatSessions(),
    listWorkspaces(),
  ]);
  historySessions.value = sessionsList.sessions ?? [];
  historyWorkspaces.value = workspaces;
}

async function deleteSelectedSessions() {
  if (selectedSessionIds.value.length === 0) return;
  const confirmed = await confirmDialogRef.value?.ask({
    title: historyConfirm("settings.historyConfirm.deleteTitle"),
    description: historyConfirm("settings.historyConfirm.deleteSelectedDesc", { count: selectedSessionIds.value.length }),
    confirmLabel: historyText.value.deleteLabel,
    cancelLabel: historyText.value.cancel,
  });
  if (!confirmed) return;
  try {
    await Promise.all(selectedSessionIds.value.map((id) => deleteChatSession(id)));
    selectedSessionIds.value = [];
    await loadSessions();
    await tauriEmit("history-updated");
  } catch (error) {
    console.error("Failed to delete sessions:", error);
  }
}

async function deleteHistoryGroup(group: HistoryGroup) {
  const sessionIds = historyGroupSessionIds(group);
  if (sessionIds.length === 0) return;
  const confirmed = await confirmDialogRef.value?.ask({
    title: historyConfirm("settings.historyConfirm.deleteGroupTitle"),
    description: historyConfirm("settings.historyConfirm.deleteGroupDesc", { name: group.name, count: sessionIds.length }),
    confirmLabel: historyConfirm("settings.historyConfirm.deleteAllLabel"),
    cancelLabel: historyText.value.cancel,
  });
  if (!confirmed) return;
  try {
    await Promise.all(sessionIds.map((id) => deleteChatSession(id)));
    const deletedIds = new Set(sessionIds);
    selectedSessionIds.value = selectedSessionIds.value.filter((id) => !deletedIds.has(id));
    await loadSessions();
    await tauriEmit("history-updated");
  } catch (error) {
    console.error("Failed to delete history group:", error);
  }
}

async function deleteSingleSession(sessionId: string) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: historyConfirm("settings.historyConfirm.deleteTitle"),
    description: historyConfirm("settings.historyConfirm.deleteSingleDesc"),
    confirmLabel: historyText.value.deleteLabel,
    cancelLabel: historyText.value.cancel,
  });
  if (!confirmed) return;
  try {
    await deleteChatSession(sessionId);
    selectedSessionIds.value = selectedSessionIds.value.filter((id) => id !== sessionId);
    await loadSessions();
    await tauriEmit("history-updated");
  } catch (error) {
    console.error("Failed to delete session:", error);
  }
}

async function clearAllSessions() {
  const confirmMsg = historyConfirm("settings.historyConfirm.clearDesc");
  const confirmed = await confirmDialogRef.value?.ask({
    title: historyConfirm("settings.historyConfirm.clearTitle"),
    description: confirmMsg,
    confirmLabel: historyText.value.clearAll,
    cancelLabel: historyText.value.cancel,
  });
  if (!confirmed) return;
  try {
    await clearAllChatSessions();
    selectedSessionIds.value = [];
    await loadSessions();
    await tauriEmit("history-updated");
  } catch (error) {
    console.error("Failed to clear sessions:", error);
  }
}

async function openSession(session: ChatSessionSummary) {
  try {
    if (
      session.workspaceId &&
      historyWorkspaces.value.some((workspace) => workspace.id === session.workspaceId)
    ) {
      await switchWorkspace(session.workspaceId);
    }
    await openSessionInOverlay(session.sessionId);
  } catch (err) {
    alert("Failed to open session: " + err);
  }
}

function formatTime(timestamp: number) {
  const date = new Date(timestamp);
  const now = new Date();

  const isToday = date.toDateString() === now.toDateString();
  if (isToday) {
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
  }

  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  const isYesterday = date.toDateString() === yesterday.toDateString();
  if (isYesterday) {
    return historyText.value.yesterday;
  }

  return `${date.getMonth() + 1}/${date.getDate()}`;
}

onMounted(() => {
  void loadSessions();
});
</script>
