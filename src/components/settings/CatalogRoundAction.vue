<template>
  <button
    type="button"
    class="round-action"
    :class="{ done: props.done, busy: props.busy }"
    :disabled="props.disabled || props.busy || (props.lockWhenDone && props.done)"
    :aria-label="props.label"
    :title="props.label"
    @click="$emit('click')"
  >
    <Loader2 v-if="props.busy" class="size-3.5 animate-spin" />
    <Check v-else-if="props.done" class="size-3.5" />
    <component :is="resolvedIcon" v-else class="size-3.5" />
  </button>
</template>

<script setup lang="ts">
import { computed, type Component } from "vue";
import { Check, Loader2, Plus } from "@lucide/vue";

const props = withDefaults(
  defineProps<{
    done?: boolean;
    busy?: boolean;
    disabled?: boolean;
    /** When true (default for install), `done` also disables the button. */
    lockWhenDone?: boolean;
    label: string;
    icon?: Component;
  }>(),
  {
    done: false,
    busy: false,
    disabled: false,
    lockWhenDone: true,
    // Avoid putting a component object in withDefaults — resolve in computed instead.
    icon: undefined,
  },
);

defineEmits<{ click: [] }>();

const resolvedIcon = computed(() => props.icon ?? Plus);
</script>

<style scoped>
.round-action {
  width: 30px;
  height: 30px;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--border) 90%, transparent);
  background: color-mix(in srgb, var(--background) 70%, transparent);
  color: var(--muted-foreground);
  display: grid;
  place-items: center;
  flex-shrink: 0;
  cursor: pointer;
  transition:
    background 0.15s ease,
    color 0.15s ease,
    border-color 0.15s ease;
}

.round-action:hover:not(:disabled) {
  color: var(--foreground);
  border-color: color-mix(in srgb, var(--foreground) 22%, var(--border));
  background: color-mix(in srgb, var(--foreground) 5%, var(--background));
}

.round-action:disabled {
  cursor: default;
  opacity: 0.7;
}

.round-action.done {
  color: var(--foreground);
  opacity: 1;
}

.round-action.busy {
  opacity: 1;
}
</style>
