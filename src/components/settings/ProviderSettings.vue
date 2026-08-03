<template>
  <section class="provider-settings">
    <AppConfirmDialog ref="confirmDialogRef" />

    <Transition name="fade-slide" mode="out-in">
      <div v-if="currentView === 'list'" key="list" class="view-container">
        <header class="view-header">
          <div>
            <h2>{{ t('settings.provider.title') }}</h2>
            <p>{{ t('settings.provider.description') }}</p>
          </div>
        </header>

        <div class="sections-container">
          <div class="section-block">
            <h4 class="section-title">DeepSeek</h4>
            <button 
              type="button" 
              class="provider-nav-card" 
              @click="currentView = 'deepseek'"
            >
              <div class="card-left">
                <div class="icon-wrapper">
                  <DeepSeekIcon :size="18" />
                </div>
                <div class="card-text">
                  <h3>{{ t('settings.provider.deepseek') }}</h3>
                  <p>DeepSeek API</p>
                </div>
              </div>
              <div class="card-right">
                <span 
                  class="status-badge" 
                  :class="{ configured: isDeepSeekConfigured }"
                >
                  {{ isDeepSeekConfigured ? t('settings.history.publicGroup') : t('settings.history.empty') }}
                </span>
                <ChevronRight class="size-4 text-muted-foreground arrow-icon" />
              </div>
            </button>
          </div>

          <div class="section-block">
            <h4 class="section-title">Gemini</h4>
            <button 
              type="button" 
              class="provider-nav-card" 
              @click="openGemini"
            >
              <div class="card-left">
                <div class="icon-wrapper">
                  <GeminiIcon :size="18" />
                </div>
                <div class="card-text">
                  <h3>{{ t('settings.provider.gemini') }}</h3>
                  <p>{{ geminiSubtitle }}</p>
                </div>
              </div>
              <div class="card-right">
                <span 
                  class="status-badge" 
                  :class="{ configured: isGeminiConfigured }"
                >
                  {{ isGeminiConfigured ? t('settings.provider.geminiSignedIn') : t('settings.provider.geminiSignedOut') }}
                </span>
                <ChevronRight class="size-4 text-muted-foreground arrow-icon" />
              </div>
            </button>
          </div>

          <div class="section-block">
            <div class="section-header-row">
              <h4 class="section-title">{{ t('settings.provider.custom') }}</h4>
              <Button 
                size="sm" 
                variant="outline" 
                class="h-7 gap-1 pl-2 pr-2.5 text-xs add-btn" 
                @click="addCustomProvider"
              >
                <Plus class="size-3.5" />
                {{ t('settings.provider.add') }}
              </Button>
            </div>

            <div v-if="settingStore.customProviders.length === 0" class="empty-state">
              <p class="text-xs text-muted-foreground">{{ t('settings.empty') }}</p>
            </div>
            <div v-else class="cards-list">
              <button 
                v-for="provider in settingStore.customProviders" 
                :key="provider.id"
                type="button" 
                class="provider-nav-card" 
                @click="startEditCustom(provider.id)"
              >
                <div class="card-left">
                  <div class="icon-wrapper">
                    <Globe2 class="size-5" />
                  </div>
                  <div class="card-text">
                    <h3>{{ provider.name }}</h3>
                    <p class="truncate max-w-[280px] font-mono text-[10px]">{{ provider.baseUrl }}</p>
                  </div>
                </div>
                <div class="card-right">
                  <span 
                    class="status-badge" 
                    :class="{ configured: !!provider.baseUrl.trim() }"
                  >
                    {{ provider.baseUrl.trim() ? t('settings.history.publicGroup') : t('settings.history.empty') }}
                  </span>
                  <ChevronRight class="size-4 text-muted-foreground arrow-icon" />
                </div>
              </button>
            </div>
          </div>
        </div>
      </div>

      <div v-else-if="currentView === 'deepseek'" key="deepseek" class="view-container">
        <div class="back-btn-row">
          <Button 
            variant="ghost" 
            size="sm" 
            class="h-8 gap-1.5 pl-1.5 text-muted-foreground hover:text-foreground back-btn"
            @click="currentView = 'list'"
          >
            <ChevronLeft class="size-4" />
            {{ t('settings.provider.back') }}
          </Button>
        </div>
        <header class="view-header edit-header">
          <div class="header-details">
            <div class="edit-title-row">
              <DeepSeekIcon :size="18" class="edit-title-icon" />
              <h2>{{ t('settings.provider.deepseek') }}</h2>
            </div>
            <p>DeepSeek API</p>
          </div>
        </header>

        <div class="edit-form border-t border-border pt-4">
          <div class="field-row">
            <label>{{ t('settings.provider.apiKey') }}</label>
            <SecretInput
              v-model="deepseekKey"
              placeholder="sk-..."
              @blur="saveDeepSeek"
            />
          </div>

          <div class="form-actions pt-6">
            <Button 
              variant="outline" 
              size="sm" 
              class="h-8 flex-1 gap-1.5" 
              @click="currentView = 'list'"
            >
              <ChevronLeft class="size-3.5" />
              {{ t('settings.provider.back') }}
            </Button>
            <Button 
              size="sm" 
              class="h-8 flex-1 gap-1.5" 
              @click="saveDeepSeekAndGoBack"
            >
              <Save class="size-3.5" />
              {{ t('settings.provider.save') }}
            </Button>
          </div>
        </div>
      </div>

      <div v-else-if="currentView === 'gemini'" key="gemini" class="view-container">
        <div class="back-btn-row">
          <Button 
            variant="ghost" 
            size="sm" 
            class="h-8 gap-1.5 pl-1.5 text-muted-foreground hover:text-foreground back-btn"
            @click="currentView = 'list'"
          >
            <ChevronLeft class="size-4" />
            {{ t('settings.provider.back') }}
          </Button>
        </div>
        <header class="view-header edit-header">
          <div class="header-details">
            <div class="edit-title-row">
              <GeminiIcon :size="18" class="edit-title-icon" />
              <h2>{{ t('settings.provider.gemini') }}</h2>
            </div>
            <p>{{ t('settings.provider.geminiDescription') }}</p>
          </div>
        </header>

        <div class="edit-form border-t border-border pt-4">
          <div class="oauth-status">
            <p class="oauth-status-label">{{ t('settings.provider.geminiAccount') }}</p>
            <p class="oauth-status-value">
              {{ isGeminiConfigured
                ? (settingStore.geminiOauth.email || t('settings.provider.geminiSignedIn'))
                : t('settings.provider.geminiSignedOut') }}
            </p>
            <p v-if="geminiError" class="oauth-error">{{ geminiError }}</p>
          </div>

          <div class="form-actions pt-2">
            <Button 
              v-if="!isGeminiConfigured && !geminiBusy"
              size="sm" 
              class="h-8 flex-1 gap-1.5" 
              @click="loginGemini"
            >
              {{ t('settings.provider.geminiLogin') }}
            </Button>
            <template v-else-if="!isGeminiConfigured && geminiBusy">
              <Button 
                size="sm" 
                class="h-8 flex-1 gap-1.5" 
                disabled
              >
                {{ t('settings.provider.geminiLoggingIn') }}
              </Button>
              <Button 
                variant="outline"
                size="sm" 
                class="h-8 flex-1 gap-1.5" 
                @click="cancelGeminiLogin"
              >
                {{ t('settings.provider.geminiCancelLogin') }}
              </Button>
            </template>
            <Button 
              v-else
              variant="outline"
              size="sm" 
              class="h-8 flex-1 gap-1.5" 
              :disabled="geminiBusy"
              @click="logoutGemini"
            >
              {{ t('settings.provider.geminiLogout') }}
            </Button>
            <Button 
              variant="outline" 
              size="sm" 
              class="h-8 flex-1 gap-1.5" 
              @click="currentView = 'list'"
            >
              <ChevronLeft class="size-3.5" />
              {{ t('settings.provider.back') }}
            </Button>
          </div>
        </div>
      </div>

      <div v-else-if="currentView === 'custom'" key="custom" class="view-container">
        <div class="back-btn-row">
          <Button 
            variant="ghost" 
            size="sm" 
            class="h-8 gap-1.5 pl-1.5 text-muted-foreground hover:text-foreground back-btn"
            @click="currentView = 'list'"
          >
            <ChevronLeft class="size-4" />
            {{ t('settings.provider.back') }}
          </Button>
        </div>
        <header class="view-header edit-header">
          <div class="header-details">
            <h2>{{ isNewProvider ? t('settings.provider.add') : t('settings.provider.custom') }}</h2>
            <p>OpenAI-compatible API</p>
          </div>

          <Button 
            v-if="!isNewProvider"
            variant="destructive"
            size="sm"
            class="h-8 gap-1.5 delete-top-btn"
            @click="deleteCustom(editingProviderId)"
          >
            <Trash2 class="size-3.5" />
            {{ t('settings.provider.delete') }}
          </Button>
        </header>

        <div class="edit-form border-t border-border pt-4">
          <div class="field-row">
            <label>{{ t('settings.provider.name') }}</label>
            <Input
              v-model="customName"
              :placeholder="t('settings.provider.namePlaceholder')"
              class="h-8 text-xs"
              @blur="saveCustom"
            />
          </div>

          <div class="field-row">
            <label>{{ t('settings.provider.baseUrl') }}</label>
            <Input
              v-model="customUrl"
              placeholder="https://api.openai.com/v1"
              class="h-8 text-xs font-mono"
              @blur="saveCustom"
            />
          </div>

          <div class="field-row">
            <label>{{ t('settings.provider.apiKey') }}</label>
            <SecretInput
              v-model="customKey"
              placeholder="sk-..."
              @blur="saveCustom"
            />
          </div>

          <div class="field-row">
            <label>{{ t('settings.provider.modelsList') }}</label>
            <textarea
              v-model="customModels"
              :placeholder="t('settings.provider.modelsPlaceholder')"
              class="custom-textarea"
              @blur="saveCustom"
            ></textarea>
          </div>

          <div class="form-actions pt-6">
            <Button 
              variant="outline" 
              size="sm" 
              class="h-8 flex-1 gap-1.5" 
              @click="currentView = 'list'"
            >
              <ChevronLeft class="size-3.5" />
              {{ t('settings.provider.back') }}
            </Button>
            
            <Button 
              size="sm" 
              class="h-8 flex-1 gap-1.5" 
              @click="saveCustomAndGoBack"
            >
              <Plus v-if="isNewProvider" class="size-3.5" />
              <Save v-else class="size-3.5" />
              {{ isNewProvider ? t('settings.provider.add') : t('settings.provider.save') }}
            </Button>
          </div>
        </div>
      </div>
    </Transition>
  </section>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { Globe2, ChevronLeft, ChevronRight, Plus, Trash2, Save } from "@lucide/vue";
import DeepSeekIcon from "@/components/icons/DeepSeekIcon.vue";
import GeminiIcon from "@/components/icons/GeminiIcon.vue";
import { useSettingStore } from "@/stores/setting";
import { useChatModelStore } from "@/stores/chatModel";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SecretInput } from "@/components/ui/secret-input";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { geminiOauthCancelLogin, geminiOauthLogin, geminiOauthLogout } from "@/services/ipc";
import { tr } from "@/services/i18n";
import type { SettingsI18nKey } from "@/services/locales/settings";

defineProps<{
  query?: string;
}>();

const settingStore = useSettingStore();
const chatModelStore = useChatModelStore();

const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);

const currentView = ref<"list" | "deepseek" | "gemini" | "custom">("list");
const editingProviderId = ref<string | null>(null);

const deepseekKey = ref(settingStore.deepseekApiKey);
const geminiBusy = ref(false);
const geminiError = ref("");

const customName = ref("");
const customUrl = ref("");
const customKey = ref("");
const customModels = ref("");

const isDeepSeekConfigured = computed(() => {
  return !!settingStore.deepseekApiKey.trim();
});

const isGeminiConfigured = computed(() => {
  const oauth = settingStore.geminiOauth;
  return !!(oauth?.accessToken?.trim() || oauth?.refreshToken?.trim());
});

const geminiSubtitle = computed(() => {
  if (isGeminiConfigured.value && settingStore.geminiOauth.email) {
    return settingStore.geminiOauth.email;
  }
  return "Antigravity";
});

const isNewProvider = computed(() => {
  if (!editingProviderId.value) return true;
  return !settingStore.customProviders.some(p => p.id === editingProviderId.value);
});

const t = (key: string) => {
  return tr(settingStore.language, key as SettingsI18nKey);
};

function openGemini() {
  geminiError.value = "";
  currentView.value = "gemini";
}

async function loginGemini() {
  geminiError.value = "";
  geminiBusy.value = true;
  try {
    await geminiOauthLogin();
    await settingStore.load();
    await chatModelStore.refresh();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    // Browser cancel / Google consent decline should not look like a hard failure.
    if (
      !/sign-in was cancelled|sign-in was canceled|access_denied/i.test(message)
    ) {
      geminiError.value = message;
    }
  } finally {
    geminiBusy.value = false;
  }
}

async function cancelGeminiLogin() {
  try {
    await geminiOauthCancelLogin();
  } catch {
    // Login await will surface the cancel/timeout result.
  }
}

async function logoutGemini() {
  geminiError.value = "";
  geminiBusy.value = true;
  try {
    await geminiOauthLogout();
    await settingStore.load();
    await chatModelStore.refresh();
  } catch (error) {
    geminiError.value = error instanceof Error ? error.message : String(error);
  } finally {
    geminiBusy.value = false;
  }
}

async function saveDeepSeek() {
  if (deepseekKey.value.trim() === settingStore.deepseekApiKey) {
    return;
  }
  await settingStore.update({ deepseekApiKey: deepseekKey.value.trim() });
  await chatModelStore.refresh();
}

async function saveDeepSeekAndGoBack() {
  await saveDeepSeek();
  currentView.value = "list";
}

function startEditCustom(id: string) {
  const provider = settingStore.customProviders.find(p => p.id === id);
  if (provider) {
    editingProviderId.value = id;
    customName.value = provider.name;
    customUrl.value = provider.baseUrl;
    customKey.value = provider.apiKey;
    customModels.value = provider.models;
    currentView.value = "custom";
  }
}

function addCustomProvider() {
  editingProviderId.value = Math.random().toString(36).substring(2, 11);
  customName.value = "";
  customUrl.value = "";
  customKey.value = "";
  customModels.value = "";
  currentView.value = "custom";
}

async function saveCustom() {
  if (!editingProviderId.value) return;
  
  const nextName = customName.value.trim() || `Custom - ${editingProviderId.value}`;
  const nextUrl = customUrl.value.trim();
  const nextKey = customKey.value.trim();
  const nextModels = customModels.value.trim();
  
  const list = [...settingStore.customProviders];
  const index = list.findIndex(p => p.id === editingProviderId.value);
  
  const updatedProvider = {
    id: editingProviderId.value,
    name: nextName,
    baseUrl: nextUrl,
    apiKey: nextKey,
    models: nextModels
  };
  
  if (index !== -1) {
    const current = list[index];
    if (
      current.name === nextName &&
      current.baseUrl === nextUrl &&
      current.apiKey === nextKey &&
      current.models === nextModels
    ) {
      return;
    }
    list[index] = updatedProvider;
  } else {
    list.push(updatedProvider);
  }
  
  await settingStore.update({ customProviders: list });
  await chatModelStore.refresh();
}

async function saveCustomAndGoBack() {
  await saveCustom();
  currentView.value = "list";
}

async function deleteCustom(id: string | null) {
  if (!id) return;
  const provider = settingStore.customProviders.find(p => p.id === id);
  if (!provider) return;
  
  const confirmed = await confirmDialogRef.value?.ask({
    title: t('settings.provider.delete'),
    description: t('settings.provider.deleteConfirm'),
    confirmLabel: t('settings.history.deleteLabel'),
    cancelLabel: t('settings.history.cancel'),
  });
  if (!confirmed) return;
  
  const list = settingStore.customProviders.filter(p => p.id !== id);
  await settingStore.update({ customProviders: list });
  await chatModelStore.refresh();
  currentView.value = "list";
}
</script>

<style scoped>
.provider-settings {
  display: flex;
  flex-direction: column;
  padding: 12px 16px 20px;
  min-height: 100%;
}

.view-container {
  display: flex;
  flex-direction: column;
  gap: 16px;
  width: 100%;
  max-width: 480px;
  margin: 0 auto;
}

header.view-header {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

header.view-header h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

header.view-header p {
  margin: 4px 0 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
  max-width: 52ch;
}

.sections-container {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.section-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.section-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.section-title {
  margin: 0;
  font-size: 11px;
  font-weight: 600;
  color: var(--muted-foreground);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.add-btn {
  font-size: 11px;
  height: 26px;
}

.cards-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.provider-nav-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--sidebar) 55%, transparent);
  cursor: pointer;
  text-align: left;
  transition: background-color 0.2s, border-color 0.2s, transform 0.15s;
  width: 100%;
}

.provider-nav-card:hover {
  background: color-mix(in srgb, var(--sidebar) 85%, transparent);
  border-color: color-mix(in srgb, var(--primary) 30%, var(--border));
}

.provider-nav-card:active {
  transform: scale(0.998);
}

.card-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.icon-wrapper {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--primary) 10%, transparent);
  color: var(--primary);
  flex-shrink: 0;
}

.card-text h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--foreground);
}

.card-text p {
  margin: 1px 0 0;
  font-size: 10px;
  color: var(--muted-foreground);
}

.card-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.status-badge {
  font-size: 9px;
  font-weight: 500;
  padding: 1px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--muted) 80%, transparent);
  color: var(--muted-foreground);
  border: 1px solid var(--border);
}

.status-badge.configured {
  background: color-mix(in srgb, var(--primary) 10%, transparent);
  color: var(--primary);
  border-color: color-mix(in srgb, var(--primary) 20%, var(--border));
}

.arrow-icon {
  transition: transform 0.2s;
}

.provider-nav-card:hover .arrow-icon {
  transform: translateX(2px);
}

.empty-state {
  padding: 20px 0;
  text-align: center;
  border: 1px dashed var(--border);
  border-radius: 8px;
}

header.view-header.edit-header {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
  width: 100%;
  gap: 12px;
}

.header-left {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
}

.back-btn {
  font-size: 12px;
  height: 28px;
}

.header-details h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.edit-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.edit-title-icon {
  flex: none;
  color: var(--primary);
}

.header-details p {
  margin: 2px 0 0;
  color: var(--muted-foreground);
  font-size: 11px;
}

.delete-top-btn {
  font-size: 11px;
  height: 28px;
  margin-top: 4px;
}

.edit-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 480px;
  width: 100%;
}

.field-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-row label {
  font-size: 11px;
  font-weight: 500;
  color: var(--muted-foreground);
}

.field-hint {
  margin: 0;
  font-size: 10px;
  line-height: 1.4;
  color: var(--muted-foreground);
}

.oauth-status {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--sidebar) 55%, transparent);
}

.oauth-status-label {
  margin: 0;
  font-size: 10px;
  color: var(--muted-foreground);
}

.oauth-status-value {
  margin: 0;
  font-size: 12px;
  font-weight: 500;
}

.oauth-error {
  margin: 4px 0 0;
  font-size: 11px;
  color: var(--destructive, #ef4444);
  line-height: 1.4;
}

.custom-textarea {
  display: flex;
  min-height: 100px;
  width: 100%;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: transparent;
  padding: 8px 10px;
  font-size: 12px;
  font-family: monospace;
  color: var(--foreground);
  resize: vertical;
  outline: none;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.custom-textarea:focus {
  border-color: var(--ring);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--ring) 20%, transparent);
}

.custom-textarea::placeholder {
  color: var(--muted-foreground);
  opacity: 0.8;
}

.form-actions {
  display: flex;
  gap: 12px;
  width: 100%;
}

.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.15s ease-out;
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateX(4px);
}

.fade-slide-leave-to {
  opacity: 0;
  transform: translateX(-4px);
}
</style>
