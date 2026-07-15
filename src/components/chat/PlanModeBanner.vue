<template>
  <div v-if="active" class="plan-mode-banner">
    <div class="plan-copy">
      <strong>{{ tr(language, "planModeActive") }}</strong>
      <span>{{ tr(language, "planModeHint") }}</span>
    </div>
    <div class="plan-actions">
      <button type="button" class="btn ghost" :disabled="busy" @click="emit('cancel')">
        {{ tr(language, "planModeCancel") }}
      </button>
      <button type="button" class="btn primary" :disabled="busy" @click="emit('approve')">
        {{ tr(language, "planModeApprove") }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { tr } from "@/services/i18n";
import type { AppLanguage } from "@/types/setting";

defineProps<{
  active: boolean;
  language: AppLanguage;
  busy?: boolean;
}>();

const emit = defineEmits<{
  approve: [];
  cancel: [];
}>();
</script>

<style scoped>
.plan-mode-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin: 0 12px 8px;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid color-mix(in srgb, #f39c12 40%, transparent);
  background: color-mix(in srgb, #f39c12 14%, transparent);
}

.plan-copy {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.plan-copy strong {
  font-size: 12px;
}

.plan-copy span {
  font-size: 11px;
  opacity: 0.75;
}

.plan-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.btn {
  border: 0;
  border-radius: 8px;
  padding: 6px 10px;
  font-size: 12px;
  cursor: pointer;
  color: inherit;
}

.btn.ghost {
  background: color-mix(in srgb, #fff 10%, transparent);
}

.btn.primary {
  background: color-mix(in srgb, #f39c12 45%, transparent);
}

.btn:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
