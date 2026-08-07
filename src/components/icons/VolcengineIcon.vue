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
      d="M6.2 5h11.6c.66 0 1.2.54 1.2 1.2v4.4c0 .66-.54 1.2-1.2 1.2h-4.2v6c0 .66-.54 1.2-1.2 1.2H6.2c-.66 0-1.2-.54-1.2-1.2V6.2C5 5.54 5.54 5 6.2 5Zm1.5 2.4v8.7h3.3V11.8h4.6V7.4H7.7Z"
    />
  </svg>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ size?: number | string }>(), { size: 16 });
</script>
