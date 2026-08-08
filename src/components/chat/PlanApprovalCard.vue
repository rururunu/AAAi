<template>
  <section v-if="visible" class="plan-approval-card">
    <header class="plan-approval-header">
      <span class="plan-approval-icon" aria-hidden="true">
        <ListChecks :size="16" :stroke-width="1.8" />
      </span>
      <div class="plan-approval-title">
        <strong>{{ tr(settingStore.language, "planModeActive") }}</strong>
        <span>{{ tr(settingStore.language, "planModeReviewHint") }}</span>
      </div>
    </header>

    <ul v-if="tasks.length" class="plan-approval-list">
      <li
        v-for="(task, index) in tasks"
        :key="`${index}-${task.content}`"
        class="plan-approval-item"
        :class="statusClass(task.status)"
        :style="{ paddingLeft: `${12 + Math.min(task.level ?? 0, 6) * 12}px` }"
      >
        <span class="plan-approval-marker" aria-hidden="true">
          <Check v-if="isCompleted(task.status)" :size="12" :stroke-width="2.5" />
          <span v-else class="plan-approval-dot" />
        </span>
        <span class="plan-approval-content">{{ task.content }}</span>
      </li>
    </ul>
    <p v-else class="plan-approval-empty">
      {{ tr(settingStore.language, "planModeNoTasksYet") }}
    </p>

    <div class="plan-approval-actions">
      <button
        type="button"
        class="plan-approval-btn ghost"
        :disabled="busy"
        @click="$emit('cancel')"
      >
        {{ tr(settingStore.language, "planModeCancel") }}
      </button>
      <button
        type="button"
        class="plan-approval-btn primary"
        :disabled="busy || !tasks.length"
        @click="$emit('approve')"
      >
        {{ tr(settingStore.language, "planModeApprove") }}
      </button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Check, ListChecks } from "@lucide/vue";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { TaskItem } from "@/types/chat";

const props = withDefaults(
  defineProps<{
    tasks: TaskItem[];
    visible?: boolean;
    busy?: boolean;
  }>(),
  {
    visible: true,
    busy: false,
  },
);

defineEmits<{
  approve: [];
  cancel: [];
}>();

const settingStore = useSettingStore();

const tasks = computed(() => props.tasks);

function isCompleted(status: string) {
  const value = status.toLowerCase();
  return value === "completed" || value === "done" || value === "complete";
}

function statusClass(status: string) {
  if (isCompleted(status)) return "completed";
  const value = status.toLowerCase();
  if (value === "in_progress" || value === "active" || value === "running") return "active";
  if (value === "cancelled" || value === "canceled") return "cancelled";
  return "pending";
}
</script>

<style scoped>
.plan-approval-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: 10px;
  padding: 12px;
  border: 1px solid color-mix(in srgb, var(--peek-accent) 22%, var(--peek-border));
  border-radius: 14px;
  background: color-mix(in srgb, var(--peek-accent) 6%, var(--peek-surface));
}

.plan-approval-header {
  display: flex;
  gap: 10px;
  align-items: flex-start;
}

.plan-approval-icon {
  flex: none;
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
  color: var(--peek-accent);
}

.plan-approval-title {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.plan-approval-title strong {
  font-size: 13px;
  font-weight: 650;
  line-height: 18px;
  color: var(--peek-text);
}

.plan-approval-title span {
  font-size: 12px;
  line-height: 16px;
  color: var(--peek-muted);
}

.plan-approval-list {
  list-style: none;
  margin: 0;
  padding: 4px 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.plan-approval-item {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  min-height: 28px;
  padding: 4px 8px 4px 12px;
  border-radius: 8px;
  color: var(--peek-text);
  font-size: 12.5px;
  line-height: 18px;
}

.plan-approval-item.active {
  background: color-mix(in srgb, var(--peek-accent) 8%, transparent);
}

.plan-approval-item.completed {
  color: var(--peek-muted);
}

.plan-approval-item.completed .plan-approval-content {
  text-decoration: line-through;
  text-decoration-color: color-mix(in srgb, var(--peek-muted) 55%, transparent);
}

.plan-approval-marker {
  flex: none;
  width: 16px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--peek-accent);
}

.plan-approval-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--peek-muted) 55%, transparent);
}

.plan-approval-item.active .plan-approval-dot {
  background: var(--peek-accent);
}

.plan-approval-content {
  flex: 1;
  min-width: 0;
}

.plan-approval-empty {
  margin: 0;
  padding: 6px 2px;
  color: var(--peek-muted);
  font-size: 12px;
}

.plan-approval-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-end;
}

.plan-approval-btn {
  height: 30px;
  padding: 0 12px;
  border-radius: 8px;
  border: 1px solid transparent;
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.plan-approval-btn:disabled {
  opacity: 0.55;
  cursor: default;
}

.plan-approval-btn.ghost {
  border-color: color-mix(in srgb, var(--peek-text) 12%, transparent);
  background: transparent;
  color: var(--peek-muted);
}

.plan-approval-btn.primary {
  background: var(--peek-accent);
  color: var(--peek-accent-fg, #fff);
}
</style>
