<template>
  <Teleport to="body">
    <ul
      v-if="open"
      ref="menuRef"
      class="model-menu-floating peek-scrollbar"
      data-tauri-drag-region="false"
      data-no-drag
      role="listbox"
      :aria-label="ariaLabel"
      :style="style"
      @mousedown.prevent
    >
      <li
        v-for="option in options"
        :key="option.value"
        class="model-menu-item"
        role="option"
        :class="{ active: option.value === selectedValue }"
        :aria-selected="option.value === selectedValue"
        @mousedown.prevent="$emit('select', option.value)"
      >
        <span class="model-option-leading">
          <component
            :is="option.icon"
            :size="12"
            class="mode-option-icon"
          />
        </span>
        <div class="model-option-text">
          <span class="model-option-name">{{ option.label }}</span>
        </div>
        <Check
          v-if="option.value === selectedValue"
          :size="12"
          class="model-option-check"
          aria-hidden="true"
        />
      </li>
    </ul>
  </Teleport>
</template>

<script setup lang="ts">
import type { Component } from "vue";
import { ref } from "vue";
import { Check } from "@lucide/vue";
import type { ChatMode } from "@/types/setting";

defineProps<{
  open: boolean;
  style: Record<string, string>;
  ariaLabel: string;
  options: Array<{
    value: ChatMode;
    label: string;
    description?: string;
    icon: Component;
  }>;
  selectedValue: ChatMode;
}>();

defineEmits<{
  select: [value: ChatMode];
}>();

const menuRef = ref<HTMLUListElement | null>(null);

defineExpose({ menuEl: menuRef });
</script>

<style scoped>
.model-option-leading {
  flex: none;
  width: 14px;
  height: 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.mode-option-icon {
  color: var(--peek-muted);
  opacity: 0.85;
  transition: color 140ms ease, opacity 140ms ease;
}

.model-menu-item:hover .mode-option-icon,
.model-menu-item.active .mode-option-icon {
  color: var(--peek-text);
  opacity: 1;
}

.model-option-text {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}

.model-option-check {
  flex: none;
  margin-left: auto;
  color: var(--peek-accent);
  opacity: 0.95;
}
</style>
