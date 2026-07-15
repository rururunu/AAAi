<template>
  <ul
    class="command-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li
      v-for="(item, index) in commands"
      :key="item.command"
      class="command-item"
      :class="{ active: index === selectedIndex }"
      role="option"
      :aria-selected="index === selectedIndex"
      @mouseenter="$emit('hover', index)"
      @mousedown.prevent="$emit('select', item.command)"
    >
      <span class="command-name">{{ item.command }}</span>
      <span class="command-desc">{{ item.description }}</span>
    </li>
  </ul>
</template>

<script setup lang="ts">
defineProps<{
  commands: Array<{ command: string; description: string }>;
  selectedIndex: number;
  ariaLabel: string;
}>();

defineEmits<{
  hover: [index: number];
  select: [command: string];
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

.command-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 12px;
  height: var(--command-row-height);
  cursor: default;
}

.command-item.active {
  background: var(--peek-list-active);
}

.command-name {
  flex: none;
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--peek-accent);
}

.command-desc {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  color: var(--peek-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
