<template>
  <Teleport to="body">
    <ul
      v-if="open"
      ref="menuRef"
      class="model-menu-floating"
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
        <span class="model-option-name">{{ option.label }}</span>
      </li>
    </ul>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from "vue";
import type { ToolApprovalMode } from "@/types/setting";

defineProps<{
  open: boolean;
  style: Record<string, string>;
  ariaLabel: string;
  options: Array<{ value: ToolApprovalMode; label: string }>;
  selectedValue: ToolApprovalMode;
}>();

defineEmits<{
  select: [value: ToolApprovalMode];
}>();

const menuRef = ref<HTMLUListElement | null>(null);

defineExpose({ menuEl: menuRef });
</script>
