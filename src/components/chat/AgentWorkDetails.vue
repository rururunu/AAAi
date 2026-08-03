<template>
  <div v-if="hasWork" class="agent-work" :class="{ 'has-running-subagent': hasRunningSubagent }">
    <button
      type="button"
      class="agent-work-toggle"
      :aria-expanded="panelExpanded"
      @click="togglePanel"
    >
      <ChevronRight class="agent-work-chevron" :class="{ open: panelExpanded }" :size="12" />
      <span>{{ panelLabel }}</span>
      <span v-if="!panelExpanded" class="agent-work-meta">{{ panelMeta }}</span>
    </button>

    <div v-if="panelExpanded" class="agent-work-body">
      <template v-if="timelineGroups.length">
        <template v-for="group in timelineGroups" :key="group.id">
          <ReasoningBlock
            v-if="group.type === 'reasoning'"
            :reasoning="group.content"
            :streaming="streaming && group.id === lastTimelineId"
            :language="language"
            embedded
          />
          <ToolActivityList
            v-else
            :activities="group.activities"
            :all-activities="visibleActivities"
            :operations="group.type === 'operations'"
            @inspect-subagent="emit('inspectSubagent', $event)"
          />
        </template>
      </template>

      <template v-else>
        <ReasoningBlock
          v-if="hasReasoning"
          :reasoning="message.reasoning ?? ''"
          :streaming="streaming"
          :language="language"
          embedded
        />
        <ToolActivityList
          v-if="executionActivities.length"
          :activities="executionActivities"
          :all-activities="visibleActivities"
          @inspect-subagent="emit('inspectSubagent', $event)"
        />
        <ToolActivityList
          v-if="operationActivities.length"
          :activities="operationActivities"
          :all-activities="visibleActivities"
          operations
          @inspect-subagent="emit('inspectSubagent', $event)"
        />
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ChevronRight } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import ReasoningBlock from "@/components/chat/ReasoningBlock.vue";
import ToolActivityList from "@/components/chat/ToolActivityList.vue";
import type { ChatMessage, ToolActivity } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";
import { tr } from "@/services/i18n";

const props = defineProps<{
  message: ChatMessage;
  language?: AppLanguage;
  showReasoning?: boolean;
}>();
const emit = defineEmits<{ inspectSubagent: [activityId: string] }>();

type TimelineGroup =
  | { type: "reasoning"; id: string; content: string }
  | { type: "execution" | "operations"; id: string; activities: ToolActivity[] };

const FILE_OPERATION_KINDS = new Set(["create", "edit", "delete", "move"]);
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
const userToggled = ref(false);
const panelExpanded = ref(true);

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
    (activity) =>
      (activity.kind !== "read" || Boolean(activity.parentActivityId)) &&
      !(activity.toolName === "ask_user" && activity.status !== "running"),
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
const operationActivities = computed(() =>
  topLevelActivities.value.filter((activity) => FILE_OPERATION_KINDS.has(activity.kind)),
);
const executionActivities = computed(() =>
  topLevelActivities.value.filter((activity) => !FILE_OPERATION_KINDS.has(activity.kind)),
);
const hasRunningSubagent = computed(() =>
  topLevelActivities.value.some(
    (activity) =>
      activity.status === "running" &&
      SUBAGENT_TOOLS.has(activity.toolName),
  ),
);
const hasReasoning = computed(
  () => props.showReasoning !== false && Boolean(props.message.reasoning?.trim()),
);
const hasExecutionWork = computed(
  () => hasReasoning.value || executionActivities.value.length > 0,
);

const timelineGroups = computed<TimelineGroup[]>(() => {
  const groups: TimelineGroup[] = [];
  for (const item of props.message.workTimeline ?? []) {
    if (item.type === "reasoning") {
      if (props.showReasoning !== false && item.content.trim()) groups.push(item);
      continue;
    }
    const activity = activityById.value.get(item.toolActivityId);
    if (!activity) continue;
    const type = FILE_OPERATION_KINDS.has(activity.kind) ? "operations" : "execution";
    const last = groups[groups.length - 1];
    if (last?.type === type) {
      last.activities.push(activity);
    } else {
      groups.push({ type, id: `group-${item.id}`, activities: [activity] });
    }
  }
  return groups;
});

const lastTimelineId = computed(() => timelineGroups.value[timelineGroups.value.length - 1]?.id);
const hasWork = computed(
  () =>
    timelineGroups.value.length > 0 ||
    hasExecutionWork.value ||
    operationActivities.value.length > 0,
);

const reasoningCount = computed(() => {
  if (timelineGroups.value.length) {
    return timelineGroups.value.filter((group) => group.type === "reasoning").length;
  }
  return hasReasoning.value ? 1 : 0;
});

const toolCount = computed(() => topLevelActivities.value.length);

const panelLabel = computed(() => tr(props.language, "processSummary"));

const panelMeta = computed(() => {
  const parts: string[] = [];
  if (reasoningCount.value) {
    parts.push(
      tr(props.language, "thinkingCount", { count: reasoningCount.value }),
    );
  }
  if (toolCount.value) {
    parts.push(tr(props.language, "toolCount", { count: toolCount.value }));
  }
  if (!parts.length && hasReasoning.value) {
    parts.push(tr(props.language, "thinking"));
  }
  return parts.length ? `(${parts.join(" · ")})` : "";
});

function shouldAutoCollapse(message: ChatMessage) {
  if (!hasWork.value) return false;
  if (
    message.askUserAnswer?.length &&
    (message.status === "pending" || message.status === "streaming")
  ) {
    return false;
  }
  if (message.askUserAnswer?.length) return true;
  return message.status === "done" || message.status === "error" || message.status === "cancelled";
}

watch(
  () => [
    props.message.status,
    props.message.askUserAnswer?.length ?? 0,
    hasWork.value,
    streaming.value,
  ],
  () => {
    if (userToggled.value) return;
    if (streaming.value && !waitingForAskUser.value) {
      panelExpanded.value = true;
      return;
    }
    if (shouldAutoCollapse(props.message) || !streaming.value) {
      panelExpanded.value = false;
    }
  },
  { immediate: true },
);

function togglePanel() {
  userToggled.value = true;
  panelExpanded.value = !panelExpanded.value;
}
</script>

<style scoped>
.agent-work {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
  margin-bottom: 6px;
  box-sizing: border-box;
}

.agent-work-toggle {
  display: flex;
  align-items: center;
  gap: 5px;
  width: 100%;
  padding: 4px 2px;
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

.agent-work.has-running-subagent > .agent-work-toggle { color: var(--peek-text); }

.agent-work-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 160ms ease;
}

.agent-work-chevron.open {
  transform: rotate(90deg);
}

.agent-work-meta {
  margin-left: auto;
  font-size: 10px;
  font-weight: 500;
  color: var(--peek-faint);
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
.agent-work-body :deep(.reasoning-block) {
  margin-bottom: 0;
}
</style>
