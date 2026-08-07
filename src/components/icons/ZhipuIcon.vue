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
      d="M5 4.5h5.4c2.85 0 4.7 1.55 4.7 4.05 0 1.7-.85 2.95-2.35 3.55L16.6 19.5h-3.45l-3.4-6.55H8.05V19.5H5V4.5Zm3.05 2.45v3.85h2.05c1.35 0 2.1-.6 2.1-1.9s-.75-1.95-2.15-1.95H8.05Z"
    />
  </svg>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ size?: number | string }>(), { size: 16 });
</script>
