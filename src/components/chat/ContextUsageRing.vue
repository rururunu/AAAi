<template>
  <TooltipProvider :delay-duration="200">
    <Tooltip>
      <TooltipTrigger as-child>
        <div
          class="context-usage-ring"
          :class="tone"
          data-tauri-drag-region="false"
          role="meter"
          :aria-label="tooltip"
          :aria-valuemin="0"
          :aria-valuemax="100"
          :aria-valuenow="percent"
        >
          <svg
            class="context-usage-ring-svg"
            :width="size"
            :height="size"
            :viewBox="`0 0 ${size} ${size}`"
            aria-hidden="true"
          >
            <circle
              class="track"
              :cx="center"
              :cy="center"
              :r="radius"
              fill="none"
              :stroke-width="stroke"
            />
            <circle
              class="progress"
              :cx="center"
              :cy="center"
              :r="radius"
              fill="none"
              :stroke-width="stroke"
              :stroke-dasharray="circumference"
              :stroke-dashoffset="dashOffset"
              stroke-linecap="round"
              :transform="`rotate(-90 ${center} ${center})`"
            />
          </svg>
        </div>
      </TooltipTrigger>
      <TooltipContent side="top" align="center" :side-offset="6">
        {{ tooltip }}
      </TooltipContent>
    </Tooltip>
  </TooltipProvider>
</template>

<script setup lang="ts">
import { computed } from "vue";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

const props = withDefaults(
  defineProps<{
    ratio: number;
    tooltip?: string;
    size?: number;
  }>(),
  {
    ratio: 0,
    tooltip: "",
    size: 18,
  },
);

const stroke = 2;
const center = computed(() => props.size / 2);
const radius = computed(() => (props.size - stroke) / 2);
const circumference = computed(() => 2 * Math.PI * radius.value);
const clampedRatio = computed(() => Math.max(0, Math.min(props.ratio, 1)));
const dashOffset = computed(
  () => circumference.value * (1 - clampedRatio.value),
);
const percent = computed(() => Math.round(props.ratio * 100));

const tone = computed(() => {
  if (props.ratio >= 0.9) return "critical";
  if (props.ratio >= 0.7) return "warn";
  return "normal";
});
</script>

<style scoped>
.context-usage-ring {
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  cursor: default;
}

.context-usage-ring-svg {
  display: block;
}

.track {
  stroke: color-mix(in srgb, var(--peek-muted) 28%, transparent);
}

.progress {
  transition: stroke-dashoffset 220ms ease, stroke 180ms ease;
}

.context-usage-ring.normal .progress {
  stroke: color-mix(in srgb, var(--peek-accent) 72%, var(--peek-muted));
}

.context-usage-ring.warn .progress {
  stroke: #f59e0b;
}

.context-usage-ring.critical .progress {
  stroke: #ef4444;
}
</style>
