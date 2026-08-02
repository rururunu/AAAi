<template>
  <component
    :is="icon"
    class="subagent-icon"
    :class="state"
    :size="size"
    :stroke-width="1.8"
    :style="{ '--subagent-icon-size': `${size}px` }"
    aria-hidden="true"
  />
</template>

<script setup lang="ts">
import { computed } from "vue";
import { CircleCheck, CircleX, LoaderCircle, Workflow } from "@lucide/vue";

const props = withDefaults(defineProps<{
  status?: string;
  size?: number;
}>(), {
  status: "idle",
  size: 14,
});

const state = computed(() => {
  if (props.status === "running") return "running";
  if (props.status === "error" || props.status === "failed") return "error";
  if (props.status === "done" || props.status === "completed") return "done";
  return "idle";
});

const icon = computed(() => {
  if (state.value === "running") return LoaderCircle;
  if (state.value === "error") return CircleX;
  if (state.value === "done") return CircleCheck;
  return Workflow;
});
</script>

<style scoped>
.subagent-icon {
  flex: none;
  width: var(--subagent-icon-size);
  height: var(--subagent-icon-size);
  color: currentColor;
}

.subagent-icon.running {
  color: var(--peek-accent);
  animation: subagent-icon-spin 900ms linear infinite;
}

.subagent-icon.done { color: #58b887; }
.subagent-icon.error { color: var(--destructive); }

@keyframes subagent-icon-spin {
  to { transform: rotate(360deg); }
}
</style>
