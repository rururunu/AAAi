<template>
  <div class="workbench-session-list" :class="`is-${variant}`">
    <div
      v-for="session in sessions"
      :key="session.sessionId"
      class="session-row"
      :class="{ active: session.sessionId === activeSessionId }"
      role="button"
      tabindex="0"
      :title="sessionHoverText(session)"
      @click="emit('select', session.sessionId)"
      @keydown.enter="emit('select', session.sessionId)"
      @keydown.space.prevent="emit('select', session.sessionId)"
    >
      <strong>{{ session.preview || untitledLabel }}</strong>
      <button
        type="button"
        class="delete-session"
        :title="deleteLabel"
        @click.stop="emit('delete', session.sessionId)"
      >
        <Trash2 :size="13" />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Trash2 } from "@lucide/vue";
import type { ChatSessionSummary } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";

const props = defineProps<{
  sessions: ChatSessionSummary[];
  activeSessionId: string;
  language: AppLanguage;
  untitledLabel: string;
  deleteLabel: string;
  variant?: "workspace" | "quick";
}>();
const emit = defineEmits<{
  select: [sessionId: string];
  delete: [sessionId: string];
}>();

function formatSessionTime(timestamp: number) {
  return new Intl.DateTimeFormat(props.language, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp));
}

function formatTurnCount(count: number) {
  return props.language === "zh-CN"
    ? `${count} \u8f6e\u5bf9\u8bdd`
    : `${count} ${count === 1 ? "turn" : "turns"}`;
}

function sessionHoverText(session: ChatSessionSummary) {
  return `${formatSessionTime(session.updatedAt)}\n${formatTurnCount(session.turnCount)}`;
}
</script>

<style scoped>
.workbench-session-list.is-workspace { padding: 2px 0 2px 22px; }
.workbench-session-list.is-quick { padding: 3px 0 0; }
.session-row {
  position: relative;
  width: 100%;
  height: 28px;
  display: flex;
  align-items: center;
  padding: 0 5px 0 7px;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-text);
  cursor: pointer;
  text-align: left;
}
.session-row:hover { background: color-mix(in srgb, var(--peek-text) 6%, transparent); }
.session-row.active { background: color-mix(in srgb, var(--peek-accent) 13%, transparent); }
.is-quick .session-row { padding-left: 9px; }
.session-row > strong {
  min-width: 0;
  display: block;
  flex: 1;
  overflow: hidden;
  font-size: 11px;
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.delete-session {
  flex: none;
  width: 23px;
  height: 23px;
  display: inline-grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  opacity: 0;
}
.session-row:hover .delete-session, .session-row:focus-within .delete-session { opacity: 1; }
.delete-session:hover { color: var(--peek-danger); background: color-mix(in srgb, var(--peek-danger) 12%, transparent); }
</style>
