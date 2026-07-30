<template>
  <section v-if="changes.length" class="changes-summary">
    <header class="changes-summary-header">
      <span class="changes-summary-icon" aria-hidden="true">
        <FileDiff :size="18" :stroke-width="1.8" />
      </span>
      <div class="changes-summary-title">
        <strong>{{ tr(settingStore.language, "editedFiles", { count: changes.length }) }}</strong>
        <span class="changes-summary-total">
          <span class="added">+{{ totals.added }}</span>
          <span class="removed">-{{ totals.removed }}</span>
        </span>
      </div>
      <div class="changes-summary-actions">
        <button
          v-if="canUndo"
          type="button"
          :disabled="busy"
          @click="$emit('undo')"
        >
          {{ tr(settingStore.language, "undoChanges") }}
          <Undo2 :size="14" :stroke-width="1.8" aria-hidden="true" />
        </button>
        <button type="button" class="review-button" @click="$emit('review')">
          {{ tr(settingStore.language, "reviewChanges") }}
        </button>
      </div>
    </header>

    <button
      v-for="change in changes"
      :key="change.id"
      type="button"
      class="changes-summary-file"
      :title="change.path"
      @click="$emit('review')"
    >
      <span class="file-path">{{ change.path }}</span>
      <span class="file-stats">
        <span class="added">+{{ change.added }}</span>
        <span class="removed">-{{ change.removed }}</span>
      </span>
    </button>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { FileDiff, Undo2 } from "@lucide/vue";
import { extractCodeChanges } from "@/services/chat/codeChanges";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage } from "@/types/chat";

const props = withDefaults(defineProps<{
  message: ChatMessage;
  canUndo?: boolean;
  busy?: boolean;
}>(), {
  canUndo: false,
  busy: false,
});

defineEmits<{
  undo: [];
  review: [];
}>();

const settingStore = useSettingStore();
const changes = computed(() => extractCodeChanges([props.message]));
const totals = computed(() => changes.value.reduce(
  (total, change) => ({
    added: total.added + change.added,
    removed: total.removed + change.removed,
  }),
  { added: 0, removed: 0 },
));
</script>

<style scoped>
.changes-summary {
  width: 100%;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--peek-text) 13%, var(--peek-border));
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-input-bg) 68%, transparent);
}

.changes-summary-header {
  min-height: 58px;
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 9px 12px;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-text) 9%, var(--peek-border));
}

.changes-summary-icon {
  flex: none;
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid color-mix(in srgb, var(--peek-accent) 34%, var(--peek-border));
  border-radius: 6px;
  color: var(--peek-accent);
  background: color-mix(in srgb, var(--peek-accent) 9%, transparent);
}

.changes-summary-title {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.changes-summary-title strong {
  overflow: hidden;
  color: var(--peek-text);
  font-size: 12px;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.changes-summary-total, .file-stats { display: inline-flex; gap: 5px; font: 10px/1.2 var(--font-mono); font-variant-numeric: tabular-nums; }
.added { color: #4ade80; }
.removed { color: #fb7185; }

.changes-summary-actions { flex: none; display: flex; align-items: center; gap: 5px; }
.changes-summary-actions button {
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 0 8px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--peek-muted);
  font-size: 11px;
  white-space: nowrap;
  cursor: pointer;
}
.changes-summary-actions button:hover:not(:disabled) { color: var(--peek-text); background: var(--peek-hover-bg); }
.changes-summary-actions button:disabled { opacity: 0.45; cursor: default; }
.changes-summary-actions .review-button { border-color: color-mix(in srgb, var(--peek-text) 13%, var(--peek-border)); color: var(--peek-text); }

.changes-summary-file {
  width: 100%;
  min-width: 0;
  min-height: 31px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 5px 13px 5px 17px;
  border: 0;
  background: transparent;
  color: var(--peek-muted);
  text-align: left;
  cursor: pointer;
}
.changes-summary-file:hover { background: color-mix(in srgb, var(--peek-text) 4%, transparent); color: var(--peek-text); }
.file-path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11px; }
.file-stats { flex: none; }

</style>
