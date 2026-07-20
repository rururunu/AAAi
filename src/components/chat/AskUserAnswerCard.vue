<template>
  <details class="ask-answer-card" role="status" :aria-label="label">
    <summary class="tool-activity-header">
      <ChevronRight class="activity-chevron" :size="13" />
      <span class="tool-activity-icon" aria-hidden="true">
        <CircleCheck :size="13" />
      </span>
      <span class="tool-activity-title">{{ label }}</span>
    </summary>

    <div class="tool-activity-detail ask-answer-body">
      <div
        v-for="(item, index) in items"
        :key="`${item.header ?? 'q'}-${index}`"
        class="ask-answer-row"
      >
        <div v-if="item.header" class="ask-answer-topic">{{ item.header }}</div>
        <div v-if="item.userSupplement" class="ask-answer-value muted">
          {{ supplementText }}
        </div>
        <div v-else class="ask-answer-value">
          {{ item.selected.join("、") }}
        </div>
      </div>
    </div>
  </details>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { ChevronRight, CircleCheck } from "@lucide/vue";
import { useSettingStore } from "@/stores/setting";
import type { AskUserAnswerItem } from "@/types/chat";
import { tr } from "@/services/i18n";

defineProps<{
  items: AskUserAnswerItem[];
}>();

const settingStore = useSettingStore();
const { language } = storeToRefs(settingStore);

const label = computed(() => tr(language.value, "yourChoice"));
const supplementText = computed(() => tr(language.value, "customAnswer"));
</script>

<style scoped>
.ask-answer-card {
  align-self: stretch;
  width: 100%;
  max-width: none;
  margin: 0;
  border: 0;
  border-radius: 6px;
  background: transparent;
  overflow: hidden;
  box-sizing: border-box;
}

.tool-activity-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  color: var(--peek-muted);
  font-size: 11px;
  line-height: 1.35;
  cursor: pointer;
  list-style: none;
  user-select: none;
  border-radius: 6px;
  transition: background 120ms ease, color 120ms ease;
}

.tool-activity-header:hover {
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
  color: var(--peek-text);
}

.tool-activity-header::-webkit-details-marker {
  display: none;
}

.activity-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 140ms ease;
}

.ask-answer-card[open] > .tool-activity-header .activity-chevron {
  transform: rotate(90deg);
}

.ask-answer-card[open] > .tool-activity-header {
  color: var(--peek-text);
  border-bottom: 0;
}

.tool-activity-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  width: 16px;
  height: 16px;
  border-radius: 4px;
  background: color-mix(in srgb, #22c55e 14%, transparent);
  color: #22c55e;
}

.tool-activity-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tool-activity-detail {
  padding: 2px 6px 8px 28px;
  font-size: 11px;
}

.ask-answer-body {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.ask-answer-row {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.ask-answer-topic {
  color: var(--peek-muted);
  font-size: 10px;
  line-height: 1.35;
}

.ask-answer-value {
  color: var(--peek-text);
  font-size: 12px;
  line-height: 1.45;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}

.ask-answer-value.muted {
  color: var(--peek-muted);
  font-style: italic;
}
</style>
