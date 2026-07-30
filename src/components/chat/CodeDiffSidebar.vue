<template>
  <aside
    class="diff-sidebar"
    :class="{ embedded }"
    :style="{ width: `${props.width}px` }"
    data-tauri-drag-region="false"
    :aria-label="tr(settingStore.language, 'codeChanges')"
  >
    <div v-if="changes.length" class="diff-sidebar-body">
      <nav ref="changeFilesRef" class="change-files peek-card-tabs" :aria-label="tr(settingStore.language, 'changedFiles')" @wheel="scrollFileTabs">
        <button
          v-for="change in changes"
          :key="change.id"
          type="button"
          class="change-file peek-card-tab"
          :class="{ active: change.id === activeId }"
          :title="change.path"
          @click="activeId = change.id"
        >
          <FileCode2 :size="15" :stroke-width="1.7" aria-hidden="true" />
          <span class="change-file-name">{{ fileName(change.path) }}</span>
          <span class="change-file-stats">
            <span class="added">+{{ change.added }}</span>
            <span class="removed">-{{ change.removed }}</span>
          </span>
        </button>
      </nav>

      <section v-if="activeChange" class="diff-view">
        <header class="diff-view-header">
          <div class="diff-path" :title="activeChange.path">{{ activeChange.path }}</div>
          <div class="diff-view-actions">
            <div class="view-mode-switch" role="group" :aria-label="tr(settingStore.language, 'diffViewMode')">
              <button
                type="button"
                :class="{ active: viewMode === 'unified' }"
                :aria-label="tr(settingStore.language, 'diffUnified')"
                :title="tr(settingStore.language, 'diffUnified')"
                @click="setViewMode('unified')"
              >
                <Rows3 :size="14" aria-hidden="true" />
              </button>
              <button
                type="button"
                :class="{ active: viewMode === 'split' }"
                :aria-label="tr(settingStore.language, 'diffSplit')"
                :title="tr(settingStore.language, 'diffSplit')"
                @click="setViewMode('split')"
              >
                <Columns2 :size="14" aria-hidden="true" />
              </button>
            </div>
            <button
              type="button"
              class="icon-button"
              :class="{ copied: copiedId === activeChange.id }"
              :aria-label="tr(settingStore.language, copiedId === activeChange.id ? 'copied' : 'copyDiff')"
              :title="tr(settingStore.language, copiedId === activeChange.id ? 'copied' : 'copyDiff')"
              @click="copyActiveDiff"
            >
              <Check v-if="copiedId === activeChange.id" :size="14" aria-hidden="true" />
              <Copy v-else :size="14" aria-hidden="true" />
            </button>
          </div>
        </header>

        <div v-if="viewMode === 'unified'" class="diff-scroll peek-scrollbar">
          <div class="diff-code" role="table" :aria-label="tr(settingStore.language, 'codeChanges')">
            <div
              v-for="(line, index) in activeChange.lines"
              :key="`${activeChange.id}-${index}`"
              class="diff-row"
              :class="line.kind"
              role="row"
            >
              <span class="line-number" role="cell">{{ line.oldLine ?? "" }}</span>
              <span class="line-number" role="cell">{{ line.newLine ?? "" }}</span>
              <span class="line-marker" role="cell">{{ marker(line.kind) }}</span>
              <code role="cell">{{ line.text }}</code>
            </div>
          </div>
        </div>
        <div v-else class="diff-split-view">
          <div class="split-column-headings" aria-hidden="true">
            <span>{{ tr(settingStore.language, "diffOriginal") }}</span>
            <span>{{ tr(settingStore.language, "diffModified") }}</span>
          </div>
          <div class="diff-scroll peek-scrollbar">
            <div class="diff-split-code" role="table" :aria-label="tr(settingStore.language, 'codeChanges')">
              <template v-for="(row, index) in splitRows" :key="`${activeChange.id}-split-${index}`">
                <div v-if="row.kind === 'separator'" class="split-separator" role="row">
                  {{ row.label }}
                </div>
                <div v-else class="split-row" role="row">
                  <div class="diff-pane" :class="row.left?.kind ?? 'empty'">
                    <span class="line-number">{{ row.left?.oldLine ?? "" }}</span>
                    <span class="line-marker">{{ marker(row.left?.kind) }}</span>
                    <code>{{ row.left?.text ?? "" }}</code>
                  </div>
                  <div class="diff-pane" :class="row.right?.kind ?? 'empty'">
                    <span class="line-number">{{ row.right?.newLine ?? "" }}</span>
                    <span class="line-marker">{{ marker(row.right?.kind) }}</span>
                    <code>{{ row.right?.text ?? "" }}</code>
                  </div>
                </div>
              </template>
            </div>
          </div>
        </div>
      </section>
    </div>

    <div v-else class="diff-empty">
      <FileDiff :size="28" :stroke-width="1.35" aria-hidden="true" />
      <p>{{ tr(settingStore.language, "noCodeChanges") }}</p>
      <span>{{ tr(settingStore.language, "changesAppearHere") }}</span>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  Check,
  Columns2,
  Copy,
  FileCode2,
  FileDiff,
  Rows3,
} from "@lucide/vue";
import { copyText } from "@/services/clipboard";
import { tr } from "@/services/i18n";
import { extractCodeChanges } from "@/services/chat/codeChanges";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage } from "@/types/chat";

type DiffLineKind = "context" | "addition" | "deletion" | "hunk" | "meta";

type DiffLine = {
  kind: DiffLineKind;
  text: string;
  oldLine?: number;
  newLine?: number;
};

type SplitRow =
  | { kind: "separator"; label: string }
  | { kind: "pair"; left?: DiffLine; right?: DiffLine };

type DiffViewMode = "unified" | "split";

type ChangeEntry = {
  id: string;
  path: string;
  diff: string;
  lines: DiffLine[];
  added: number;
  removed: number;
};

const props = defineProps<{
  messages: ChatMessage[];
  width: number;
  embedded?: boolean;
}>();

const settingStore = useSettingStore();
const activeId = ref("");
const changeFilesRef = ref<HTMLElement | null>(null);
const copiedId = ref("");
const storedViewMode = localStorage.getItem("aaai.diffViewMode");
const viewMode = ref<DiffViewMode>(storedViewMode === "split" ? "split" : "unified");
let copyResetTimer: ReturnType<typeof setTimeout> | null = null;

const changes = computed<ChangeEntry[]>(() => {
  return extractCodeChanges(props.messages).map((change) => ({
    ...change,
    lines: parseUnifiedDiff(change.diff),
  })).reverse();
});

const activeChange = computed(() =>
  changes.value.find((change) => change.id === activeId.value) ?? changes.value[0],
);
const splitRows = computed(() => buildSplitRows(activeChange.value?.lines ?? []));

function scrollFileTabs(event: WheelEvent) {
  const tabs = changeFilesRef.value;
  if (!tabs || tabs.scrollWidth <= tabs.clientWidth) return;
  event.preventDefault();
  tabs.scrollLeft += Math.abs(event.deltaY) >= Math.abs(event.deltaX) ? event.deltaY : event.deltaX;
}

watch(
  changes,
  (next) => {
    if (!next.some((change) => change.id === activeId.value)) {
      activeId.value = next[0]?.id ?? "";
    }
  },
  { immediate: true },
);

function parseUnifiedDiff(diff: string): DiffLine[] {
  let oldLine = 0;
  let newLine = 0;
  return diff.replace(/\r\n/g, "\n").split("\n").map((raw): DiffLine => {
    const hunk = raw.match(/^@@\s+-(\d+)(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s+@@(.*)$/);
    if (hunk) {
      oldLine = Number(hunk[1]);
      newLine = Number(hunk[2]);
      return { kind: "hunk", text: raw };
    }
    if (raw.startsWith("diff --git ") || raw.startsWith("index ") || raw.startsWith("--- ") || raw.startsWith("+++ ")) {
      return { kind: "meta", text: raw };
    }
    if (raw.startsWith("+")) {
      const line = { kind: "addition" as const, text: raw.slice(1), newLine };
      newLine += 1;
      return line;
    }
    if (raw.startsWith("-")) {
      const line = { kind: "deletion" as const, text: raw.slice(1), oldLine };
      oldLine += 1;
      return line;
    }
    if (raw.startsWith(" ")) {
      const line = { kind: "context" as const, text: raw.slice(1), oldLine, newLine };
      oldLine += 1;
      newLine += 1;
      return line;
    }
    return { kind: "meta", text: raw };
  });
}

function buildSplitRows(lines: DiffLine[]): SplitRow[] {
  const rows: SplitRow[] = [];
  let index = 0;
  while (index < lines.length) {
    const line = lines[index]!;
    if (line.kind === "meta") {
      index += 1;
      continue;
    }
    if (line.kind === "hunk") {
      rows.push({ kind: "separator", label: line.text });
      index += 1;
      continue;
    }
    if (line.kind === "context") {
      rows.push({ kind: "pair", left: line, right: line });
      index += 1;
      continue;
    }

    const deletions: DiffLine[] = [];
    const additions: DiffLine[] = [];
    while (lines[index]?.kind === "deletion") deletions.push(lines[index++]!);
    while (lines[index]?.kind === "addition") additions.push(lines[index++]!);
    if (!deletions.length && !additions.length && line.kind === "addition") {
      additions.push(line);
      index += 1;
    }
    const rowCount = Math.max(deletions.length, additions.length);
    for (let pairIndex = 0; pairIndex < rowCount; pairIndex += 1) {
      rows.push({
        kind: "pair",
        left: deletions[pairIndex],
        right: additions[pairIndex],
      });
    }
  }
  return rows;
}

function marker(kind?: DiffLineKind) {
  if (kind === "addition") return "+";
  if (kind === "deletion") return "-";
  return "";
}

function setViewMode(mode: DiffViewMode) {
  viewMode.value = mode;
  localStorage.setItem("aaai.diffViewMode", mode);
}

function fileName(path: string) {
  return path.replace(/\\/g, "/").split("/").pop() || path;
}

async function copyActiveDiff() {
  const change = activeChange.value;
  if (!change) return;
  await copyText(change.diff);
  copiedId.value = change.id;
  if (copyResetTimer) clearTimeout(copyResetTimer);
  copyResetTimer = setTimeout(() => {
    copiedId.value = "";
    copyResetTimer = null;
  }, 1600);
}
</script>

<style scoped>
.diff-sidebar {
  flex: none;
  box-sizing: border-box;
  width: 520px;
  min-width: 320px;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding-top: 34px;
  border: 0;
  border-left: 0;
  border-radius: 0;
  background: transparent;
  color: var(--peek-text);
}

.diff-sidebar.embedded { flex: 1; width: 100% !important; min-width: 0; padding-top: 0; }

.icon-button {
  flex: none;
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}

.icon-button:hover {
  border-color: var(--peek-border);
  background: var(--peek-hover-bg);
  color: var(--peek-text);
}

.icon-button.copied { color: #4ade80; }

.diff-view-actions { flex: none; display: flex; align-items: center; gap: 5px; }
.view-mode-switch {
  display: inline-flex;
  height: 26px;
  padding: 2px;
  border: 1px solid color-mix(in srgb, var(--peek-text) 11%, var(--peek-border));
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-input-bg) 70%, transparent);
}
.view-mode-switch button {
  width: 25px;
  height: 20px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--peek-icon, var(--peek-muted));
  cursor: pointer;
}
.view-mode-switch button:hover { color: var(--peek-text); }
.view-mode-switch button.active {
  background: color-mix(in srgb, var(--peek-accent) 16%, var(--peek-surface));
  color: var(--peek-accent);
}

.diff-sidebar-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.change-files {
  flex: none;
}

.change-file {
  flex: 0 0 200px;
  width: 200px;
  min-width: 200px;
  max-width: 260px;
  gap: 7px;
  padding-right: 9px;
  padding-left: 9px;
  text-align: left;
}
.change-file-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11px; }
.change-file-stats { flex: none; display: flex; gap: 5px; font: 10px/1 var(--font-mono); font-variant-numeric: tabular-nums; }
.added { color: #4ade80; }
.removed { color: #fb7185; }

.diff-view { flex: 1; min-height: 0; display: flex; flex-direction: column; }
.diff-view-header { flex: none; min-height: 38px; display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 4px 7px 4px 10px; border-bottom: 1px solid color-mix(in srgb, var(--peek-text) 8%, var(--peek-border)); }
.diff-path { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--peek-muted); font-size: 10px; }
.diff-scroll { flex: 1; min-height: 0; overflow-x: hidden; overflow-y: auto; background: transparent; }
.diff-code { width: 100%; min-width: 0; padding: 7px 0 16px; font: 11px/1.55 var(--font-mono); }
.diff-row { min-height: 17px; display: grid; grid-template-columns: 38px 38px 18px minmax(0, 1fr); }
.diff-row.addition { background: color-mix(in srgb, #22c55e 15%, transparent); }
.diff-row.deletion { background: color-mix(in srgb, #f43f5e 15%, transparent); }
.diff-row.hunk { margin: 5px 0; background: color-mix(in srgb, var(--peek-accent) 12%, transparent); color: var(--peek-accent); }
.diff-row.meta { color: var(--peek-muted); }
.line-number { padding-right: 7px; color: color-mix(in srgb, var(--peek-muted) 70%, transparent); text-align: right; user-select: none; }
.line-marker { color: var(--peek-muted); text-align: center; user-select: none; }
.diff-row.addition .line-marker { color: #4ade80; }
.diff-row.deletion .line-marker { color: #fb7185; }
.diff-row code { min-width: 0; padding-right: 14px; color: inherit; font: inherit; white-space: pre-wrap; overflow-wrap: anywhere; }

.diff-split-view { flex: 1; min-height: 0; display: flex; flex-direction: column; }
.split-column-headings {
  flex: none;
  height: 27px;
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  border-bottom: 1px solid color-mix(in srgb, var(--peek-text) 8%, var(--peek-border));
  color: var(--peek-muted);
  font-size: 9px;
  line-height: 27px;
  text-transform: uppercase;
}
.split-column-headings span { padding: 0 9px; }
.split-column-headings span + span { border-left: 1px solid color-mix(in srgb, var(--peek-text) 9%, var(--peek-border)); }
.diff-split-code { width: 100%; min-width: 0; padding: 7px 0 16px; font: 11px/1.55 var(--font-mono); }
.split-row { width: 100%; min-width: 0; display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); }
.diff-pane { min-width: 0; min-height: 17px; display: grid; grid-template-columns: 36px 16px minmax(0, 1fr); }
.diff-pane + .diff-pane { border-left: 1px solid color-mix(in srgb, var(--peek-text) 9%, var(--peek-border)); }
.diff-pane code { min-width: 0; padding-right: 9px; color: inherit; font: inherit; white-space: pre-wrap; overflow-wrap: anywhere; }
.diff-pane.addition { background: color-mix(in srgb, #22c55e 15%, transparent); }
.diff-pane.deletion { background: color-mix(in srgb, #f43f5e 15%, transparent); }
.diff-pane.empty { background: color-mix(in srgb, var(--peek-text) 2%, transparent); }
.diff-pane.addition .line-marker { color: #4ade80; }
.diff-pane.deletion .line-marker { color: #fb7185; }
.split-separator {
  min-height: 24px;
  padding: 4px 9px;
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
  color: var(--peek-accent);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.diff-empty {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 7px;
  padding: 28px;
  color: var(--peek-muted);
  text-align: center;
}

.diff-empty p { margin: 5px 0 0; color: var(--peek-text); font-size: 12px; }
.diff-empty span { max-width: 240px; font-size: 10px; line-height: 1.5; }
</style>
