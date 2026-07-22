<template>
  <div class="accent-color-field">
    <div class="accent-preview" :style="previewStyle">
      <button
        type="button"
        class="accent-swatch"
        :aria-label="pickColorLabel"
        @click="openPicker"
      >
        <span class="accent-swatch-fill" :style="{ background: displayColor }" />
        <span class="accent-swatch-ring" :style="{ borderColor: displayColor }" />
      </button>

      <div class="accent-samples">
        <span class="accent-sample accent-sample-link">Link</span>
        <span class="accent-sample accent-sample-pill">Action</span>
        <span class="accent-sample accent-sample-dot" />
      </div>

      <input
        ref="colorInputRef"
        type="color"
        class="accent-native-input"
        :value="displayColor"
        tabindex="-1"
        @input="onPickerInput"
      />
    </div>

    <div class="accent-controls">
      <Input
        v-model="hexDraft"
        class="accent-hex-input h-8 font-mono text-xs uppercase"
        maxlength="7"
        spellcheck="false"
        :aria-label="hexLabel"
        @keydown.enter.prevent="commitHex"
        @blur="commitHex"
      />
      <button
        v-if="!isDefault"
        type="button"
        class="accent-reset-btn"
        :title="resetLabel"
        :aria-label="resetLabel"
        @click="emit('reset')"
      >
        <RotateCcw :size="14" />
      </button>
    </div>

    <div class="accent-presets" role="listbox" :aria-label="presetsLabel">
      <button
        v-for="preset in presets"
        :key="preset.color"
        type="button"
        class="accent-preset"
        :class="{ active: displayColor === preset.color }"
        :title="preset.label"
        :aria-label="preset.label"
        @click="selectPreset(preset.color)"
      >
        <span class="accent-preset-fill" :style="{ background: preset.color }" />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { RotateCcw } from "@lucide/vue";
import { Input } from "@/components/ui/input";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import {
  isDefaultAccentColor,
  normalizeAccentColor,
} from "@/types/setting";

const props = defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
  reset: [];
}>();

const settingStore = useSettingStore();
const colorInputRef = ref<HTMLInputElement | null>(null);
const hexDraft = ref(normalizeAccentColor(props.modelValue));

watch(
  () => props.modelValue,
  (value) => {
    hexDraft.value = normalizeAccentColor(value);
  },
);

const displayColor = computed(() => normalizeAccentColor(props.modelValue));
const isDefault = computed(() => isDefaultAccentColor(props.modelValue));

const previewStyle = computed(() => ({
  "--preview-accent": displayColor.value,
  "--preview-accent-soft": `color-mix(in srgb, ${displayColor.value} 16%, transparent)`,
  "--preview-accent-border": `color-mix(in srgb, ${displayColor.value} 38%, transparent)`,
}));

const presets = [
  { color: "#e8ecf2", label: "Soft white" },
  { color: "#ffffff", label: "Pure white" },
  { color: "#3794ff", label: "Blue" },
  { color: "#6ea8e0", label: "Sky" },
  { color: "#a78bfa", label: "Purple" },
  { color: "#5ecf8a", label: "Green" },
  { color: "#f08aa0", label: "Rose" },
  { color: "#c07a3a", label: "Amber" },
];

const resetLabel = computed(() => tr(settingStore.language, "resetThemeColor"));
const pickColorLabel = computed(() =>
  settingStore.language === "zh-CN" ? "打开取色器" : "Open color picker",
);
const hexLabel = computed(() =>
  settingStore.language === "zh-CN" ? "十六进制颜色值" : "Hex color value",
);
const presetsLabel = computed(() =>
  settingStore.language === "zh-CN" ? "预设颜色" : "Preset colors",
);

function openPicker() {
  colorInputRef.value?.click();
}

function onPickerInput(event: Event) {
  const value = (event.target as HTMLInputElement).value;
  if (/^#[0-9a-f]{6}$/i.test(value)) {
    emit("update:modelValue", value.toLowerCase());
  }
}

function commitHex() {
  let next = hexDraft.value.trim();
  if (!next.startsWith("#")) {
    next = `#${next}`;
  }
  if (/^#[0-9a-f]{6}$/i.test(next)) {
    emit("update:modelValue", next.toLowerCase());
    return;
  }
  hexDraft.value = displayColor.value;
}

function selectPreset(color: string) {
  emit("update:modelValue", color);
}
</script>

<style scoped>
.accent-color-field {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
}

.accent-preview {
  position: relative;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--muted) 72%, transparent), transparent),
    var(--card);
}

.accent-swatch {
  position: relative;
  flex-shrink: 0;
  width: 44px;
  height: 44px;
  padding: 0;
  border: none;
  border-radius: 12px;
  background:
    linear-gradient(45deg, #555 25%, transparent 25%),
    linear-gradient(-45deg, #555 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, #555 75%),
    linear-gradient(-45deg, transparent 75%, #555 75%);
  background-size: 10px 10px;
  background-position: 0 0, 0 5px, 5px -5px, -5px 0;
  background-color: #333;
  cursor: pointer;
}

.accent-swatch-fill {
  position: absolute;
  inset: 4px;
  border-radius: 9px;
  box-shadow: inset 0 0 0 1px rgb(255 255 255 / 18%);
}

.accent-swatch-ring {
  position: absolute;
  inset: -2px;
  border: 2px solid transparent;
  border-radius: 14px;
  pointer-events: none;
  opacity: 0.85;
}

.accent-samples {
  display: flex;
  flex: 1;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.accent-sample-link {
  font-size: 12px;
  font-weight: 500;
  color: var(--preview-accent);
  text-decoration: underline;
  text-decoration-color: color-mix(in srgb, var(--preview-accent) 45%, transparent);
  text-underline-offset: 3px;
}

.accent-sample-pill {
  padding: 2px 10px;
  border: 1px solid var(--preview-accent-border);
  border-radius: 999px;
  background: var(--preview-accent-soft);
  color: var(--preview-accent);
  font-size: 11px;
  line-height: 1.4;
}

.accent-sample-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--preview-accent);
  box-shadow: 0 0 0 3px var(--preview-accent-soft);
}

.accent-native-input {
  position: absolute;
  width: 0;
  height: 0;
  opacity: 0;
  pointer-events: none;
}

.accent-controls {
  display: flex;
  align-items: center;
  gap: 6px;
}

.accent-hex-input {
  flex: 1;
  min-width: 0;
}

.accent-reset-btn {
  display: inline-flex;
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--background);
  color: var(--muted-foreground);
  transition: background 140ms ease, color 140ms ease, border-color 140ms ease;
}

.accent-reset-btn:hover {
  border-color: color-mix(in srgb, var(--preview-accent) 35%, var(--border));
  background: var(--preview-accent-soft);
  color: var(--foreground);
}

.accent-presets {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.accent-preset {
  position: relative;
  width: 22px;
  height: 22px;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: 999px;
  background:
    linear-gradient(45deg, #666 25%, transparent 25%),
    linear-gradient(-45deg, #666 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, #666 75%),
    linear-gradient(-45deg, transparent 75%, #666 75%);
  background-size: 6px 6px;
  background-position: 0 0, 0 3px, 3px -3px, -3px 0;
  background-color: #444;
  cursor: pointer;
  transition: transform 120ms ease, box-shadow 120ms ease;
}

.accent-preset:hover {
  transform: translateY(-1px);
}

.accent-preset.active {
  box-shadow: 0 0 0 2px var(--background), 0 0 0 3px var(--preview-accent);
}

.accent-preset-fill {
  position: absolute;
  inset: 2px;
  border-radius: 999px;
  box-shadow: inset 0 0 0 1px rgb(255 255 255 / 16%);
}
</style>
