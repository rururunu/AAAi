<template>
  <div
    class="command-list workspace-panel peek-scrollbar"
    data-tauri-drag-region="false"
  >
    <div class="workspace-panel-header">
      <span>{{ title }}</span>
    </div>

    <button
      v-if="!quickSelectOnly"
      type="button"
      class="workspace-new-row"
      :disabled="saving"
      @mousedown.prevent="$emit('addNew')"
    >
      <Plus :size="13" />
      {{ newWorkspaceLabel }}
    </button>
    <p
      v-if="quickSelectOnly && workspaces.length === 0"
      class="workspace-empty"
    >
      {{ noPreviousWorkspacesLabel }}
    </p>
    <button
      v-for="(workspace, index) in workspaces"
      :key="workspace.id"
      type="button"
      class="workspace-option"
      :class="{
        current: workspace.id === currentWorkspace?.id,
        active: selectedIndex === index + (quickSelectOnly ? 0 : 1),
      }"
      @mousedown.prevent="$emit('select', workspace)"
    >
      <span class="workspace-radio">
        <Check v-if="workspace.id === currentWorkspace?.id" :size="10" />
      </span>
      <span class="workspace-option-copy">
        <span class="workspace-option-title">
          <strong>{{ workspace.name }}</strong>
          <span v-if="workspaceSourceLabel(workspace.source)" class="workspace-source">
            {{ workspaceSourceLabel(workspace.source) }}
          </span>
        </span>
        <small>{{ workspace.root }}</small>
      </span>
    </button>
    <p v-if="error" class="workspace-error">{{ error }}</p>
  </div>
</template>

<script setup lang="ts">
import { Check, Plus } from "@lucide/vue";
import { workspaceSourceLabel, type Workspace } from "@/commands/workspace";

defineProps<{
  title: string;
  quickSelectOnly: boolean;
  workspaces: Workspace[];
  currentWorkspace: Workspace | null;
  selectedIndex: number;
  saving: boolean;
  error: string;
  newWorkspaceLabel: string;
  noPreviousWorkspacesLabel: string;
}>();

defineEmits<{
  addNew: [];
  select: [workspace: Workspace];
}>();
</script>

<style scoped>
.command-list {
  --command-row-height: 30px;
  --command-list-padding: 8px;
  --picker-meta-row-height: 28px;
  --command-list-visible-rows: 8;
  list-style: none;
  margin: 0;
  padding: 4px 0;
  border-bottom: 1px solid var(--peek-border);
  background: var(--peek-list-bg);
  flex: none;
  max-height: min(
    calc(var(--command-row-height) * var(--command-list-visible-rows) + var(--command-list-padding)),
    72vh
  );
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.workspace-panel {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px;
  max-height: min(320px, 72vh);
}

.workspace-panel-header {
  min-height: 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: var(--peek-text);
  font-size: 12px;
  font-weight: 600;
}

.workspace-new-row,
.workspace-option {
  border: 0;
  font: inherit;
  color: inherit;
  cursor: pointer;
}

.workspace-new-row,
.workspace-option {
  width: 100%;
  min-height: 34px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 7px;
  border-radius: 5px;
  background: transparent;
  text-align: left;
}

.workspace-new-row {
  color: var(--peek-accent);
  font-size: 12px;
}

.workspace-new-row:hover,
.workspace-option:hover,
.workspace-option.active,
.workspace-option.current {
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
}

.workspace-radio {
  width: 15px;
  height: 15px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  border: 1px solid var(--peek-border);
  border-radius: 50%;
}

.workspace-option.current .workspace-radio {
  border-color: var(--peek-accent);
  background: var(--peek-accent);
  color: var(--peek-surface);
}

.workspace-option-copy {
  min-width: 0;
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 1px;
}

.workspace-option-copy strong,
.workspace-option-copy small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workspace-option-title {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
}

.workspace-option-title strong {
  min-width: 0;
}

.workspace-option-copy strong {
  font-size: 12px;
  font-weight: 600;
}

.workspace-option-copy small {
  color: var(--peek-muted);
  font-size: 10px;
}

.workspace-source {
  flex: none;
  padding: 1px 5px;
  border: 1px solid color-mix(in srgb, var(--peek-accent) 38%, var(--peek-border));
  border-radius: 999px;
  color: var(--peek-accent);
  font-size: 9px;
  font-weight: 600;
  line-height: 1.3;
}

.workspace-error {
  margin: 0;
  color: var(--destructive);
  font-size: 10px;
  line-height: 1.4;
}

.workspace-empty {
  margin: 0;
  padding: 8px 7px;
  color: var(--peek-muted);
  font-size: 11px;
}
</style>
