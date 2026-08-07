<template>
  <div
    v-if="segments.length"
    class="agent-work"
    :class="{ 'has-running-subagent': hasRunningSubagent }"
  >
    <template v-for="segment in segments" :key="segment.id">
      <ReasoningBlock
        v-if="segment.type === 'reasoning'"
        :reasoning="segment.content"
        :streaming="streaming && segment.id === lastSegmentId"
        :language="language"
        embedded
      />

      <Markdown
        v-else-if="segment.type === 'content'"
        :content="segment.content"
        class="agent-work-content"
        @preview-image="emit('previewImage', $event)"
      />

      <ToolActivityList
        v-else-if="segment.type === 'inline'"
        :activities="segment.activities"
        :all-activities="visibleActivities"
        :operations="segment.operations"
        :cards-collapsed="false"
        @inspect-subagent="emit('inspectSubagent', $event)"
      />

      <!-- Process details: collapsible summary for multi-step work. -->
      <ToolActivityList
        v-else-if="segment.type === 'process' && !processSegmentCollapsible(segment)"
        :activities="segment.activities"
        :all-activities="visibleActivities"
        :operations="segment.operations"
        flat
        @inspect-subagent="emit('inspectSubagent', $event)"
      />

      <section v-else-if="segment.type === 'process'" class="agent-work-details">
        <button
          type="button"
          class="agent-work-toggle"
          :aria-expanded="isProcessOpen(segment.id)"
          @click="toggleProcess(segment.id)"
        >
          <ChevronRight
            class="agent-work-chevron"
            :class="{ open: isProcessOpen(segment.id) }"
            :size="12"
          />
          <span class="agent-work-label">{{ processHeadline(segment) }}</span>
        </button>
        <div v-if="isProcessOpen(segment.id)" class="agent-work-body">
          <ToolActivityList
            :activities="segment.activities"
            :all-activities="visibleActivities"
            :operations="segment.operations"
            :cards-collapsed="displayMode === 'compact'"
            flat
            @inspect-subagent="emit('inspectSubagent', $event)"
          />
        </div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ChevronRight } from "@lucide/vue";
import { computed, reactive, watch } from "vue";
import Markdown from "@/components/chat/Markdown.vue";
import ReasoningBlock from "@/components/chat/ReasoningBlock.vue";
import ToolActivityList from "@/components/chat/ToolActivityList.vue";
import type { ChatMessage, ToolActivity } from "@/types/chat";
import type { AgentWorkDisplay, AppLanguage } from "@/types/setting";
import { SUBAGENT_TOOLS } from "@/services/chat/subagentTools";
import {
  isProcessSegmentCollapsible,
  summarizeProcessActivities,
} from "@/services/chat/toolActivityDisplay";

const props = withDefaults(
  defineProps<{
    message: ChatMessage;
    language?: AppLanguage;
    showReasoning?: boolean;
    /** detailed = shell/diff inline; compact = fold into process details. */
    displayMode?: AgentWorkDisplay;
    /** Content is a special-cased marker (e.g. "configure a provider") rendered
     * elsewhere, so skip showing it as regular inline text here. */
    suppressContent?: boolean;
  }>(),
  {
    displayMode: "detailed",
  },
);
const emit = defineEmits<{
  inspectSubagent: [activityId: string];
  previewImage: [source: string];
}>();

type TimelineSegment =
  | { type: "reasoning"; id: string; content: string }
  | { type: "content"; id: string; content: string }
  | { type: "inline"; id: string; activities: ToolActivity[]; operations: boolean }
  | { type: "process"; id: string; activities: ToolActivity[]; operations: boolean };

const SHOWCASE_KINDS = new Set(["shell", "create", "edit", "delete", "move"]);
const TASK_LIST_TOOLS = new Set(["update_tasks", "todo_write"]);

const processOpen = reactive(new Map<string, boolean>());
const userToggledProcess = reactive(new Set<string>());

const streaming = computed(
  () => props.message.status === "pending" || props.message.status === "streaming",
);
const waitingForAskUser = computed(
  () =>
    props.message.toolActivities?.some(
      (activity) => activity.toolName === "ask_user" && activity.status === "running",
    ) ?? false,
);
const visibleActivities = computed(() =>
  (props.message.toolActivities ?? []).filter(
    (activity) => !(activity.toolName === "ask_user" && activity.status !== "running"),
  ),
);
const activityById = computed(
  () =>
    new Map(
      visibleActivities.value
        .filter((activity) => !activity.parentActivityId)
        .map((activity) => [activity.id, activity]),
    ),
);
const topLevelActivities = computed(() =>
  visibleActivities.value.filter((activity) => !activity.parentActivityId),
);

const hasRunningSubagent = computed(() =>
  topLevelActivities.value.some(
    (activity) => activity.status === "running" && SUBAGENT_TOOLS.has(activity.toolName),
  ),
);

/** Task lists + (in detailed mode) shell/file edits stay in the open stream. */
function isInlineActivity(activity: ToolActivity): boolean {
  if (TASK_LIST_TOOLS.has(activity.toolName)) return true;
  if (props.displayMode === "compact") return false;
  return SHOWCASE_KINDS.has(activity.kind);
}

function isOperationsActivity(activity: ToolActivity): boolean {
  return SHOWCASE_KINDS.has(activity.kind) && activity.kind !== "shell";
}

function segmentKind(activity: ToolActivity): "inline" | "process" {
  return isInlineActivity(activity) ? "inline" : "process";
}

function pushActivity(segments: TimelineSegment[], activity: ToolActivity) {
  const kind = segmentKind(activity);
  const operations = isOperationsActivity(activity);
  const last = segments[segments.length - 1];
  if (last && last.type === kind && last.operations === operations) {
    last.activities.push(activity);
    return;
  }
  const base = {
    id: `${kind}-${activity.id}`,
    activities: [activity],
    operations,
  };
  segments.push(kind === "inline" ? { type: "inline", ...base } : { type: "process", ...base });
}

type TextSegment = Extract<TimelineSegment, { type: "reasoning" | "content" }>;

function isTextSegment(segment: TimelineSegment): segment is TextSegment {
  return segment.type === "reasoning" || segment.type === "content";
}

/**
 * Append any part of `finalText` that isn't already covered by the matching
 * segments in `out`. Keeps the reply visible even when the timeline is
 * missing, partial, or stale (persisted history predating this feature, a
 * reply delivered in one lump instead of incremental deltas, etc.) without
 * mutating the reactive timeline items that were copied in.
 */
function reconcileTrailingText(
  out: TimelineSegment[],
  kind: "reasoning" | "content",
  finalText: string | undefined,
) {
  const finalValue = finalText ?? "";
  if (!finalValue) return;
  let accumulated = "";
  for (const segment of out) {
    if (isTextSegment(segment) && segment.type === kind) accumulated += segment.content;
  }
  if (finalValue.length <= accumulated.length) return;
  const missing = finalValue.slice(accumulated.length);
  const last = out[out.length - 1];
  if (last && isTextSegment(last) && last.type === kind) {
    last.content += missing;
  } else {
    out.push({
      type: kind,
      id: `${kind}-final-${out.length}`,
      content: missing,
    } as TimelineSegment);
  }
}

/**
 * Once the turn is finished, fold every interleaved reasoning chunk into a
 * single collapsed "思考过程" entry so the finished message stays short.
 * Live turns keep chronological interleaving for follow-along.
 */
function coalesceCompletedReasoning(out: TimelineSegment[]): TimelineSegment[] {
  if (streaming.value) return out;
  const fromMessage = props.message.reasoning?.trim() ?? "";
  const fromSegments = out
    .filter((segment): segment is Extract<TimelineSegment, { type: "reasoning" }> => {
      return segment.type === "reasoning";
    })
    .map((segment) => segment.content)
    .join("");
  const combined = fromMessage || fromSegments;
  const withoutReasoning = out.filter((segment) => segment.type !== "reasoning");
  if (!combined || props.showReasoning === false) return withoutReasoning;

  const firstIdx = out.findIndex((segment) => segment.type === "reasoning");
  const block: TimelineSegment = {
    type: "reasoning",
    id: `${props.message.id}-reasoning-completed`,
    content: combined,
  };
  if (firstIdx <= 0) return [block, ...withoutReasoning];

  const before = out.slice(0, firstIdx).filter((segment) => segment.type !== "reasoning");
  const after = out.slice(firstIdx).filter((segment) => segment.type !== "reasoning");
  return [...before, block, ...after];
}

/** Single chronological stream with process-detail chunks interleaved. */
const segments = computed<TimelineSegment[]>(() => {
  let out: TimelineSegment[] = [];
  const timeline = props.message.workTimeline ?? [];
  const seen = new Set<string>();

  for (const item of timeline) {
    if (item.type === "content" && props.suppressContent) continue;
    if (item.type === "reasoning" || item.type === "content") {
      if (item.content.trim()) {
        // Copy rather than reuse the store's item so reconciliation below
        // never mutates reactive state held elsewhere.
        out.push({ ...item });
      }
      continue;
    }
    const activity = activityById.value.get(item.toolActivityId);
    if (!activity) continue;
    seen.add(activity.id);
    pushActivity(out, activity);
  }

  reconcileTrailingText(out, "reasoning", props.message.reasoning);
  if (!props.suppressContent) {
    reconcileTrailingText(out, "content", props.message.content);
  }

  if (props.showReasoning === false) {
    out = out.filter((segment) => segment.type !== "reasoning");
  }

  for (const activity of topLevelActivities.value) {
    if (seen.has(activity.id)) continue;
    pushActivity(out, activity);
  }
  return coalesceCompletedReasoning(out);
});

const lastSegmentId = computed(() => segments.value[segments.value.length - 1]?.id);

function processSegmentCollapsible(segment: Extract<TimelineSegment, { type: "process" }>) {
  return isProcessSegmentCollapsible(segment.activities, visibleActivities.value);
}

function processHeadline(segment: Extract<TimelineSegment, { type: "process" }>) {
  const language = props.language ?? "zh-CN";
  return summarizeProcessActivities(segment.activities, language);
}

function isProcessOpen(id: string) {
  return processOpen.get(id) ?? false;
}

function toggleProcess(id: string) {
  userToggledProcess.add(id);
  processOpen.set(id, !isProcessOpen(id));
}

/** Collapsed by default; only expand while showcase work is actively running. */
function shouldAutoExpandProcess(segment: Extract<TimelineSegment, { type: "process" }>) {
  if (waitingForAskUser.value) return true;
  return segment.activities.some(
    (activity) => activity.status === "running" && SHOWCASE_KINDS.has(activity.kind),
  );
}

watch(
  () =>
    [
      props.message.status,
      props.message.askUserAnswer?.length ?? 0,
      streaming.value,
      segments.value
        .map((segment) =>
          segment.type === "process"
            ? `${segment.id}:${segment.activities.map((a) => a.status).join(",")}`
            : segment.id,
        )
        .join("|"),
    ] as const,
  () => {
    for (const segment of segments.value) {
      if (segment.type !== "process") continue;
      if (userToggledProcess.has(segment.id)) continue;
      processOpen.set(segment.id, shouldAutoExpandProcess(segment));
    }
  },
  { immediate: true },
);
</script>

<style scoped>
.agent-work {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
  margin-bottom: 8px;
  box-sizing: border-box;
}

.agent-work :deep(.shell-terminal-card),
.agent-work :deep(.file-diff-card),
.agent-work :deep(.task-list-card.embedded) {
  margin-left: 0;
  margin-right: 0;
}

.agent-work-content :deep(> *:first-child) {
  margin-top: 0;
}

.agent-work-content :deep(> *:last-child) {
  margin-bottom: 0;
}

.agent-work-details {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
}

.agent-work-toggle {
  display: flex;
  align-items: center;
  gap: 5px;
  width: 100%;
  padding: 3px 2px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--peek-muted);
  font-size: 11px;
  font-weight: 550;
  line-height: 1.35;
  cursor: pointer;
  text-align: left;
  transition:
    color 140ms ease,
    background 140ms ease;
}

.agent-work-toggle:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
}

.agent-work.has-running-subagent .agent-work-toggle {
  color: var(--peek-text);
}

.agent-work-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 160ms ease;
}

.agent-work-chevron.open {
  transform: rotate(90deg);
}

.agent-work-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.agent-work-body {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 2px 0 2px 2px;
  border-left: 1.5px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
  margin-left: 5px;
  padding-left: 10px;
}

.agent-work-body :deep(.tool-activity-list),
.agent-work :deep(.tool-activity-list),
.agent-work :deep(.reasoning-block) {
  margin-bottom: 0;
}
</style>
