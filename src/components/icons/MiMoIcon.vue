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
      d="M4.5 5.25h3.2l2.55 7.4 2.55-7.4h3.2l-4.35 13.5H8.85L4.5 5.25Zm11.2 0H19.5v13.5h-3.05c-.55-1.7-1.35-3.55-2.35-5.55.9-1.7 1.75-3.55 2.35-5.25 0-.55.15-1.65.25-2.7Z"
    />
  </svg>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ size?: number | string }>(), { size: 16 });
</script>
