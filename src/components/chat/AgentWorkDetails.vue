<template>
  <div v-if="hasWork" class="agent-work">
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
          <div v-else-if="group.type === 'operations'" class="operations-group">
            <ToolActivityList :activities="group.activities" operations />
          </div>
          <details
            v-else
            class="timeline-tool-group"
            :open="streaming || expandedExecutionIds.has(group.id)"
            @toggle="(event) => onTimelineToggle(group.id, event)"
          >
            <summary class="inner-work-toggle">
              <ChevronRight class="agent-work-chevron" :size="12" />
              <span>{{ executionLabel }}</span>
              <span class="agent-work-meta">{{ group.activities.length }}</span>
            </summary>
            <div class="inner-work-body">
              <ToolActivityList :activities="group.activities" />
            </div>
          </details>
        </template>
      </template>

      <template v-else>
        <ReasoningBlock
          v-if="message.reasoning"
          :reasoning="message.reasoning"
          :streaming="streaming"
          :language="language"
          embedded
        />
        <ToolActivityList v-if="executionActivities.length" :activities="executionActivities" />
        <section v-if="operationActivities.length" class="operations-group">
          <ToolActivityList :activities="operationActivities" operations />
        </section>
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
}>();

type TimelineGroup =
  | { type: "reasoning"; id: string; content: string }
  | { type: "execution" | "operations"; id: string; activities: ToolActivity[] };

const FILE_OPERATION_KINDS = new Set(["create", "edit", "delete", "move"]);
const userToggled = ref(false);
const panelExpanded = ref(true);
const expandedExecutionIds = ref<Set<string>>(new Set());

const streaming = computed(
  () => props.message.status === "pending" || props.message.status === "streaming",
);
const visibleActivities = computed(() =>
  (props.message.toolActivities ?? []).filter(
    (activity) => !(activity.toolName === "ask_user" && activity.status !== "running"),
  ),
);
const activityById = computed(
  () => new Map(visibleActivities.value.map((activity) => [activity.id, activity])),
);
const operationActivities = computed(() =>
  visibleActivities.value.filter((activity) => FILE_OPERATION_KINDS.has(activity.kind)),
);
const executionActivities = computed(() =>
  visibleActivities.value.filter((activity) => !FILE_OPERATION_KINDS.has(activity.kind)),
);
const hasReasoning = computed(() => Boolean(props.message.reasoning?.trim()));
const hasExecutionWork = computed(
  () => hasReasoning.value || executionActivities.value.length > 0,
);

const timelineGroups = computed<TimelineGroup[]>(() => {
  const groups: TimelineGroup[] = [];
  for (const item of props.message.workTimeline ?? []) {
    if (item.type === "reasoning") {
      if (item.content.trim()) groups.push(item);
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

const toolCount = computed(() => visibleActivities.value.length);

const panelLabel = computed(() => tr(props.language, "processSummary"));
const executionLabel = computed(() => tr(props.language, "executionDetails"));

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
  // User answered an ask_user prompt: collapse process chrome.
  if (message.askUserAnswer?.length) return true;
  // AI finished (or failed): always collapse regardless of final text length.
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
    if (streaming.value && !props.message.askUserAnswer?.length) {
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

function onTimelineToggle(groupId: string, event: Event) {
  const target = event.currentTarget as HTMLDetailsElement | null;
  if (!target) return;
  const next = new Set(expandedExecutionIds.value);
  if (target.open) next.add(groupId);
  else next.delete(groupId);
  expandedExecutionIds.value = next;
}
</script>

<style scoped>
.agent-work {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
  margin-bottom: 8px;
  box-sizing: border-box;
}
.operations-group,
.timeline-tool-group {
  min-width: 0;
}
.timeline-tool-group {
  border: 0;
}
.agent-work-toggle,
.inner-work-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 5px 8px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 85%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-surface) 92%, transparent);
  color: var(--peek-muted);
  font-size: 12px;
  font-weight: 600;
  line-height: 1.35;
  cursor: pointer;
  text-align: left;
  list-style: none;
  transition: border-color 140ms ease, background 140ms ease, color 140ms ease;
}
.inner-work-toggle::-webkit-details-marker {
  display: none;
}
.agent-work-toggle:hover,
.inner-work-toggle:hover {
  border-color: color-mix(in srgb, var(--peek-accent) 28%, var(--peek-border));
  color: var(--peek-text);
}
.agent-work-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 160ms ease;
}
.agent-work-chevron.open,
.timeline-tool-group[open] > .inner-work-toggle .agent-work-chevron {
  transform: rotate(90deg);
}
.agent-work-meta {
  margin-left: auto;
  font-size: 11px;
  font-weight: 500;
  color: var(--peek-faint);
}
.agent-work-body,
.inner-work-body {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.agent-work-body {
  margin-top: 2px;
}
.inner-work-body {
  margin-top: 6px;
}
.agent-work-body :deep(.tool-activity-list),
.operations-group :deep(.tool-activity-list) {
  margin-bottom: 0;
}
</style>
