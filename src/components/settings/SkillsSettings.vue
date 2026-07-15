<template>
  <section class="skills-settings">
    <AppConfirmDialog ref="confirmDialogRef" />

    <header>
      <div>
        <h2>{{ copy.title }}</h2>
        <p>{{ copy.description }}</p>
      </div>
      <div class="header-actions">
        <Button
          variant="ghost"
          size="icon"
          class="size-8 shrink-0 text-muted-foreground"
          :title="copy.openDir"
          :aria-label="copy.openDir"
          :disabled="busy"
          @click="openDir"
        >
          <FolderOpen class="size-3.5" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="h-8 gap-1.5"
          :disabled="busy"
          @click="installFile"
        >
          <FilePlus class="size-3.5" />
          {{ copy.installFile }}
        </Button>
        <Button
          size="sm"
          class="h-8 gap-1.5"
          :disabled="busy"
          @click="installFolder"
        >
          <FolderPlus class="size-3.5" />
          {{ copy.installFolder }}
        </Button>
      </div>
    </header>

    <p v-if="error" class="form-error">{{ error }}</p>

    <div class="skill-list">
      <p v-if="!loading && filtered.length === 0" class="empty">{{ copy.empty }}</p>
      <p v-else-if="loading" class="empty">{{ copy.loading }}</p>
      <article v-for="skill in filtered" :key="`${skill.source}-${skill.name}`" class="skill-card">
        <div class="skill-main">
          <div class="skill-title-row">
            <strong>{{ skill.title || skill.name }}</strong>
            <span class="badge" :class="{ on: skill.source === 'builtin' }">
              {{ skill.source === "builtin" ? copy.builtin : copy.user }}
            </span>
          </div>
          <p v-if="skill.description" class="skill-desc">{{ skill.description }}</p>
          <p v-if="skill.title && skill.title !== skill.name" class="skill-meta">
            {{ skill.name }}
          </p>
        </div>
        <div class="skill-actions">
          <Button
            v-if="skill.source === 'user'"
            type="button"
            variant="ghost"
            size="icon"
            class="size-8 shrink-0 text-muted-foreground hover:text-destructive"
            :title="copy.remove"
            :aria-label="copy.remove"
            :disabled="busy"
            @click="remove(skill)"
          >
            <Trash2 class="size-3.5" />
          </Button>
        </div>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { FilePlus, FolderOpen, FolderPlus, Trash2 } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import {
  installSkill,
  listSkills,
  openSkillsDir,
  selectSkillFile,
  selectSkillFolder,
  uninstallSkill,
  type SkillInfo,
} from "@/commands/skills";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";

const props = defineProps<{ query?: string }>();
const settingStore = useSettingStore();
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);

const skills = ref<SkillInfo[]>([]);
const loading = ref(true);
const busy = ref(false);
const error = ref("");

const copy = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "skills.title"),
    description: tr(language, "skills.description"),
    empty: tr(language, "skills.empty"),
    loading: tr(language, "skills.loading"),
    builtin: tr(language, "skills.builtin"),
    user: tr(language, "skills.user"),
    installFolder: tr(language, "skills.installFolder"),
    installFile: tr(language, "skills.installFile"),
    openDir: tr(language, "skills.openDir"),
    remove: tr(language, "skills.remove"),
    deleteTitle: tr(language, "skills.deleteTitle"),
    deleteDesc: tr(language, "skills.deleteDesc"),
    deleteConfirm: tr(language, "skills.deleteConfirm"),
    cancel: tr(language, "skills.cancel"),
  };
});

const filtered = computed(() => {
  const query = props.query?.trim().toLowerCase() ?? "";
  if (!query) return skills.value;
  return skills.value.filter((skill) => {
    const haystack = [skill.name, skill.title, skill.description, skill.source]
      .join(" ")
      .toLowerCase();
    return haystack.includes(query);
  });
});

async function refresh() {
  loading.value = true;
  error.value = "";
  try {
    skills.value = await listSkills();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    skills.value = [];
  } finally {
    loading.value = false;
  }
}

async function installFolder() {
  busy.value = true;
  error.value = "";
  try {
    const path = await selectSkillFolder();
    if (!path) return;
    await installSkill(path);
    await refresh();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    busy.value = false;
  }
}

async function installFile() {
  busy.value = true;
  error.value = "";
  try {
    const path = await selectSkillFile();
    if (!path) return;
    await installSkill(path);
    await refresh();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    busy.value = false;
  }
}

async function openDir() {
  busy.value = true;
  error.value = "";
  try {
    await openSkillsDir();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    busy.value = false;
  }
}

async function remove(skill: SkillInfo) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: copy.value.deleteTitle,
    description: copy.value.deleteDesc.replace("{name}", skill.title || skill.name),
    confirmLabel: copy.value.deleteConfirm,
    cancelLabel: copy.value.cancel,
  });
  if (!confirmed) return;
  busy.value = true;
  error.value = "";
  try {
    await uninstallSkill(skill.name);
    await refresh();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    busy.value = false;
  }
}

onMounted(() => {
  void refresh();
});
</script>

<style scoped>
.skills-settings {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 12px 16px 20px;
}

header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

header h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

header p {
  margin: 4px 0 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
  max-width: 48ch;
}

.header-actions {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.form-error,
.empty {
  margin: 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
}

.form-error {
  color: #ef4444;
}

.skill-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.skill-card {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--sidebar) 55%, transparent);
}

.skill-main {
  min-width: 0;
  flex: 1;
}

.skill-title-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.skill-title-row strong {
  font-size: 13px;
  font-weight: 600;
}

.badge {
  font-size: 10px;
  min-width: 2.5em;
  text-align: center;
  padding: 2px 7px;
  border-radius: 999px;
  border: 1px solid var(--border);
  color: var(--muted-foreground);
}

.badge.on {
  color: var(--foreground);
  border-color: color-mix(in srgb, var(--foreground) 22%, var(--border));
  background: color-mix(in srgb, var(--foreground) 6%, transparent);
}

.skill-desc {
  margin: 6px 0 0;
  font-size: 12px;
  line-height: 1.45;
  color: var(--muted-foreground);
}

.skill-meta {
  margin: 4px 0 0;
  font-size: 11px;
  color: var(--muted-foreground);
  opacity: 0.85;
}

.skill-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}
</style>
