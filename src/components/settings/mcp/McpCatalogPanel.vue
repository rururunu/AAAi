<template>
  <div class="catalog-toolbar">
    <div class="relative flex-1">
      <Search class="text-muted-foreground pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2" />
      <Input
        v-model="query"
        class="h-8 pl-8"
        :placeholder="copy.catalogSearch"
        @keydown.enter.prevent="$emit('search')"
      />
    </div>
    <Button size="sm" class="h-8" :disabled="loading" @click="$emit('search')">
      {{ loading ? copy.searching : copy.search }}
    </Button>
  </div>
  <p class="catalog-hint">{{ copy.catalogHint }}</p>
  <p v-if="runtimeHint" class="catalog-hint">{{ runtimeHint }}</p>
  <p v-if="error" class="form-error">{{ error }}</p>

  <div v-if="showCurated" class="catalog-section">
    <h3>{{ copy.curatedTitle }}</h3>
    <div class="server-list">
      <article v-for="entry in curatedEntries" :key="`curated-${entry.name}`" class="server-card catalog-card">
        <div class="server-main">
          <div class="server-title-row">
            <strong>{{ entry.title }}</strong>
            <span class="badge on">{{ copy.curatedBadge }}</span>
            <span v-if="isInstalled(entry.install.id)" class="badge">{{ copy.added }}</span>
          </div>
          <p class="catalog-desc">{{ entry.description }}</p>
          <p class="command-line"><code>{{ formatCommand(entry.install) }}</code></p>
        </div>
        <div class="server-actions">
          <Button
            size="sm"
            class="h-8"
            :disabled="saving || isInstalled(entry.install.id)"
            @click="$emit('install', entry)"
          >
            {{ isInstalled(entry.install.id) ? copy.added : copy.install }}
          </Button>
        </div>
      </article>
    </div>
  </div>

  <div class="catalog-section">
    <div class="section-head">
      <h3>{{ copy.registryTitle }}</h3>
      <span v-if="registryMeta" class="section-meta">{{ registryMeta }}</span>
    </div>
    <p v-if="!loading && registryEntries.length === 0" class="empty">
      {{ copy.catalogEmpty }}
    </p>
    <div class="server-list">
      <article v-for="entry in registryEntries" :key="entry.name" class="server-card catalog-card">
        <div class="server-main">
          <div class="server-title-row">
            <strong>{{ entry.title }}</strong>
            <span v-if="entry.package.registryType" class="badge">{{ entry.package.registryType }}</span>
            <span v-if="isInstalled(entry.install.id)" class="badge">{{ copy.added }}</span>
          </div>
          <p class="catalog-desc">{{ entry.description }}</p>
          <p class="command-line"><code>{{ formatCommand(entry.install) }}</code></p>
          <p v-if="entry.requiredEnv.length" class="env-line">
            {{ copy.needsEnv(entry.requiredEnv.map((item) => item.name).join(", ")) }}
          </p>
        </div>
        <div class="server-actions">
          <Button
            size="sm"
            class="h-8"
            :disabled="saving || isInstalled(entry.install.id)"
            @click="$emit('install', entry)"
          >
            {{ isInstalled(entry.install.id) ? copy.added : copy.install }}
          </Button>
        </div>
      </article>
    </div>
    <div v-if="nextCursor || loading" class="load-more">
      <Button
        variant="ghost"
        size="sm"
        class="h-8"
        :disabled="loading || !nextCursor"
        @click="$emit('load-more')"
      >
        {{ loading ? copy.searching : copy.loadMore }}
      </Button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Search } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { CatalogEntry } from "@/services/mcp/registry";
import type { McpServerConfig } from "@/types/setting";

const props = defineProps<{
  query: string;
  loading: boolean;
  error: string;
  runtimeHint: string;
  showCurated: boolean;
  curatedEntries: CatalogEntry[];
  registryEntries: CatalogEntry[];
  registryMeta: string;
  nextCursor: string | undefined;
  saving: boolean;
  isInstalled: (id: string) => boolean;
  copy: {
    catalogSearch: string;
    search: string;
    searching: string;
    catalogHint: string;
    curatedTitle: string;
    curatedBadge: string;
    registryTitle: string;
    catalogEmpty: string;
    install: string;
    added: string;
    needsEnv: (names: string) => string;
    loadMore: string;
  };
}>();

const emit = defineEmits<{
  search: [];
  install: [entry: CatalogEntry];
  "load-more": [];
  "update:query": [value: string];
}>();

const query = computed({
  get: () => props.query,
  set: (value: string) => emit("update:query", value),
});

function formatCommand(server: McpServerConfig) {
  return [server.command, ...(server.args ?? [])].filter(Boolean).join(" ");
}
</script>

<style scoped>
.catalog-toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
}

.catalog-hint,
.empty {
  margin: 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
}

.form-error {
  margin: 0;
  color: #ef4444;
  font-size: 12px;
  line-height: 1.5;
}

.catalog-section h3 {
  margin: 0 0 8px;
  font-size: 11px;
  font-weight: 650;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--muted-foreground);
}

.section-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}

.section-head h3 {
  margin: 0;
}

.section-meta {
  color: var(--muted-foreground);
  font-size: 11px;
}

.load-more {
  display: flex;
  justify-content: center;
  padding: 8px 0 4px;
}

.catalog-desc {
  margin: 6px 0 0;
  font-size: 12px;
  line-height: 1.45;
  color: var(--muted-foreground);
}

.server-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
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
</style>
