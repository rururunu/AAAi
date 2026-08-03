<template>
  <div v-if="segments.length" class="agent-work" :class="{ 'has-running-subagent': hasRunningSubagent }">
    <template v-for="segment in segments" :key="segment.id">
      <ReasoningBlock
        v-if="segment.type === 'reasoning'"
        :reasoning="segment.content"
        :streaming="streaming && segment.id === lastSegmentId"
        :language="language"
        embedded
      />

      <ToolActivityList
        v-else-if="segment.type === 'inline'"
        :activities="segment.activities"
        :all-activities="visibleActivities"
        :operations="segment.operations"
        :cards-collapsed="false"
        @inspect-subagent="emit('inspectSubagent', $event)"
      />

      <!-- Process details follow the timeline (Cursor-style), not dumped at the end. -->
      <section v-else class="agent-work-details">
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
import ReasoningBlock from "@/components/chat/ReasoningBlock.vue";
import ToolActivityList from "@/components/chat/ToolActivityList.vue";
import type { ChatMessage, ToolActivity } from "@/types/chat";
import type { AgentWorkDisplay, AppLanguage } from "@/types/setting";
import { tr } from "@/services/i18n";

const props = withDefaults(defineProps<{
  message: ChatMessage;
  language?: AppLanguage;
  showReasoning?: boolean;
  /** detailed = shell/diff inline; compact = fold into process details. */
  displayMode?: AgentWorkDisplay;
}>(), {
  displayMode: "detailed",
});
const emit = defineEmits<{ inspectSubagent: [activityId: string] }>();

type TimelineSegment =
  | { type: "reasoning"; id: string; content: string }
  | {
      type: "inline" | "process";
      id: string;
      activities: ToolActivity[];
      operations: boolean;
    };

const SHOWCASE_KINDS = new Set(["shell", "create", "edit", "delete", "move"]);
const TASK_LIST_TOOLS = new Set(["update_tasks", "todo_write"]);
const SUBAGENT_TOOLS = new Set([
  "run_subagent",
  "run_parallel_subagents",
  "run_skill",
  "explore_codebase",
  "research_topic",
  "review_code",
  "review_security",
  "generate_word",
]);

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
    (activity) =>
      activity.status === "running" && SUBAGENT_TOOLS.has(activity.toolName),
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
  if (
    last &&
    last.type === kind &&
    last.operations === operations
  ) {
    last.activities.push(activity);
    return;
  }
  segments.push({
    type: kind,
    id: `${kind}-${activity.id}`,
    activities: [activity],
    operations,
  });
}

/** Single chronological stream — process-detail chunks interleaved like Cursor. */
const segments = computed<TimelineSegment[]>(() => {
  const out: TimelineSegment[] = [];
  const timeline = props.message.workTimeline ?? [];
  const seen = new Set<string>();

  if (timeline.length) {
    for (const item of timeline) {
      if (item.type === "reasoning") {
        if (props.showReasoning !== false && item.content.trim()) {
          out.push(item);
        }
        continue;
      }
      const activity = activityById.value.get(item.toolActivityId);
      if (!activity) continue;
      seen.add(activity.id);
      pushActivity(out, activity);
    }
  } else {
    if (props.showReasoning !== false && props.message.reasoning?.trim()) {
      out.push({
        type: "reasoning",
        id: "stream-reasoning",
        content: props.message.reasoning ?? "",
      });
    }
  }

  for (const activity of topLevelActivities.value) {
    if (seen.has(activity.id)) continue;
    pushActivity(out, activity);
  }
  return out;
});

const lastSegmentId = computed(() => segments.value[segments.value.length - 1]?.id);
const panelLabel = computed(() => tr(props.language, "processSummary"));

function processHeadline(segment: Extract<TimelineSegment, { type: "process" }>) {
  const titles = segment.activities
    .map((activity) => activity.title?.trim())
    .filter((title): title is string => Boolean(title));
  if (titles.length === 1) return titles[0]!;
  if (titles.length > 1 && titles.length <= 3) return titles.join(" · ");
  if (titles.length > 3) {
    return `${titles[0]} · ${tr(props.language, "toolCount", { count: titles.length })}`;
  }
  return `${panelLabel.value} · ${tr(props.language, "toolCount", { count: segment.activities.length })}`;
}

function isProcessOpen(id: string) {
  return processOpen.get(id) ?? false;
}

function toggleProcess(id: string) {
  userToggledProcess.add(id);
  processOpen.set(id, !isProcessOpen(id));
}

/** Expand process only when it has active showcase work (or ask_user). Reads stay one-line. */
function shouldAutoExpandProcess(segment: Extract<TimelineSegment, { type: "process" }>) {
  if (waitingForAskUser.value) return true;
  return segment.activities.some(
    (activity) =>
      activity.status === "running" &&
      (SHOWCASE_KINDS.has(activity.kind) || TASK_LIST_TOOLS.has(activity.toolName)),
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
  transition: color 140ms ease, background 140ms ease;
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
