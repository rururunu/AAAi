<template>
  <div v-if="active" class="plan-mode-banner">
    <div class="plan-copy">
      <strong>{{ tr(language, "planModeActive") }}</strong>
      <span>{{ tr(language, "planModeHint") }}</span>
    </div>
    <div class="plan-actions">
      <Button type="button" variant="ghost" size="sm" :disabled="busy" @click="emit('cancel')">
        {{ tr(language, "planModeCancel") }}
      </Button>
      <Button type="button" variant="default" size="sm" :disabled="busy" @click="emit('approve')">
        {{ tr(language, "planModeApprove") }}
      </Button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Button } from "@/components/ui/button";
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
  border: 1px solid color-mix(in srgb, var(--peek-warning) 40%, transparent);
  background: color-mix(in srgb, var(--peek-warning) 14%, transparent);
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
</style>
