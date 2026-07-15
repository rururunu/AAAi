<template>
  <div v-if="enrichedActivities.length" class="tool-activity-list" :class="{ operations }">
    <details
      v-for="item in enrichedActivities"
      :key="item.activity.id"
      class="tool-activity-card"
      :class="[item.activity.kind, item.activity.status]"
      open
    >
      <summary class="tool-activity-header">
        <ChevronRight class="activity-chevron" :size="13" />
        <span class="tool-activity-icon" aria-hidden="true">
          <component :is="icon(item.activity)" :size="13" />
        </span>
        <span class="tool-activity-title">{{ item.activity.title }}</span>
        <span v-if="isFuzzy(item.activity)" class="fuzzy-badge">{{ tr(settingStore.language, "fuzzyMatch") }}</span>
        <span v-if="(operations || item.activity.preview) && item.edits.length" class="change-stats">
          <span class="added">+{{ item.stats.added }}</span>
          <span class="removed">-{{ item.stats.removed }}</span>
        </span>
        <span v-if="item.activity.status === 'running'" class="tool-activity-status">
          {{
            tr(
              settingStore.language,
              item.activity.toolName === "ask_user"
                ? "waitingAnswer"
                : item.activity.preview
                  ? "waitingApproval"
                  : "running",
            )
          }}
        </span>
        <span v-else-if="item.activity.status === 'error'" class="tool-activity-status error">{{ tr(settingStore.language, "failed") }}</span>
      </summary>

      <div v-if="(operations || item.activity.preview) && item.edits.length" class="operation-edits" :class="{ flat: item.edits.length === 1 }">
        <pre
          v-if="item.edits.length === 1"
          class="structured-diff flat"
        ><code><span
          v-for="(line, lineIndex) in item.edits[0].oldLines"
          :key="`old-${lineIndex}`"
          class="diff-line deletion"
        ><span class="diff-marker">-</span><span v-html="highlightLine(line, item.activity)" /></span><span
          v-for="(line, lineIndex) in item.edits[0].newLines"
          :key="`new-${lineIndex}`"
          class="diff-line addition"
        ><span class="diff-marker">+</span><span v-html="highlightLine(line, item.activity)" /></span></code></pre>
        <details
          v-for="(edit, index) in item.edits.length > 1 ? item.edits : []"
          :key="index"
          class="operation-edit"
          open
        >
          <summary>
            <ChevronRight class="edit-chevron" :size="12" />
            <span>{{ edit.label }}</span>
            <span class="change-stats edit-stats">
              <span class="added">+{{ edit.newLines.length }}</span>
              <span class="removed">-{{ edit.oldLines.length }}</span>
            </span>
          </summary>
          <pre class="structured-diff"><code><span
            v-for="(line, lineIndex) in edit.oldLines"
            :key="`old-${lineIndex}`"
            class="diff-line deletion"
          ><span class="diff-marker">-</span><span v-html="highlightLine(line, item.activity)" /></span><span
            v-for="(line, lineIndex) in edit.newLines"
            :key="`new-${lineIndex}`"
            class="diff-line addition"
          ><span class="diff-marker">+</span><span v-html="highlightLine(line, item.activity)" /></span></code></pre>
        </details>
      </div>
      <div v-else-if="item.activity.detail" class="tool-activity-detail">
        <Markdown :content="item.activity.detail" />
      </div>
      <div v-else-if="shouldShowResult(item.activity)" class="tool-activity-detail">
        <Markdown :content="formatResult(item.activity.result!)" />
      </div>
    </details>
  </div>
</template>

<script setup lang="ts">
import { computed, type Component } from "vue";
import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/common";
import {
  ChevronRight,
  FilePenLine,
  FilePlus2,
  FileX2,
  FolderSearch,
  MoveRight,
  Terminal,
  Wrench,
} from "@lucide/vue";
import Markdown from "@/components/chat/Markdown.vue";
import type { ToolActivity } from "@/types/chat";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";

const props = withDefaults(defineProps<{
  activities: ToolActivity[];
  operations?: boolean;
}>(), {
  operations: false,
});
const settingStore = useSettingStore();

type StructuredEdit = {
  label: string;
  oldLines: string[];
  newLines: string[];
};

const HIDE_RESULT_TOOLS = new Set([
  "read_file", "list_folder", "find_files", "search_files", "list_symbols", "fetch_url",
]);

const enrichedActivities = computed(() =>
  props.activities
    .filter((activity) => !(activity.toolName === "ask_user" && activity.status !== "running"))
    .map((activity) => {
      const edits = operationEdits(activity);
      return {
        activity,
        edits,
        stats: edits.reduce(
          (total, edit) => ({
            added: total.added + edit.newLines.length,
            removed: total.removed + edit.oldLines.length,
          }),
          { added: 0, removed: 0 },
        ),
      };
    }),
);

function operationEdits(activity: ToolActivity): StructuredEdit[] {
  const args = activity.arguments ?? {};
  if (activity.toolName === "replace_many_in_file" && Array.isArray(args.edits)) {
    return args.edits.map((value, index) => {
      const edit = value as Record<string, unknown>;
      return makeEdit(tr(settingStore.language, "edit", { count: index + 1 }), edit.old_string, edit.new_string);
    });
  }

  const preview = activity.preview;
  if (preview && (preview.oldText != null || preview.newText != null)) {
    const label =
      preview.kind === "create"
        ? tr(settingStore.language, "addContent")
        : preview.kind === "delete"
          ? tr(settingStore.language, "deleteContent")
          : tr(settingStore.language, "editContent");
    return [makeEdit(label, preview.oldText ?? "", preview.newText ?? "")];
  }

  if (activity.toolName === "replace_in_file") {
    return [makeEdit(tr(settingStore.language, "editContent"), args.old_string, args.new_string)];
  }
  if (activity.toolName === "write_file") {
    return [makeEdit(tr(settingStore.language, "addContent"), "", args.content)];
  }
  if (activity.kind === "delete") {
    const deleted = args.old_string ?? args.symbol ?? args.start_anchor ?? activity.detail ?? "";
    return [makeEdit(tr(settingStore.language, "deleteContent"), deleted, "")];
  }
  return [];
}

function makeEdit(label: string, oldValue: unknown, newValue: unknown): StructuredEdit {
  return {
    label,
    oldLines: toLines(oldValue),
    newLines: toLines(newValue),
  };
}

function toLines(value: unknown) {
  if (typeof value !== "string" || !value.length) return [];
  const lines = value.split(/\r?\n/);
  if (lines[lines.length - 1] === "") lines.pop();
  return lines;
}

const LANGUAGE_BY_EXTENSION: Record<string, string> = {
  c: "c", cc: "cpp", cpp: "cpp", cs: "csharp", css: "css", go: "go",
  html: "xml", java: "java", js: "javascript", json: "json", jsx: "javascript",
  kt: "kotlin", md: "markdown", php: "php", py: "python", rb: "ruby", rs: "rust",
  sh: "bash", sql: "sql", ts: "typescript", tsx: "typescript", vue: "xml",
  xml: "xml", yaml: "yaml", yml: "yaml",
};

function highlightLine(line: string, activity: ToolActivity) {
  const path = String(activity.arguments?.path ?? "");
  const extension = path.split(".").pop()?.toLowerCase() ?? "";
  const language = LANGUAGE_BY_EXTENSION[extension];
  const highlighted = language && hljs.getLanguage(language)
    ? hljs.highlight(line, { language }).value
    : hljs.highlightAuto(line).value;
  return DOMPurify.sanitize(highlighted);
}

function shouldShowResult(activity: ToolActivity) {
  return Boolean(activity.result && activity.status !== "running" && !HIDE_RESULT_TOOLS.has(activity.toolName));
}

function isFuzzy(activity: ToolActivity) {
  return /fuzzy/i.test(activity.title) || /fuzzy/i.test(activity.result ?? "");
}

function icon(activity: ToolActivity): Component {
  switch (activity.kind) {
    case "shell": return Terminal;
    case "create": return FilePlus2;
    case "edit": return FilePenLine;
    case "delete": return FileX2;
    case "move": return MoveRight;
    case "read": return FolderSearch;
    default: return Wrench;
  }
}

function formatResult(result: string) {
  return result.startsWith("```") ? result : `\`\`\`\n${result}\n\`\`\``;
}
</script>

<style scoped>
.tool-activity-list { display: flex; flex-direction: column; gap: 8px; width: 100%; margin-bottom: 10px; box-sizing: border-box; }
.tool-activity-list.operations { gap: 6px; }
.tool-activity-card { width: 100%; box-sizing: border-box; border: 1px solid color-mix(in srgb, var(--peek-border) 80%, transparent); border-radius: 7px; background: color-mix(in srgb, var(--peek-surface) 88%, transparent); overflow: hidden; }
.tool-activity-card.running { border-color: color-mix(in srgb, var(--peek-accent) 35%, var(--peek-border)); }
.tool-activity-card.error { border-color: color-mix(in srgb, #ef4444 40%, var(--peek-border)); }
.tool-activity-header { display: flex; align-items: center; gap: 7px; padding: 8px 10px; color: var(--peek-text); font-size: 12px; line-height: 1.4; cursor: pointer; list-style: none; user-select: none; }
.tool-activity-header::-webkit-details-marker, .operation-edit > summary::-webkit-details-marker { display: none; }
.activity-chevron, .edit-chevron { flex: none; color: var(--peek-faint); transition: transform 150ms ease; }
.tool-activity-card[open] > .tool-activity-header .activity-chevron, .operation-edit[open] > summary .edit-chevron { transform: rotate(90deg); }
.tool-activity-card[open] > .tool-activity-header { border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 60%, transparent); }
.tool-activity-icon { display: inline-flex; align-items: center; justify-content: center; flex: none; width: 20px; height: 20px; border-radius: 4px; background: color-mix(in srgb, var(--peek-accent) 12%, transparent); color: var(--peek-accent); }
.tool-activity-title { flex: 1; min-width: 0; overflow-wrap: anywhere; }
.fuzzy-badge { flex: none; font-size: 10px; padding: 1px 6px; border-radius: 999px; background: color-mix(in srgb, #eab308 22%, transparent); color: #eab308; font-weight: 650; }
.change-stats { display: inline-flex; align-items: center; gap: 5px; flex: none; font-family: var(--font-mono); font-size: 11px; font-weight: 650; }
.change-stats .added { color: #22c55e; }
.change-stats .removed { color: #ef4444; }
.tool-activity-status { flex: none; color: var(--peek-muted); font-size: 11px; }
.tool-activity-status.error { color: #f87171; }
.tool-activity-detail { padding: 8px 10px 10px; font-size: 12px; }
.tool-activity-detail :deep(pre) { margin: 0; max-height: 220px; overflow: auto; }
.operation-edits { display: flex; flex-direction: column; gap: 0; padding: 0; }
.operation-edits.flat { padding: 0; }
.operation-edit { overflow: hidden; border-top: 1px solid color-mix(in srgb, var(--peek-border) 70%, transparent); background: transparent; }
.operation-edit > summary { display: flex; align-items: center; gap: 5px; padding: 6px 10px; color: var(--peek-muted); font-size: 11px; font-weight: 600; cursor: pointer; list-style: none; }
.edit-stats { margin-left: auto; }
.structured-diff { max-height: 240px; margin: 0; overflow: auto; border-top: 1px solid var(--peek-border); background: color-mix(in srgb, var(--peek-input-bg) 82%, transparent); font-family: var(--font-mono); font-size: 11px; line-height: 1.55; }
.structured-diff.flat { border-top: 0; }
.structured-diff code { display: block; min-width: max-content; }
.diff-line { display: flex; min-width: 100%; padding: 0 9px 0 0; white-space: pre; }
.diff-marker { display: inline-block; flex: none; width: 22px; padding-left: 7px; user-select: none; }
.structured-diff :deep(.hljs-comment), .structured-diff :deep(.hljs-quote) { color: #7f8c98; font-style: italic; }
.structured-diff :deep(.hljs-keyword), .structured-diff :deep(.hljs-selector-tag), .structured-diff :deep(.hljs-type), .structured-diff :deep(.hljs-literal) { color: #c792ea; }
.structured-diff :deep(.hljs-string), .structured-diff :deep(.hljs-regexp), .structured-diff :deep(.hljs-attribute) { color: #addb67; }
.structured-diff :deep(.hljs-number), .structured-diff :deep(.hljs-symbol) { color: #f78c6c; }
.structured-diff :deep(.hljs-title), .structured-diff :deep(.hljs-section), .structured-diff :deep(.hljs-built_in) { color: #82aaff; }
.structured-diff :deep(.hljs-variable), .structured-diff :deep(.hljs-params) { color: #f07178; }
.diff-line.deletion { background: color-mix(in srgb, #ef4444 19%, transparent); color: color-mix(in srgb, #fecaca 88%, var(--peek-text)); }
.diff-line.addition { background: color-mix(in srgb, #22c55e 19%, transparent); color: color-mix(in srgb, #bbf7d0 88%, var(--peek-text)); }
:global([data-theme="light"]) .diff-line.deletion, :global([data-theme="cream"]) .diff-line.deletion { color: #991b1b; }
:global([data-theme="light"]) .diff-line.addition, :global([data-theme="cream"]) .diff-line.addition { color: #166534; }
.tool-activity-card.create .tool-activity-icon { background: color-mix(in srgb, #22c55e 15%, transparent); color: #22c55e; }
.tool-activity-card.edit .tool-activity-icon { background: color-mix(in srgb, #eab308 15%, transparent); color: #eab308; }
.tool-activity-card.delete .tool-activity-icon { background: color-mix(in srgb, #ef4444 15%, transparent); color: #ef4444; }
</style>
