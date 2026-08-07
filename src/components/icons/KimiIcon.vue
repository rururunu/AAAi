<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    fill="currentColor"
    aria-hidden="true"
    :width="size"
    :height="size"
  >
    <path d="M6.2 5h3.1l2.7 6.35L14.7 5h3.1v14.5h-2.85v-8.2l-2.55 5.9h-1.8l-2.55-5.9v8.2H6.2V5Z" />
  </svg>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ size?: number | string }>(), { size: 16 });
</script>
