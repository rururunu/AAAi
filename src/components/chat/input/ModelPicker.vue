<template>
  <ul
    class="command-list model-picker-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <template v-if="loading && models.length === 0">
      <li class="picker-status">{{ loadingText }}</li>
    </template>
    <template v-else-if="error && models.length === 0">
      <li class="picker-status error">{{ error }}</li>
    </template>
    <template v-else-if="models.length === 0">
      <li class="picker-status">{{ emptyText }}</li>
    </template>
    <template v-else>
      <template v-for="group in groups" :key="group.provider">
        <li class="model-group-header" role="presentation">
          <span class="model-group-leading" aria-hidden="true">
            <component
              :is="providerIcon(group.provider === 'other' ? '' : group.provider)"
              v-if="providerIcon(group.provider === 'other' ? '' : group.provider)"
              :size="12"
              class="model-group-icon"
            />
            <span v-else class="model-icon-dot" />
          </span>
          <span class="model-group-label">{{ group.label }}</span>
        </li>
        <li
          v-for="entry in group.entries"
          :key="`${entry.model.provider}:${entry.model.id}`"
          class="command-item model-picker-item"
          :class="{
            active: entry.index === selectedIndex,
            current: isModelEntrySelected(entry.model, selectedModelId, selectedProvider),
          }"
          role="option"
          :aria-selected="entry.index === selectedIndex"
          @mouseenter="$emit('hover', entry.index)"
          @mousedown.prevent="$emit('select', entry.model)"
        >
          <span class="model-name">{{ getModelDisplayLabel(entry.model) }}</span>

          <span
            v-if="getModelDisplaySubtitle(entry.model)"
            class="model-meta"
          >{{ getModelDisplaySubtitle(entry.model) }}</span>

          <Check
            v-if="isModelEntrySelected(entry.model, selectedModelId, selectedProvider)"
            :size="13"
            class="model-check"
            aria-hidden="true"
          />
        </li>
      </template>
    </template>

    <li
      class="command-item model-picker-refresh"
      :class="{ active: selectedIndex === refreshIndex }"
      role="option"
      :aria-selected="selectedIndex === refreshIndex"
      :aria-disabled="refreshing"
      @mouseenter="$emit('hover', refreshIndex)"
      @mousedown.prevent="!refreshing && $emit('refresh')"
    >
      <RefreshCw
        :size="12"
        class="refresh-icon"
        :class="{ spinning: refreshing }"
      />
      <span class="refresh-label">{{ refreshText }}</span>
    </li>
  </ul>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Check, RefreshCw } from "@lucide/vue";
import type { ChatModelInfo } from "@/types/chat";
import {
  getModelDisplayLabel,
  getModelDisplaySubtitle,
  getProviderIcon,
  groupModelsByProvider,
} from "@/lib/providerIcons";
import { isModelEntrySelected } from "@/lib/modelThinking";

const props = defineProps<{
  models: ChatModelInfo[];
  selectedModelId: string;
  selectedProvider: string;
  selectedIndex: number;
  loading: boolean;
  refreshing?: boolean;
  error: string | null;
  loadingText: string;
  emptyText: string;
  refreshText: string;
  ariaLabel: string;
}>();

defineEmits<{
  hover: [index: number];
  select: [model: ChatModelInfo];
  refresh: [];
}>();

const providerIcon = getProviderIcon;

/** Flat model index for keyboard nav; headers are skipped. */
const groups = computed(() => {
  let index = 0;
  return groupModelsByProvider(props.models).map((group) => ({
    provider: group.provider,
    label: group.label,
    entries: group.models.map((model) => {
      const entry = { model, index };
      index += 1;
      return entry;
    }),
  }));
});

/** Refresh is always the last navigable row (models[0..n-1], then refresh). */
const refreshIndex = computed(() => props.models.length);
</script>

<style scoped>
.command-list {
  --command-row-height: 32px;
  --command-list-padding: 6px;
  --command-list-visible-rows: 12;
  --model-group-header-height: 24px;
  list-style: none;
  margin: 0;
  padding: 4px 0 0;
  border-bottom: 1px solid var(--peek-border);
  background: var(--peek-list-bg);
  flex: none;
  max-height: min(
    calc(
      var(--command-row-height) * var(--command-list-visible-rows) +
        var(--command-list-padding) +
        34px
    ),
    calc(100vh - 140px)
  );
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.picker-status {
  padding: 8px 12px;
  font-size: 12px;
  line-height: 1.45;
  color: var(--peek-muted);
  pointer-events: none;
}

.picker-status.error {
  color: color-mix(in srgb, var(--destructive) 82%, var(--peek-muted));
}

.model-group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: var(--model-group-header-height);
  padding: 6px 12px 2px;
  pointer-events: none;
  user-select: none;
}

.model-group-header + .model-group-header {
  margin-top: 2px;
}

.model-picker-item + .model-group-header {
  margin-top: 5px;
  border-top: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
}

.model-group-leading {
  flex: none;
  width: 16px;
  height: 16px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--peek-muted);
  opacity: 0.9;
}

.model-group-icon {
  opacity: 0.9;
}

.model-group-label {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--peek-muted);
}

.command-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px 0 36px;
  height: var(--command-row-height);
  cursor: default;
}

.model-picker-refresh {
  padding-left: 12px;
}

.command-item.active {
  background: var(--peek-list-active);
}

.model-picker-item.current:not(.active) {
  background: color-mix(in srgb, var(--peek-accent) 7%, transparent);
}

.model-icon-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--peek-muted) 55%, transparent);
}

.model-name {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  font-weight: 500;
  line-height: 16px;
  color: var(--peek-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.model-meta {
  flex: none;
  max-width: 38%;
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 14px;
  color: var(--peek-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.model-check {
  flex: none;
  color: var(--peek-accent);
  opacity: 0.95;
}

.model-picker-refresh {
  margin-top: 2px;
  height: 30px;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 90%, transparent);
  color: var(--peek-muted);
  gap: 8px;
}

.model-picker-refresh.active {
  color: var(--peek-text);
}

.model-picker-refresh[aria-disabled="true"] {
  opacity: 0.55;
  cursor: default;
}

.refresh-icon {
  flex: none;
  opacity: 0.85;
}

.refresh-label {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  line-height: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.spinning {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
