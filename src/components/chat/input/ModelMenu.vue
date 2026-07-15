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
      <template v-if="loading">
        <li class="model-menu-item model-status">
          {{ loadingText }}
        </li>
      </template>
      <template v-else-if="error">
        <li class="model-menu-item model-status error">
          {{ error }}
        </li>
      </template>
      <template v-else-if="models.length === 0">
        <li class="model-menu-item model-status">
          {{ emptyText }}
        </li>
      </template>
      <template v-else>
        <li
          v-for="model in models"
          :key="model.id"
          class="model-menu-item"
          role="option"
          :class="{ active: model.id === selectedModelId }"
          :aria-selected="model.id === selectedModelId"
          @mousedown.prevent="$emit('select', model.id)"
        >
          <span class="model-option-name">{{ model.id }}</span>
          <span v-if="model.ownedBy" class="model-option-id">{{ model.ownedBy }}</span>
        </li>
      </template>
    </ul>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from "vue";
import type { ChatModelInfo } from "@/types/chat";

defineProps<{
  open: boolean;
  style: Record<string, string>;
  ariaLabel: string;
  loading: boolean;
  error: string | null;
  models: ChatModelInfo[];
  selectedModelId: string;
  loadingText: string;
  emptyText: string;
}>();

defineEmits<{
  select: [modelId: string];
}>();

const menuRef = ref<HTMLUListElement | null>(null);

defineExpose({ menuEl: menuRef });
</script>
