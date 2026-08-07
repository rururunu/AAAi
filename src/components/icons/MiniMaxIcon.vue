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
      d="M12 3.5 4.8 7.4v9.2L12 20.5l7.2-3.9V7.4L12 3.5Zm0 2.35 4.55 2.45v.2L12 10.95 7.45 8.5v-.2L12 5.85Zm-5.05 4.05 4.2 2.25v4.55l-4.2-2.3V9.9Zm6.9 6.8v-4.55l4.2-2.25v4.5l-4.2 2.3Z"
    />
  </svg>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ size?: number | string }>(), { size: 16 });
</script>
