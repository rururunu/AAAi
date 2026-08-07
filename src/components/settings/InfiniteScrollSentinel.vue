<template>
  <div ref="el" class="infinite-scroll-sentinel" aria-hidden="true">
    <Loader2 v-if="loading" class="size-3.5 animate-spin" />
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { Loader2 } from "@lucide/vue";

const props = defineProps<{
  /** Whether more pages can be loaded. */
  hasMore: boolean;
  /** True while a fetch is in flight. */
  loading?: boolean;
}>();

const emit = defineEmits<{
  load: [];
}>();

const el = ref<HTMLElement | null>(null);
let observer: IntersectionObserver | null = null;

function findScrollRoot(node: HTMLElement | null): Element | null {
  let current: HTMLElement | null = node?.parentElement ?? null;
  while (current) {
    const style = getComputedStyle(current);
    const overflowY = style.overflowY;
    if (overflowY === "auto" || overflowY === "scroll" || overflowY === "overlay") {
      return current;
    }
    current = current.parentElement;
  }
  return null;
}

function disconnect() {
  observer?.disconnect();
  observer = null;
}

function connect() {
  disconnect();
  const target = el.value;
  if (!target || !props.hasMore) return;

  observer = new IntersectionObserver(
    (entries) => {
      const hit = entries.some((entry) => entry.isIntersecting);
      if (!hit || props.loading || !props.hasMore) return;
      emit("load");
    },
    {
      root: findScrollRoot(target),
      rootMargin: "160px 0px",
      threshold: 0,
    },
  );
  observer.observe(target);
}

onMounted(() => {
  connect();
});

onBeforeUnmount(() => {
  disconnect();
});

watch(
  () => [props.hasMore, props.loading] as const,
  () => {
    // Re-arm after loads finish so a still-visible sentinel can fetch again.
    connect();
    if (props.hasMore && !props.loading && el.value) {
      const root = findScrollRoot(el.value);
      const rect = el.value.getBoundingClientRect();
      const rootRect = root?.getBoundingClientRect();
      const visible = rootRect
        ? rect.top <= rootRect.bottom + 160
        : rect.top <= window.innerHeight + 160;
      if (visible) emit("load");
    }
  },
);
</script>

<style scoped>
.infinite-scroll-sentinel {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 20px;
  padding: 4px 0 0;
  color: var(--muted-foreground);
}
</style>
