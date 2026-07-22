<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    fill="currentColor"
    aria-hidden="true"
    :width="size"
    :height="size"
  >
    <path
      d="M12 2.5c.4 2.8 1.7 5 3.8 7.1 2.1 2.1 4.3 3.4 7.1 3.8-2.8.4-5 1.7-7.1 3.8-2.1 2.1-3.4 4.3-3.8 7.1-.4-2.8-1.7-5-3.8-7.1C6.1 14.1 3.9 12.8 1.1 12.4c2.8-.4 5-1.7 7.1-3.8C10.3 6.5 11.6 4.3 12 2.5z"
    />
  </svg>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    size?: number | string;
  }>(),
  {
    size: 16,
  },
);
</script>
