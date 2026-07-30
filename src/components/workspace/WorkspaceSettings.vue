<template>
  <section class="workspace-settings">
    <AppConfirmDialog ref="confirmDialogRef" />
    <header>
      <div>
        <h2>{{ copy.title }}</h2>
        <p>{{ copy.description }}</p>
      </div>
      <Button
        size="sm"
        class="h-8 gap-1.5"
        :disabled="saving"
        @click="addWorkspace"
      >
        <Plus class="size-3.5" />
        {{ copy.newWorkspace }}
      </Button>
    </header>

    <p v-if="error" class="form-error">{{ error }}</p>

    <div class="workspace-list">
      <p v-if="filtered.length === 0" class="empty">{{ copy.empty }}</p>
      <article v-for="workspace in filtered" :key="workspace.id">
        <button type="button" class="workspace-select" @click="select(workspace)">
          <span class="radio" :class="{ active: workspace.id === current?.id }">
            <Check v-if="workspace.id === current?.id" class="size-3" />
          </span>
          <span class="copy">
            <span class="workspace-title">
              <strong>{{ workspace.name }}</strong>
              <span v-if="workspaceSourceLabel(workspace.source)" class="workspace-source">
                {{ workspaceSourceLabel(workspace.source) }}
              </span>
            </span>
            <span>{{ workspace.root }}</span>
          </span>
          <span v-if="workspace.id === current?.id" class="current-label">{{ copy.current }}</span>
        </button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="size-8 shrink-0 text-muted-foreground hover:text-destructive"
          :title="copy.deleteWorkspace"
          :aria-label="copy.deleteWorkspace"
          @click="remove(workspace)"
        >
          <Trash2 class="size-3.5" />
        </Button>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Check, Plus, Trash2 } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import {
  createWorkspace,
  deleteWorkspace,
  getCurrentWorkspace,
  listWorkspaces,
  selectWorkspaceFolder,
  switchWorkspace,
  workspaceSourceLabel,
  type Workspace,
} from "@/commands/workspace";

const props = defineProps<{ query?: string }>();
const settingStore = useSettingStore();
const workspaces = ref<Workspace[]>([]);
const current = ref<Workspace | null>(null);
const saving = ref(false);
const error = ref("");
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
let unlisten: UnlistenFn | null = null;

const copy = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "workspace.title"),
    description: tr(language, "workspace.description"),
    newWorkspace: tr(language, "workspace.newWorkspace"),
    empty: tr(language, "workspace.empty"),
    current: tr(language, "workspace.current"),
    deleteWorkspace: tr(language, "workspace.deleteWorkspace"),
    cancel: tr(language, "workspace.cancel"),
    confirmDelete: tr(language, "workspace.confirmDelete"),
    deleteConfirm: (name: string) => tr(language, "workspace.deleteConfirm", { name }),
  };
});

const filtered = computed(() => {
  const query = props.query?.trim().toLowerCase() ?? "";
  if (!query) return workspaces.value;
  return workspaces.value.filter((workspace) =>
    `${workspace.name} ${workspace.root}`.toLowerCase().includes(query),
  );
});

async function load() {
  [workspaces.value, current.value] = await Promise.all([
    listWorkspaces(),
    getCurrentWorkspace(),
  ]);
}

async function addWorkspace() {
  if (saving.value) return;
  saving.value = true;
  error.value = "";
  try {
    const root = await selectWorkspaceFolder();
    if (!root) return;
    const workspace = await createWorkspace(root);
    current.value = await switchWorkspace(workspace.id);
    await load();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    saving.value = false;
  }
}

async function select(workspace: Workspace) {
  if (workspace.id !== current.value?.id) {
    current.value = await switchWorkspace(workspace.id);
  }
}

async function remove(workspace: Workspace) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: copy.value.deleteWorkspace,
    description: copy.value.deleteConfirm(workspace.name),
    confirmLabel: copy.value.confirmDelete,
    cancelLabel: copy.value.cancel,
  });
  if (!confirmed) return;
  await deleteWorkspace(workspace.id);
  await load();
}

onMounted(async () => {
  await load();
  unlisten = await listen("workspaces-changed", () => void load());
});

onUnmounted(() => unlisten?.());
</script>

<style scoped>
.workspace-settings { padding: 16px; }
.workspace-settings > header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; padding-bottom: 14px; }
.workspace-settings h2 { margin: 0; font-size: 14px; font-weight: 600; }
.workspace-settings header p, .empty { margin: 3px 0 0; color: var(--muted-foreground); font-size: 12px; }
.form-error { margin: 0 0 10px; color: var(--destructive); font-size: 11px; }
.workspace-list { border-top: 1px solid var(--border); }
.empty { padding: 28px 0; text-align: center; }
.workspace-list article { min-height: 58px; display: flex; align-items: center; gap: 8px; border-bottom: 1px solid var(--border); }
.workspace-select { min-width: 0; display: flex; flex: 1; align-items: center; gap: 10px; padding: 10px 4px; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
.radio { width: 17px; height: 17px; display: inline-flex; align-items: center; justify-content: center; flex: none; border: 1px solid var(--border); border-radius: 50%; }
.radio.active { border-color: var(--primary); background: var(--primary); color: var(--primary-foreground); }
.copy { min-width: 0; display: flex; flex: 1; flex-direction: column; gap: 2px; }
.workspace-title { min-width: 0; display: flex; align-items: center; gap: 7px; }
.copy strong { min-width: 0; overflow: hidden; font-size: 13px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
.copy span { overflow: hidden; color: var(--muted-foreground); font-size: 11px; text-overflow: ellipsis; white-space: nowrap; }
.copy .workspace-source { flex: none; padding: 1px 6px; border: 1px solid color-mix(in srgb, var(--primary) 35%, var(--border)); border-radius: 999px; color: var(--primary); font-size: 9px; font-weight: 600; line-height: 1.35; }
.current-label { flex: none; color: var(--primary); font-size: 11px; font-weight: 600; }
</style>
