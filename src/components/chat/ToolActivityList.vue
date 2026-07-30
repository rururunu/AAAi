<template>
  <div v-if="enrichedActivities.length" class="tool-activity-list" :class="{ operations, nested }">
    <section
      v-for="item in enrichedActivities"
      :key="item.activity.id"
      class="tool-activity-card"
      :class="[
        item.activity.kind,
        item.activity.status,
        { subagent: isSubagentTool(item.activity), 'subagent-running': isRunningSubagent(item.activity) },
      ]"
    >
      <div class="tool-activity-header">
        <button
          type="button"
          class="tool-activity-main"
          :aria-expanded="isExpanded(item.activity)"
          @click="toggleActivity(item.activity)"
        >
        <ChevronRight class="activity-chevron" :class="{ open: isExpanded(item.activity) }" :size="12" />
        <span class="tool-activity-icon" aria-hidden="true">
          <component :is="icon(item.activity)" :size="12" />
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
        </button>
        <button
          v-if="showInspectAction && isSubagentTool(item.activity) && !childAgentRows(item.activity).length"
          type="button"
          class="inspect-subagent-button"
          :aria-label="inspectLabel"
          :title="inspectLabel"
          @click.stop="emit('inspectSubagent', item.activity.id)"
        >
          <PanelRightOpen :size="13" />
        </button>
      </div>

      <div v-if="isSubagentTool(item.activity) && childAgentRows(item.activity).length" class="child-agent-rows">
        <button
          v-for="agent in childAgentRows(item.activity)"
          :key="agent.id"
          type="button"
          class="child-agent-row"
          :title="agent.prompt"
          @click.stop="emit('inspectSubagent', agent.id)"
        >
          <span class="tool-activity-icon" aria-hidden="true">
            <LoaderCircle v-if="agent.status === 'running'" class="child-agent-spinner" :size="12" />
            <Bot v-else :size="12" />
          </span>
          <span class="child-agent-title">{{ agent.title }}</span>
          <span v-if="agent.status === 'running'" class="tool-activity-status">{{ tr(settingStore.language, "running") }}</span>
          <span v-else-if="agent.status === 'error'" class="tool-activity-status error">{{ tr(settingStore.language, "failed") }}</span>
          <PanelRightOpen :size="13" class="child-agent-inspect" />
        </button>
      </div>

      <div v-if="isExpanded(item.activity) && (!isSubagentTool(item.activity) || showSubagentDetails)" class="tool-activity-body">
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

        <ToolActivityList
          v-if="childActivities(item.activity).length"
          class="subagent-activity-list"
          :activities="childActivities(item.activity)"
          :all-activities="activityPool"
          :show-inspect-action="showInspectAction"
          :show-subagent-details="showSubagentDetails"
          nested
          @inspect-subagent="emit('inspectSubagent', $event)"
        />
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, type Component } from "vue";
import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/common";
import {
  ChevronRight,
  Bot,
  FilePenLine,
  FilePlus2,
  FileX2,
  FolderSearch,
  MoveRight,
  Terminal,
  LoaderCircle,
  PanelRightOpen,
  Wrench,
} from "@lucide/vue";
import Markdown from "@/components/chat/Markdown.vue";
import type { ToolActivity } from "@/types/chat";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";

const props = withDefaults(defineProps<{
  activities: ToolActivity[];
  allActivities?: ToolActivity[];
  operations?: boolean;
  nested?: boolean;
  showInspectAction?: boolean;
  showSubagentDetails?: boolean;
}>(), {
  operations: false,
  nested: false,
  showInspectAction: true,
  showSubagentDetails: false,
});
const emit = defineEmits<{ inspectSubagent: [activityId: string] }>();
const settingStore = useSettingStore();
const inspectLabel = computed(() => tr(settingStore.language, "subagent.view"));
const expandedIds = ref(new Set<string>());
const previousStatuses = new Map<string, ToolActivity["status"]>();

const activityPool = computed(() => props.allActivities ?? props.activities);

type StructuredEdit = {
  label: string;
  oldLines: string[];
  newLines: string[];
};

type ChildAgentRow = {
  id: string;
  title: string;
  prompt: string;
  status: ToolActivity["status"];
};

const HIDE_RESULT_TOOLS = new Set([
  "read_file", "list_folder", "find_files", "search_files", "list_symbols", "fetch_url",
]);

const enrichedActivities = computed(() =>
  props.activities
    .filter((activity) => {
      if (activity.kind === "read" && !props.nested) return false;
      return !(activity.toolName === "ask_user" && activity.status !== "running");
    })
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
  if (isRunningSubagent(activity)) return LoaderCircle;
  if (isSubagentTool(activity)) return Bot;
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

const SUBAGENT_TOOLS = new Set([
  "run_subagent",
  "run_readonly_subagent",
  "run_parallel_subagents",
  "run_skill",
  "run_readonly_skill",
  "explore_codebase",
  "research_topic",
  "review_code",
  "review_security",
  "generate_word",
]);

function isSubagentTool(activity: ToolActivity) {
  return SUBAGENT_TOOLS.has(activity.toolName);
}

function isRunningSubagent(activity: ToolActivity) {
  return isSubagentTool(activity) && activity.status === "running";
}

function childActivities(activity: ToolActivity) {
  return activityPool.value.filter((candidate) => candidate.parentActivityId === activity.id);
}

function childAgentRows(activity: ToolActivity): ChildAgentRow[] {
  if (!isSubagentTool(activity)) return [];
  const args = activity.arguments ?? {};
  const prompts = Array.isArray(args.tasks)
    ? args.tasks.map((value) => {
        if (typeof value !== "object" || value == null) return "";
        return String((value as Record<string, unknown>).prompt ?? "").trim();
      })
    : [args.prompt, args.task, args.description]
        .filter((value): value is string => typeof value === "string" && Boolean(value.trim()))
        .slice(0, 1)
        .map((value) => value.trim());
  const groups = new Map<string, ToolActivity[]>();
  for (const child of childActivities(activity)) {
    const key = child.subagentId ?? "default";
    const group = groups.get(key) ?? [];
    group.push(child);
    groups.set(key, group);
  }
  const grouped = [...groups.values()];
  const count = Math.max(prompts.length, grouped.length, 1);
  return Array.from({ length: count }, (_, index) => {
    const prompt = prompts[index] ?? prompts[0] ?? activity.title;
    const children = grouped[index] ?? [];
    const status = children.some((child) => child.status === "error")
      ? "error"
      : children.some((child) => child.status === "running")
        ? "running"
        : activity.status;
    return {
      id: `${activity.id}:${index}`,
      title: shortTaskTitle(prompt, index),
      prompt,
      status,
    };
  });
}

function shortTaskTitle(prompt: string, index: number) {
  const lines = prompt.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const heading = lines.find((line) => /^#{1,6}\s+/.test(line));
  const source = heading ?? lines[0] ?? "";
  const cleaned = source
    .replace(/^#{1,6}\s+/, "")
    .replace(/^(?:任务|task|assignment)\s*[:：-]\s*/i, "")
    .replace(/[`*_~]/g, "")
    .trim();
  const prefix = tr(settingStore.language, "subagent.numbered", { count: index + 1 });
  const title = cleaned ? `${prefix} · ${cleaned}` : prefix;
  return title.length > 72 ? `${title.slice(0, 71)}...` : title;
}

function isExpanded(activity: ToolActivity) {
  return expandedIds.value.has(activity.id);
}

function setExpanded(activityId: string, expanded: boolean) {
  const next = new Set(expandedIds.value);
  if (expanded) next.add(activityId);
  else next.delete(activityId);
  expandedIds.value = next;
}

function toggleActivity(activity: ToolActivity) {
  if (isSubagentTool(activity) && !props.showSubagentDetails) {
    return;
  }
  setExpanded(activity.id, !isExpanded(activity));
}

function formatResult(result: string) {
  return result.startsWith("```") ? result : `\`\`\`\n${result}\n\`\`\``;
}

watch(
  () => activityPool.value.map((activity) => `${activity.id}:${activity.status}`).join("|"),
  () => {
    for (const activity of activityPool.value) {
      const previous = previousStatuses.get(activity.id);
      if (
        previous === undefined &&
        (activity.status === "running" || activity.status === "error") &&
        (!isSubagentTool(activity) || props.showSubagentDetails)
      ) {
        setExpanded(activity.id, true);
      } else if (previous === "running" && activity.status === "done") {
        setExpanded(activity.id, false);
      } else if (activity.status === "error" && previous !== "error") {
        setExpanded(activity.id, true);
      }
      previousStatuses.set(activity.id, activity.status);
    }
  },
  { immediate: true },
);
</script>

<style scoped>
.tool-activity-list { display: flex; flex-direction: column; gap: 3px; width: 100%; margin-bottom: 0; box-sizing: border-box; }
.tool-activity-list.operations { gap: 3px; }
.tool-activity-list.nested {
  gap: 2px;
  margin: 2px 8px 8px 28px;
  width: calc(100% - 36px);
  padding-left: 8px;
  border-left: 1px solid color-mix(in srgb, var(--peek-border) 82%, transparent);
}
.tool-activity-card {
  width: 100%;
  box-sizing: border-box;
  border: 0;
  border-radius: 6px;
  background: transparent;
  overflow: hidden;
}
.tool-activity-card.running {
  background: color-mix(in srgb, var(--peek-accent) 7%, transparent);
}
.tool-activity-card.subagent {
  margin: 3px 0;
  border: 0;
  background: transparent;
}
.tool-activity-card.subagent-running {
  background: transparent;
  box-shadow: none;
}
.tool-activity-card.subagent > .tool-activity-header {
  min-height: 34px;
  padding: 6px 8px;
  color: var(--peek-text);
  font-weight: 600;
}
.tool-activity-card.subagent > .tool-activity-body > .tool-activity-detail {
  padding: 4px 12px 10px 36px;
  color: color-mix(in srgb, var(--peek-text) 84%, var(--peek-muted));
  line-height: 1.55;
}
.tool-activity-card.subagent-running > .tool-activity-header .tool-activity-icon {
  background: transparent;
}
.tool-activity-card.subagent-running > .tool-activity-header .tool-activity-icon :deep(svg) {
  animation: subagent-tool-spin 900ms linear infinite;
}
.tool-activity-card.error {
  background: color-mix(in srgb, var(--destructive) 8%, transparent);
}
.tool-activity-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  color: var(--peek-muted);
  font-size: 11px;
  line-height: 1.35;
  width: 100%;
  background: transparent;
  border-radius: 6px;
  transition: background 120ms ease, color 120ms ease;
}
.tool-activity-main {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
  user-select: none;
}
.tool-activity-header:hover {
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
  color: var(--peek-text);
}
.operation-edit > summary::-webkit-details-marker { display: none; }
.activity-chevron, .edit-chevron { flex: none; color: var(--peek-faint); transition: transform 150ms ease; }
.activity-chevron.open, .operation-edit[open] > summary .edit-chevron { transform: rotate(90deg); }
.tool-activity-main[aria-expanded="true"] {
  color: var(--peek-text);
  border-bottom: 0;
}
.tool-activity-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  width: 16px;
  height: 16px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
  color: var(--peek-accent);
}
.tool-activity-title { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.fuzzy-badge { flex: none; font-size: 9px; padding: 0 5px; border-radius: 999px; background: color-mix(in srgb, #eab308 22%, transparent); color: #eab308; font-weight: 650; }
.change-stats { display: inline-flex; align-items: center; gap: 4px; flex: none; font-family: var(--font-mono); font-size: 10px; font-weight: 650; }
.change-stats .added { color: #22c55e; }
.change-stats .removed { color: var(--destructive); }
.tool-activity-status { flex: none; color: var(--peek-muted); font-size: 10px; }
.tool-activity-status.error { color: var(--destructive); }
.inspect-subagent-button { flex: none; width: 23px; height: 23px; display: inline-flex; align-items: center; justify-content: center; padding: 0; border: 0; border-radius: 4px; color: var(--peek-muted); background: transparent; cursor: pointer; }
.inspect-subagent-button:hover { color: var(--peek-accent); background: color-mix(in srgb, var(--peek-accent) 12%, transparent); }
.child-agent-rows { display: flex; flex-direction: column; gap: 2px; margin: 0 6px 5px 28px; padding-left: 8px; border-left: 1px solid color-mix(in srgb, var(--peek-border) 78%, transparent); }
.child-agent-row { width: 100%; min-width: 0; min-height: 29px; display: flex; align-items: center; gap: 6px; padding: 3px 6px; border: 0; border-radius: 5px; background: transparent; color: var(--peek-muted); text-align: left; cursor: pointer; }
.child-agent-row:hover { color: var(--peek-text); background: color-mix(in srgb, var(--peek-text) 5%, transparent); }
.child-agent-title { flex: 1; min-width: 0; overflow: hidden; font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
.child-agent-inspect { flex: none; color: var(--peek-faint); }
.child-agent-row:hover .child-agent-inspect { color: var(--peek-accent); }
.child-agent-spinner { animation: subagent-tool-spin 900ms linear infinite; }
.tool-activity-detail {
  padding: 2px 6px 8px 28px;
  font-size: 11px;
  color: var(--peek-muted);
}
.tool-activity-detail :deep(pre) { margin: 0; max-height: 180px; overflow: auto; border-radius: 6px; }
.operation-edits { display: flex; flex-direction: column; gap: 0; padding: 0 0 4px; }
.operation-edits.flat { padding: 0 0 4px; }
.operation-edit { overflow: hidden; border-top: 0; background: transparent; margin: 0 6px 0 28px; border-radius: 6px; }
.operation-edit > summary { display: flex; align-items: center; gap: 5px; padding: 4px 6px; color: var(--peek-muted); font-size: 10px; font-weight: 600; cursor: pointer; list-style: none; border-radius: 5px; }
.operation-edit > summary:hover { background: color-mix(in srgb, var(--peek-text) 4%, transparent); }
.edit-stats { margin-left: auto; }
.structured-diff { max-height: 200px; margin: 0; overflow: auto; border: 0; border-radius: 6px; background: color-mix(in srgb, var(--peek-input-bg) 82%, transparent); font-family: var(--font-mono); font-size: 11px; line-height: 1.55; }
.structured-diff.flat { margin: 0 6px 6px 28px; border-radius: 6px; }
.structured-diff code { display: block; min-width: max-content; }
.diff-line { display: flex; min-width: 100%; padding: 0 9px 0 0; white-space: pre; }
.diff-marker { display: inline-block; flex: none; width: 22px; padding-left: 7px; user-select: none; }
.structured-diff :deep(.hljs-comment), .structured-diff :deep(.hljs-quote) { color: #7f8c98; font-style: italic; }
.structured-diff :deep(.hljs-keyword), .structured-diff :deep(.hljs-selector-tag), .structured-diff :deep(.hljs-type), .structured-diff :deep(.hljs-literal) { color: #c792ea; }
.structured-diff :deep(.hljs-string), .structured-diff :deep(.hljs-regexp), .structured-diff :deep(.hljs-attribute) { color: #addb67; }
.structured-diff :deep(.hljs-number), .structured-diff :deep(.hljs-symbol) { color: #f78c6c; }
.structured-diff :deep(.hljs-title), .structured-diff :deep(.hljs-section), .structured-diff :deep(.hljs-built_in) { color: #82aaff; }
.structured-diff :deep(.hljs-variable), .structured-diff :deep(.hljs-params) { color: #f07178; }
.diff-line.deletion { background: color-mix(in srgb, var(--destructive) 19%, transparent); color: color-mix(in srgb, #fecaca 88%, var(--peek-text)); }
.diff-line.addition { background: color-mix(in srgb, #22c55e 19%, transparent); color: color-mix(in srgb, #bbf7d0 88%, var(--peek-text)); }
:global([data-theme="light"]) .diff-line.deletion, :global([data-theme="cream"]) .diff-line.deletion { color: #991b1b; }
:global([data-theme="light"]) .diff-line.addition, :global([data-theme="cream"]) .diff-line.addition { color: #166534; }
.tool-activity-card.create .tool-activity-icon { background: color-mix(in srgb, #22c55e 15%, transparent); color: #22c55e; }
.tool-activity-card.edit .tool-activity-icon { background: color-mix(in srgb, #eab308 15%, transparent); color: #eab308; }
.tool-activity-card.delete .tool-activity-icon { background: color-mix(in srgb, var(--destructive) 15%, transparent); color: var(--destructive); }
@keyframes subagent-tool-spin { to { transform: rotate(360deg); } }
</style>
