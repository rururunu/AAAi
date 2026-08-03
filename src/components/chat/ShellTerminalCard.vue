<template>
  <section class="shell-terminal-card" :class="[status, { collapsed: !expanded }]">
    <button
      type="button"
      class="shell-terminal-header"
      :aria-expanded="expanded"
      @click="expanded = !expanded"
    >
      <ChevronRight class="shell-terminal-chevron" :class="{ open: expanded }" :size="12" aria-hidden="true" />
      <span class="shell-terminal-prompt" aria-hidden="true">&gt;_</span>
      <span class="shell-terminal-title">{{ title }}</span>
      <span v-if="status === 'running'" class="shell-terminal-status">{{ runningLabel }}</span>
      <span v-else-if="status === 'error'" class="shell-terminal-status error">{{ failedLabel }}</span>
    </button>
    <pre v-if="expanded && body" class="shell-terminal-body peek-scrollbar"><code>{{ body }}</code></pre>
    <pre v-else-if="expanded && status === 'running'" class="shell-terminal-body muted peek-scrollbar"><code>{{ waitingLabel }}</code></pre>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ChevronRight } from "@lucide/vue";
import type { ToolActivity } from "@/types/chat";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";

const props = withDefaults(defineProps<{
  activity: ToolActivity;
  startCollapsed?: boolean;
}>(), {
  startCollapsed: false,
});

const settingStore = useSettingStore();
const expanded = ref(!props.startCollapsed || props.activity.status === "running");
const runningLabel = computed(() => tr(settingStore.language, "running"));
const failedLabel = computed(() => tr(settingStore.language, "failed"));
const waitingLabel = computed(() =>
  settingStore.language === "zh-CN" ? "正在运行…" : "Running…",
);

const status = computed(() => props.activity.status);

watch(
  () => props.activity.status,
  (next, prev) => {
    if (next === "running") expanded.value = true;
    else if (props.startCollapsed && prev === "running" && next !== "running") {
      expanded.value = false;
    }
  },
);

const title = computed(() => {
  const args = props.activity.arguments ?? {};
  const description = typeof args.description === "string" ? args.description.trim() : "";
  if (description) return description;
  const raw = props.activity.title
    .replace(/^执行命令[：:]\s*/u, "")
    .replace(/^运行命令[：:]\s*/u, "")
    .replace(/^Run(?:ning)?(?:\s+command)?[：:]\s*/i, "")
    .trim();
  return raw || props.activity.title;
});

const body = computed(() => {
  const command = String(props.activity.arguments?.command ?? "").trim();
  const output = (props.activity.result ?? extractOutputFromDetail(props.activity.detail)).trim();
  if (output) {
    if (command && !output.includes(command.slice(0, Math.min(40, command.length)))) {
      return `$ ${command}\n\n${output}`;
    }
    return output;
  }
  if (command) return `$ ${command}`;
  return "";
});

function extractOutputFromDetail(detail?: string | null): string {
  if (!detail) return "";
  const outputMatch = detail.match(/\*\*输出[：:]\*\*\s*```[^\n]*\n([\s\S]*?)```/);
  if (outputMatch?.[1]) return outputMatch[1].trimEnd();
  const fence = detail.match(/```(?:powershell|bash|shell|ps1)?\n([\s\S]*?)```/);
  return fence?.[1]?.trimEnd() ?? detail;
}
</script>

<style scoped>
.shell-terminal-card {
  width: 100%;
  box-sizing: border-box;
  margin: 4px 0 8px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-input-bg) 92%, #0b0d10);
  overflow: hidden;
}
.shell-terminal-card.running {
  border-color: color-mix(in srgb, var(--peek-accent) 35%, var(--peek-border));
}
.shell-terminal-card.error {
  border-color: color-mix(in srgb, var(--destructive) 40%, var(--peek-border));
}
.shell-terminal-header {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  min-height: 34px;
  margin: 0;
  padding: 7px 12px;
  border: 0;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
  background: color-mix(in srgb, var(--peek-panel) 55%, transparent);
  color: color-mix(in srgb, var(--peek-text) 88%, var(--peek-muted));
  font: inherit;
  font-size: 12px;
  line-height: 1.35;
  text-align: left;
  cursor: pointer;
}
.shell-terminal-card.collapsed .shell-terminal-header {
  border-bottom: 0;
}
.shell-terminal-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 140ms ease;
}
.shell-terminal-chevron.open {
  transform: rotate(90deg);
}
.shell-terminal-prompt {
  flex: none;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 700;
  color: var(--peek-muted);
  letter-spacing: -0.04em;
}
.shell-terminal-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 550;
}
.shell-terminal-status {
  flex: none;
  font-size: 10px;
  color: var(--peek-muted);
}
.shell-terminal-status.error {
  color: var(--destructive);
}
.shell-terminal-body {
  margin: 0;
  max-height: var(--agent-card-max-height, 240px);
  overflow: auto;
  padding: 10px 12px 12px;
  color: color-mix(in srgb, var(--peek-text) 82%, #c8d0d8);
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-word;
}
.shell-terminal-body.muted {
  color: var(--peek-muted);
}
.shell-terminal-body code {
  font: inherit;
  color: inherit;
  background: transparent;
}
</style>
