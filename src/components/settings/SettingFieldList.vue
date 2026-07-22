<template>
  <p v-if="items.length === 0" class="text-muted-foreground px-4 py-6 text-sm">
    {{ emptyText }}
  </p>

  <section v-for="group in groups" :key="group.id" class="py-2">
    <h2 class="text-muted-foreground px-4 py-2 text-[11px] font-semibold tracking-wider uppercase">
      {{ group.title }}
    </h2>

    <article
      v-for="item in group.items"
      :key="item.id"
      class="border-t border-border px-4 py-3.5"
      :class="
        item.type === 'custom-color'
          ? 'grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(240px,320px)] lg:items-start'
          : 'grid grid-cols-[minmax(0,1fr)_220px] items-start gap-4'
      "
    >
      <div class="space-y-1">
        <p class="text-muted-foreground text-[11px]">AAAi › {{ item.path }}</p>
        <h3 class="text-sm font-medium">{{ item.title }}</h3>
        <p class="text-muted-foreground text-xs leading-relaxed">{{ item.description }}</p>
      </div>

      <div class="pt-0.5">
        <Select
          v-if="item.type === 'select-color'"
          :model-value="settingStore.colorScheme"
          @update:model-value="(v) => emit('color-scheme-change', v)"
        >
          <SelectTrigger class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="option in colorSchemeSelectOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </SelectItem>
          </SelectContent>
        </Select>

        <Select
          v-else-if="item.type === 'select-language'"
          :model-value="settingStore.language"
          @update:model-value="(v) => emit('language-change', v)"
        >
          <SelectTrigger class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="option in languageSelectOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </SelectItem>
          </SelectContent>
        </Select>

        <Select
          v-else-if="item.type === 'select-zoom'"
          :model-value="String(settingStore.zoom)"
          @update:model-value="(v) => emit('zoom-change', v)"
        >
          <SelectTrigger class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="option in zoomSelectOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </SelectItem>
          </SelectContent>
        </Select>

        <Select
          v-else-if="item.type === 'select-reasoning-effort'"
          :model-value="settingStore.reasoningEffort"
          @update:model-value="(v) => emit('reasoning-effort-change', v)"
        >
          <SelectTrigger class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="option in reasoningEffortSelectOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </SelectItem>
          </SelectContent>
        </Select>

        <AccentColorField
          v-else-if="item.type === 'custom-color'"
          :model-value="settingStore.customAccentColor"
          @update:model-value="(value) => emit('custom-accent-change', value)"
          @reset="emit('reset-custom-accent')"
        />

        <div v-else-if="item.type === 'select-model'" class="space-y-1.5">
          <div class="flex items-center gap-1.5">
            <Select
              :model-value="resolveModelSelectValue(item.id === 'multimodalModel' ? settingStore.multimodalModel : settingStore.chatModel)"
              :disabled="chatModelStore.loading || availableModelOptions.length === 0"
              @update:model-value="(v) => item.id === 'multimodalModel' ? emit('multimodal-model-change', v) : emit('default-model-change', v)"
            >
              <SelectTrigger class="w-full">
                <SelectValue :placeholder="modelStatusText">
                  <span
                    v-if="selectedModelOption(item.id)"
                    class="inline-flex min-w-0 items-center gap-1.5"
                  >
                    <component
                      :is="selectedModelOption(item.id)?.icon"
                      v-if="selectedModelOption(item.id)?.icon"
                      class="size-3.5 shrink-0 text-muted-foreground"
                    />
                    <span class="truncate">{{ selectedModelOption(item.id)?.label }}</span>
                  </span>
                </SelectValue>
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in availableModelOptions"
                  :key="option.value"
                  :value="option.value"
                  :text-value="option.label"
                >
                  <template v-if="option.icon" #leading>
                    <component
                      :is="option.icon"
                      class="size-3.5 shrink-0 text-muted-foreground"
                    />
                  </template>
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>
            <Button
              type="button"
              variant="outline"
              size="icon"
              class="size-8 shrink-0"
              :disabled="chatModelStore.loading || chatModelStore.refreshing"
              :title="refreshModelsLabel"
              :aria-label="refreshModelsLabel"
              @click="refreshModelList"
            >
              <RefreshCw
                class="size-3.5"
                :class="{ 'animate-spin': chatModelStore.refreshing }"
              />
            </Button>
          </div>
          <Select
            v-if="selectedModelThinkingTierOptions(item.id).length > 1"
            :model-value="item.id === 'multimodalModel' ? settingStore.multimodalModel : settingStore.chatModel"
            :disabled="chatModelStore.loading"
            @update:model-value="(v) => item.id === 'multimodalModel' ? emit('multimodal-model-change', v) : emit('default-model-change', v)"
          >
            <SelectTrigger class="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="option in selectedModelThinkingTierOptions(item.id)"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </SelectItem>
            </SelectContent>
          </Select>
          <p v-if="chatModelStore.error" class="text-[10px] leading-4 text-destructive">
            {{ chatModelStore.error }}
          </p>
        </div>

        <Select
          v-else-if="item.type === 'select-reasoning-language'"
          :model-value="settingStore.reasoningLanguage"
          @update:model-value="(v) => emit('reasoning-language-change', v)"
        >
          <SelectTrigger class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="option in reasoningLanguageSelectOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </SelectItem>
          </SelectContent>
        </Select>

        <Select
          v-else-if="item.type === 'select-web-search-provider'"
          :model-value="settingStore.webSearchProvider"
          :disabled="!settingStore.webSearchEnabled"
          @update:model-value="(v) => emit('web-search-provider-change', v)"
        >
          <SelectTrigger class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="option in webSearchProviderSelectOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </SelectItem>
          </SelectContent>
        </Select>

        <Select
          v-else-if="item.type === 'select-tool-approval-mode'"
          :model-value="settingStore.toolApprovalMode"
          @update:model-value="(v) => emit('tool-approval-mode-change', v)"
        >
          <SelectTrigger class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="option in toolApprovalModeSelectOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </SelectItem>
          </SelectContent>
        </Select>

        <SecretInput
          v-else-if="item.type === 'secret'"
          :model-value="apiKeyDraft"
          :placeholder="apiKeyPlaceholder"
          @update:model-value="(v) => emit('update:apiKeyDraft', String(v))"
          @blur="emit('save-api-key')"
        />

        <SecretInput
          v-else-if="item.type === 'memory-secret'"
          :model-value="mem0ApiKeyDraft"
          placeholder="m0-..."
          :disabled="!settingStore.memoryEnabled"
          @update:model-value="(v) => emit('update:mem0ApiKeyDraft', String(v))"
          @blur="emit('save-memory-settings')"
        />

        <SecretInput
          v-else-if="item.type === 'search-secret'"
          :model-value="item.id === 'serperApiKey' ? serperApiKeyDraft : tavilyApiKeyDraft"
          :placeholder="item.id === 'serperApiKey' ? 'serper-...' : 'tvly-...'"
          :disabled="!settingStore.webSearchEnabled"
          @update:model-value="(v) => onSearchSecretInput(item.id, v)"
          @blur="emit('save-web-search-settings')"
        />

        <Input
          v-else-if="item.type === 'memory-text'"
          :model-value="item.id === 'mem0UserId' ? mem0UserIdDraft : mem0BaseUrlDraft"
          :disabled="!settingStore.memoryEnabled"
          @update:model-value="(v) => onMemoryTextInput(item.id, v)"
          @blur="emit('save-memory-settings')"
        />

        <button
          v-else-if="item.type === 'toggle'"
          type="button"
          class="setting-toggle"
          :class="{ active: toggleActive(item.id) }"
          :aria-pressed="toggleActive(item.id)"
          @click="emit('toggle', item.id)"
        >
          <span class="setting-toggle-knob"></span>
        </button>

        <div
          v-else-if="item.type === 'slider'"
          class="flex items-center gap-3 w-full max-w-[200px]"
        >
          <input
            type="range"
            :min="item.min ?? 10"
            :max="item.max ?? 100"
            :step="item.step ?? 5"
            :value="getSettingValue(item.id)"
            @input="(e) => onSliderChange(item.id, e)"
            class="setting-slider h-1.5 w-full cursor-pointer appearance-none rounded-lg bg-border accent-primary focus:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          />
          <span class="text-xs font-semibold tabular-nums min-w-[36px] text-right text-muted-foreground select-none">
            {{ getSettingValue(item.id) }}%
          </span>
        </div>

        <HotkeyRecordField
          v-else-if="item.type === 'hotkey-record'"
          :model-value="settingStore.secondaryHotkey"
          @update:model-value="(value) => (settingStore.secondaryHotkey = value)"
        />

        <span
          v-else
          class="text-sm"
          :class="{ 'font-mono text-xs break-all': item.id === 'appIdentifier' }"
        >{{ item.value }}</span>
      </div>
    </article>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { RefreshCw } from "@lucide/vue";
import AccentColorField from "@/components/settings/AccentColorField.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SecretInput } from "@/components/ui/secret-input";
import HotkeyRecordField from "@/components/settings/HotkeyRecordField.vue";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSettingStore } from "@/stores/setting";
import { useChatModelStore } from "@/stores/chatModel";
import { getProviderIcon, getModelDisplayLabel, isDeepSeekProvider, isGeminiProvider } from "@/lib/providerIcons";
import {
  findModelEntry,
  isModelEntrySelected,
  localizeThinkingTierLabel,
} from "@/lib/modelThinking";
import { tr } from "@/services/i18n";
import type { SettingDefinition } from "@/pages/Settings/settingsDefinitions";
import {
  colorSchemeOptions,
  languageOptions,
  localizedOptionLabel,
  reasoningEffortOptions,
  reasoningLanguageOptions,
  webSearchProviderOptions,
  toolApprovalModeOptions,
  zoomOptions,
} from "@/types/setting";

const props = defineProps<{
  items: SettingDefinition[];
  emptyText: string;
  apiKeyDraft: string;
  mem0ApiKeyDraft: string;
  mem0UserIdDraft: string;
  mem0BaseUrlDraft: string;
  serperApiKeyDraft: string;
  tavilyApiKeyDraft: string;
}>();

const emit = defineEmits<{
  toggle: [id: string];
  "slider-change": [id: string, value: number];
  "color-scheme-change": [value: unknown];
  "language-change": [value: unknown];
  "zoom-change": [value: unknown];
  "reasoning-effort-change": [value: unknown];
  "reasoning-language-change": [value: unknown];
  "tool-approval-mode-change": [value: unknown];
  "web-search-provider-change": [value: unknown];
  "default-model-change": [value: unknown];
  "multimodal-model-change": [value: unknown];
  "custom-accent-change": [value: string];
  "reset-custom-accent": [];
  "update:apiKeyDraft": [value: string];
  "save-api-key": [];
  "update:mem0ApiKeyDraft": [value: string];
  "update:mem0UserIdDraft": [value: string];
  "update:mem0BaseUrlDraft": [value: string];
  "save-memory-settings": [];
  "update:serperApiKeyDraft": [value: string];
  "update:tavilyApiKeyDraft": [value: string];
  "save-web-search-settings": [];
}>();

const settingStore = useSettingStore();
const chatModelStore = useChatModelStore();

const apiKeyPlaceholder = computed(() => tr(settingStore.language, "settings.apiKeyPlaceholder"));

const groups = computed(() => {
  const map = new Map<string, SettingDefinition[]>();
  for (const item of props.items) {
    const list = map.get(item.group) ?? [];
    list.push(item);
    map.set(item.group, list);
  }
  return Array.from(map.entries()).map(([title, groupItems]) => ({
    id: title,
    title,
    items: groupItems,
  }));
});

const colorSchemeSelectOptions = computed(() =>
  colorSchemeOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const languageSelectOptions = computed(() =>
  languageOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const reasoningEffortSelectOptions = computed(() =>
  reasoningEffortOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const reasoningLanguageSelectOptions = computed(() =>
  reasoningLanguageOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const webSearchProviderSelectOptions = computed(() =>
  webSearchProviderOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const toolApprovalModeSelectOptions = computed(() =>
  toolApprovalModeOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const zoomSelectOptions = computed(() =>
  zoomOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const availableModelOptions = computed(() => {
  const models = [...chatModelStore.models];
  const current = settingStore.chatModel.trim();
  if (
    current &&
    models.length > 0 &&
    !models.some((model) => isModelEntrySelected(model, current))
  ) {
    models.unshift({ id: current, ownedBy: "", provider: "" });
  }
  return models.map((model) => {
    const name = getModelDisplayLabel(model);
    const showOwner =
      !!model.ownedBy &&
      !isDeepSeekProvider(model.provider) &&
      !isGeminiProvider(model.provider);
    return {
      value: model.id,
      label: showOwner ? `${name} · ${model.ownedBy}` : name,
      icon: getProviderIcon(model.provider),
    };
  });
});

function resolveModelSelectValue(variantOrDefaultId: string) {
  const entry = findModelEntry(chatModelStore.models, variantOrDefaultId);
  return entry?.id ?? variantOrDefaultId;
}

function selectedModelThinkingTierOptions(itemId: string) {
  const selectedId =
    itemId === "multimodalModel"
      ? settingStore.multimodalModel
      : settingStore.chatModel;
  const entry = findModelEntry(chatModelStore.models, selectedId);
  if (!entry?.thinkingVariants?.length) {
    return [];
  }
  return entry.thinkingVariants.map((variant) => ({
    value: variant.id,
    label: localizeThinkingTierLabel(variant.label, settingStore.language),
  }));
}

function selectedModelOption(itemId: string) {
  const selectedId =
    itemId === "multimodalModel"
      ? settingStore.multimodalModel
      : settingStore.chatModel;
  const entry = findModelEntry(chatModelStore.models, selectedId);
  const optionValue = entry?.id ?? selectedId;
  return availableModelOptions.value.find((option) => option.value === optionValue) ?? null;
}

const modelStatusText = computed(() => {
  if (chatModelStore.loading) {
    return tr(settingStore.language, "loadingModels");
  }
  return tr(settingStore.language, "noModels");
});

const refreshModelsLabel = computed(() =>
  tr(settingStore.language, "refreshModels"),
);

async function refreshModelList() {
  await chatModelStore.reload();
}

function toggleActive(id: string) {
  if (id === "memoryEnabled") return settingStore.memoryEnabled;
  if (id === "webSearchEnabled") return settingStore.webSearchEnabled;
  if (id === "lspEnabled") return settingStore.lspEnabled;
  if (id === "passToolReasoning") return settingStore.passToolReasoning;
  if (id === "multimodalSplitAnalysis") return settingStore.multimodalSplitAnalysis;
  if (id === "largeContextEnabled") return settingStore.largeContextEnabled;
  return false;
}

function getSettingValue(id: string) {
  if (id === "opacity") {
    return settingStore.opacity;
  }
  return 100;
}

function onSliderChange(id: string, event: Event) {
  const target = event.target as HTMLInputElement;
  const value = parseInt(target.value, 10);
  emit("slider-change", id, value);
}

function onMemoryTextInput(id: string, value: string | number) {
  if (id === "mem0UserId") emit("update:mem0UserIdDraft", String(value));
  if (id === "mem0BaseUrl") emit("update:mem0BaseUrlDraft", String(value));
}

function onSearchSecretInput(id: string, value: string | number) {
  if (id === "serperApiKey") emit("update:serperApiKeyDraft", String(value));
  if (id === "tavilyApiKey") emit("update:tavilyApiKeyDraft", String(value));
}
</script>

<style scoped>
.setting-toggle {
  position: relative;
  width: 44px;
  height: 24px;
  margin: 0;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--muted);
  cursor: default;
  transition: background 160ms ease, border-color 160ms ease;
}

.setting-toggle.active {
  background: var(--primary);
  border-color: color-mix(in srgb, var(--primary) 70%, white 30%);
}

.setting-toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: white;
  box-shadow: 0 1px 3px rgb(0 0 0 / 28%);
  transition: transform 160ms ease;
}

.setting-toggle.active .setting-toggle-knob {
  transform: translateX(20px);
}
</style>
