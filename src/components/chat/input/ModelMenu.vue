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
          <span class="model-option-leading">
            <component
              :is="providerIcon(model.provider)"
              v-if="providerIcon(model.provider)"
              :size="12"
              class="model-option-icon"
            />
            <span v-else class="model-option-icon-fallback" aria-hidden="true" />
          </span>
          <div class="model-option-text">
            <span class="model-option-name">{{ formatModelDisplayName(model.id, model.provider) }}</span>
            <span
              v-if="model.ownedBy && !isDeepSeekProvider(model.provider)"
              class="model-option-id"
            >{{ model.ownedBy }}</span>
          </div>
          <Check
            v-if="model.id === selectedModelId"
            :size="12"
            class="model-option-check"
            aria-hidden="true"
          />
        </li>
      </template>
    </ul>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { Check } from "@lucide/vue";
import type { ChatModelInfo } from "@/types/chat";
import { getProviderIcon, formatModelDisplayName, isDeepSeekProvider } from "@/lib/providerIcons";

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
const providerIcon = getProviderIcon;

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

.model-option-icon {
  color: var(--peek-muted);
  opacity: 0.85;
  transition: color 140ms ease, opacity 140ms ease;
}

.model-option-icon-fallback {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--peek-muted) 45%, transparent);
}

.model-menu-item:hover .model-option-icon,
.model-menu-item.active .model-option-icon {
  color: var(--peek-text);
  opacity: 1;
}

.model-option-text {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.model-option-check {
  flex: none;
  margin-left: auto;
  color: var(--peek-accent);
  opacity: 0.95;
}
</style>
