<script setup lang="ts">
import { ref, watch, onBeforeUnmount, nextTick } from "vue";
import { RotateCcw } from "@lucide/vue";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";

function codeToPrimary(code: string, key: string): string | null {
  if (code === "Space" || key === " ") return "Space";
  if (code === "Tab") return "Tab";
  if (code === "Enter") return "Enter";
  if (code === "Backspace") return "Backspace";
  if (code === "Delete") return "Delete";
  if (code === "Insert") return "Insert";
  if (code === "Home") return "Home";
  if (code === "End") return "End";
  if (code === "PageUp") return "PageUp";
  if (code === "PageDown") return "PageDown";
  if (code === "ArrowLeft") return "Left";
  if (code === "ArrowRight") return "Right";
  if (code === "ArrowUp") return "Up";
  if (code === "ArrowDown") return "Down";
  if (code === "Semicolon") return ";";
  if (code === "Quote") return "'";
  if (code === "Comma") return ",";
  if (code === "Period") return ".";
  if (code === "Slash") return "/";
  if (code === "Backslash") return "\\";
  if (code === "Minus") return "-";
  if (code === "Equal") return "=";
  if (code === "BracketLeft") return "[";
  if (code === "BracketRight") return "]";
  if (code === "Backquote") return "`";
  if (/^F([1-9]|1[0-2])$/.test(code)) return code;
  if (/^Digit([0-9])$/.test(code)) return code.slice(5);
  if (/^Key([A-Z])$/.test(code)) return code.slice(3);
  if (key.length === 1 && /[a-zA-Z0-9]/.test(key)) return key.toUpperCase();
  return null;
}

function eventToHotkey(event: KeyboardEvent): string | null {
  const primary = codeToPrimary(event.code, event.key);
  if (!primary) return null;
  if (!(event.ctrlKey || event.shiftKey || event.altKey || event.metaKey)) {
    return null;
  }
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.shiftKey) parts.push("Shift");
  if (event.altKey) parts.push("Alt");
  if (event.metaKey) parts.push("Meta");
  parts.push(primary);
  return parts.join("+");
}

const props = withDefaults(defineProps<{
  modelValue: string;
  settingKey: "primaryHotkey" | "secondaryHotkey";
  mode?: "double-modifier" | "chord";
  defaultValue: string;
}>(), {
  mode: "chord",
});

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const settingStore = useSettingStore();
const recording = ref(false);
const draft = ref("");
const buttonRef = ref<HTMLButtonElement | null>(null);

const displayValue = () => props.modelValue || props.defaultValue;
const formattedDisplayValue = () => props.mode === "double-modifier"
  ? `${displayValue()} x 2`
  : displayValue().split("+").join(" + ");

function startRecording() {
  recording.value = true;
  draft.value = "";
  void nextTick(() => buttonRef.value?.focus());
}

function cancelRecording() {
  recording.value = false;
  draft.value = "";
}

async function commitHotkey(value: string) {
  recording.value = false;
  draft.value = "";
  if (value === props.modelValue) return;
  emit("update:modelValue", value);
  await settingStore.update(
    props.settingKey === "primaryHotkey"
      ? { primaryHotkey: value }
      : { secondaryHotkey: value },
  );
}

async function resetDefault() {
  cancelRecording();
  if (props.modelValue === props.defaultValue) return;
  emit("update:modelValue", props.defaultValue);
  await settingStore.update(
    props.settingKey === "primaryHotkey"
      ? { primaryHotkey: props.defaultValue }
      : { secondaryHotkey: props.defaultValue },
  );
}

function onKeyDown(event: KeyboardEvent) {
  if (!recording.value) return;
  event.preventDefault();
  event.stopPropagation();

  if (event.key === "Escape") {
    cancelRecording();
    return;
  }

  if (props.mode === "double-modifier") {
    const modifier = event.key === "Control" ? "Ctrl" : event.key;
    if (["Ctrl", "Shift", "Alt", "Meta"].includes(modifier)) {
      draft.value = `${modifier} x 2`;
      void commitHotkey(modifier);
    }
    return;
  }

  if (["Control", "Shift", "Alt", "Meta"].includes(event.key)) {
    const parts: string[] = [];
    if (event.ctrlKey) parts.push("Ctrl");
    if (event.shiftKey) parts.push("Shift");
    if (event.altKey) parts.push("Alt");
    if (event.metaKey) parts.push("Meta");
    draft.value = parts.join(" + ") + (parts.length ? " + …" : "…");
    return;
  }

  const hotkey = eventToHotkey(event);
  if (!hotkey) return;
  draft.value = hotkey.split("+").join(" + ");
  void commitHotkey(hotkey);
}

function onBlur() {
  if (recording.value) {
    cancelRecording();
  }
}

watch(
  () => props.modelValue,
  () => {
    if (!recording.value) draft.value = "";
  },
);

onBeforeUnmount(() => {
  cancelRecording();
});
</script>

<template>
  <div class="flex w-full flex-col gap-2">
    <button
      ref="buttonRef"
      type="button"
      class="hotkey-record-btn"
      :class="{ recording }"
      :aria-pressed="recording"
      @click="startRecording"
      @keydown="onKeyDown"
      @blur="onBlur"
    >
      <span v-if="recording" class="text-muted-foreground">
        {{ draft || tr(settingStore.language, "settings.hotkey.recording") }}
      </span>
      <span v-else class="font-mono text-xs tracking-wide">
        {{ formattedDisplayValue() }}
      </span>
    </button>
    <div class="flex items-center gap-2">
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground text-[11px] underline-offset-2 hover:underline"
        @click="startRecording"
      >
        {{ tr(settingStore.language, "settings.hotkey.record") }}
      </button>
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground inline-flex items-center gap-1 text-[11px] underline-offset-2 hover:underline"
        :title="tr(settingStore.language, 'settings.hotkey.reset')"
        @click="resetDefault"
      >
        <RotateCcw class="size-3" />
        {{ tr(settingStore.language, "settings.hotkey.reset") }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.hotkey-record-btn {
  display: flex;
  width: 100%;
  min-height: 2rem;
  align-items: center;
  justify-content: center;
  border-radius: 0.5rem;
  border: 1px solid var(--border);
  background: var(--background);
  padding: 0.35rem 0.6rem;
  text-align: center;
  transition:
    border-color 120ms ease,
    box-shadow 120ms ease;
}

.hotkey-record-btn:hover {
  border-color: color-mix(in srgb, var(--peek-accent, var(--primary)) 45%, var(--border));
}

.hotkey-record-btn.recording {
  border-color: var(--peek-accent, var(--primary));
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--peek-accent, var(--primary)) 35%, transparent);
}
</style>
