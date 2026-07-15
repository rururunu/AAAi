<template>
  <div class="server-list">
    <p v-if="servers.length === 0 && !disabledActions" class="empty">{{ copy.empty }}</p>
    <article v-for="server in servers" :key="server.id" class="server-card">
      <div class="server-main">
        <div class="server-title-row">
          <strong>{{ serverTitle(server) }}</strong>
          <span class="badge" :class="{ on: server.enabled !== false }">
            {{ server.enabled !== false ? copy.enabled : copy.disabled }}
          </span>
        </div>
        <p v-if="server.description?.trim()" class="catalog-desc">{{ server.description }}</p>
        <p class="command-line">
          <code>{{ formatCommand(server) }}</code>
        </p>
        <p v-if="server.title?.trim() && server.title.trim() !== server.id" class="env-line">
          {{ server.id }}
        </p>
        <p v-if="server.env?.length" class="env-line">
          {{ copy.envCount(server.env.length) }}
        </p>
      </div>
      <div class="server-actions">
        <button
          type="button"
          class="setting-toggle"
          :class="{ active: server.enabled !== false }"
          :aria-pressed="server.enabled !== false"
          :title="copy.enabled"
          @click="$emit('toggle', server)"
        >
          <span class="setting-toggle-knob" />
        </button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="size-8 shrink-0 text-muted-foreground"
          :title="copy.edit"
          :aria-label="copy.edit"
          :disabled="disabledActions"
          @click="$emit('edit', server)"
        >
          <Pencil class="size-3.5" />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="size-8 shrink-0 text-muted-foreground hover:text-destructive"
          :title="copy.remove"
          :aria-label="copy.remove"
          :disabled="disabledActions"
          @click="$emit('remove', server)"
        >
          <Trash2 class="size-3.5" />
        </Button>
      </div>
    </article>
  </div>
</template>

<script setup lang="ts">
import { Pencil, Trash2 } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import type { McpServerConfig } from "@/types/setting";

defineProps<{
  servers: McpServerConfig[];
  disabledActions: boolean;
  copy: {
    empty: string;
    enabled: string;
    disabled: string;
    edit: string;
    remove: string;
    envCount: (count: number) => string;
  };
}>();

defineEmits<{
  toggle: [server: McpServerConfig];
  edit: [server: McpServerConfig];
  remove: [server: McpServerConfig];
}>();

function formatCommand(server: McpServerConfig) {
  return [server.command, ...(server.args ?? [])].filter(Boolean).join(" ");
}

function serverTitle(server: McpServerConfig) {
  return server.title?.trim() || server.id;
}
</script>

<style scoped>
.empty {
  margin: 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
}

.catalog-desc {
  margin: 6px 0 0;
  font-size: 12px;
  line-height: 1.45;
  color: var(--muted-foreground);
}

.server-card {
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--sidebar) 55%, transparent);
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 12px;
}

.server-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.server-main {
  min-width: 0;
  flex: 1;
}

.server-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.server-title-row strong {
  font-size: 13px;
}

.badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--muted-foreground) 16%, transparent);
  color: var(--muted-foreground);
}

.badge.on {
  background: color-mix(in srgb, var(--primary) 18%, transparent);
  color: var(--primary);
}

.command-line {
  margin: 6px 0 0;
  font-size: 11px;
  overflow: hidden;
}

.command-line code {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  color: var(--muted-foreground);
}

.env-line {
  margin: 4px 0 0;
  font-size: 11px;
  color: var(--muted-foreground);
}

.server-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

.setting-toggle {
  position: relative;
  width: 36px;
  height: 20px;
  border: 0;
  border-radius: 999px;
  background: color-mix(in srgb, var(--muted-foreground) 28%, transparent);
  cursor: pointer;
  padding: 0;
  flex: none;
}

.setting-toggle.active {
  background: color-mix(in srgb, var(--primary) 75%, transparent);
}

.setting-toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 999px;
  background: white;
  transition: transform 140ms ease;
}

.setting-toggle.active .setting-toggle-knob {
  transform: translateX(16px);
}
</style>
