<template>
  <section class="about-page">
    <div class="product-overview">
      <div class="brand-visual" aria-hidden="true">
        <div class="brand-mark">
          <img :src="appIconUrl" alt="" />
        </div>
      </div>

      <div class="product-details">
        <header class="product-header">
          <p class="product-kicker">{{ copy.title }}</p>
          <div class="product-title-row">
            <h1>{{ name }}</h1>
            <span class="version-badge">{{ copy.versionLabel }}</span>
            <span v-if="updaterStore.updateAvailable" class="update-badge">
              {{ copy.updateAvailable }}
            </span>
          </div>
          <p class="product-description">{{ copy.description }}</p>
        </header>

        <section class="application-section">
          <h2>{{ copy.application }}</h2>
          <dl class="product-meta">
            <div>
              <dt>{{ copy.appName }}</dt>
              <dd>{{ name }}</dd>
            </div>
            <div>
              <dt>{{ copy.version }}</dt>
              <dd class="mono">{{ version }}</dd>
            </div>
            <div>
              <dt>{{ copy.identifier }}</dt>
              <dd class="mono">{{ identifier }}</dd>
            </div>
            <div>
              <dt>{{ copy.runtime }}</dt>
              <dd>{{ copy.runtimeValue }}</dd>
            </div>
          </dl>
        </section>

        <section class="update-section">
          <div class="update-copy">
            <h2>{{ copy.updates }}</h2>
            <p v-if="updaterStore.updateAvailable">
              {{ copy.updateAvailableDetail }}
            </p>
            <p v-else-if="updaterStore.status === 'up-to-date'">
              {{ copy.upToDateDetail }}
            </p>
            <p v-else-if="updaterStore.status === 'checking' || updaterStore.isBusy">
              {{ statusCopy }}
            </p>
            <p v-else-if="updaterStore.errorMessage">
              {{ copy.error }}: {{ updaterStore.errorMessage }}
            </p>
            <p v-else>{{ copy.upToDateDetail }}</p>
          </div>

          <div class="update-actions">
            <button
              type="button"
              class="update-button"
              :disabled="updaterStore.isBusy"
              @click="handleCheckUpdate"
            >
              {{ copy.check }}
            </button>
            <button
              v-if="updaterStore.updateAvailable"
              type="button"
              class="update-button primary"
              :disabled="updaterStore.isBusy"
              @click="handleInstallUpdate"
            >
              {{ copy.updateNow }}
            </button>
          </div>
        </section>
      </div>
    </div>

    <section class="privacy-section">
      <span class="privacy-icon"><ShieldCheck :size="17" /></span>
      <div>
        <h2>{{ copy.privacy }}</h2>
        <p>{{ copy.privacyDescription }}</p>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ShieldCheck } from "@lucide/vue";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import { useUpdaterStore } from "@/stores/updater";
import appIconAsset from "../../../src-tauri/icons/AAAi-transparent.svg";

const settingStore = useSettingStore();
const updaterStore = useUpdaterStore();
const props = defineProps<{ name: string; version: string; identifier: string }>();
const versionForCopy = computed(() => props.version || "-");
const appIconUrl = appIconAsset;
const copy = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "about.title"),
    description: tr(language, "about.description"),
    versionLabel: tr(language, "about.versionLabel", { version: versionForCopy.value }),
    application: tr(language, "about.application"),
    appName: tr(language, "about.appName"),
    version: tr(language, "about.version"),
    identifier: tr(language, "about.identifier"),
    runtime: tr(language, "about.runtime"),
    runtimeValue: tr(language, "about.runtimeValue"),
    privacy: tr(language, "about.privacy"),
    privacyDescription: tr(language, "about.privacyDescription"),
    updates: tr(language, "updater.sectionTitle"),
    updateAvailable: tr(language, "updater.updateAvailable"),
    updateAvailableDetail: tr(language, "updater.updateAvailableDetail", {
      version: updaterStore.latestVersion || "?",
    }),
    upToDate: tr(language, "updater.upToDate"),
    upToDateDetail: tr(language, "updater.upToDateDetail"),
    check: tr(language, "updater.check"),
    updateNow: tr(language, "updater.updateNow"),
    error: tr(language, "updater.error"),
  };
});

const statusCopy = computed(() => {
  const language = settingStore.language;
  if (updaterStore.progress.phase === "installing") {
    return tr(language, "updater.installing");
  }
  if (updaterStore.status === "downloading" || updaterStore.progress.phase === "downloading") {
    return tr(language, "updater.downloading");
  }
  return tr(language, "updater.checking");
});

async function handleCheckUpdate() {
  updaterStore.resetTransientError();
  await updaterStore.check();
}

async function handleInstallUpdate() {
  await updaterStore.install();
}
</script>

<style scoped>
.about-page {
  box-sizing: border-box;
  width: min(100%, 920px);
  margin: 0 auto;
  padding: 44px 38px 48px;
  color: var(--peek-text);
}

.product-overview {
  display: grid;
  grid-template-columns: minmax(180px, 0.72fr) minmax(360px, 1.45fr);
  align-items: center;
  gap: clamp(36px, 6vw, 72px);
  padding: 18px 8px 38px;
}

.brand-visual {
  min-width: 0;
  display: grid;
  place-items: center;
}

.brand-mark {
  width: min(100%, 230px);
  aspect-ratio: 1;
  display: grid;
  place-items: center;
}

.brand-mark img {
  width: 100%;
  height: 100%;
  display: block;
  object-fit: contain;
  opacity: 0.92;
}

:global([data-theme="dark"]) .brand-mark img {
  filter: invert(1);
}

.product-details {
  min-width: 0;
}
.product-header {
  padding-bottom: 27px;
}
.product-kicker {
  margin: 0 0 8px;
  color: var(--peek-faint);
  font-size: 10px;
  font-weight: 650;
  text-transform: uppercase;
}

.product-title-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.product-title-row h1 {
  min-width: 0;
  margin: 0;
  font-size: 30px;
  font-weight: 700;
  line-height: 1.15;
  letter-spacing: 0;
}

.version-badge {
  flex: none;
  padding: 4px 7px;
  border: 1px solid var(--peek-border);
  border-radius: 5px;
  background: color-mix(in srgb, var(--peek-text) 3%, transparent);
  color: var(--peek-muted);
  font-size: 10px;
  white-space: nowrap;
}

.update-badge {
  flex: none;
  padding: 4px 7px;
  border: 1px solid color-mix(in srgb, var(--peek-accent) 35%, var(--peek-border));
  border-radius: 5px;
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
  color: var(--peek-accent);
  font-size: 10px;
  white-space: nowrap;
}

.update-section {
  margin-top: 18px;
  padding-top: 18px;
  border-top: 1px solid var(--peek-border);
}

.update-section h2 {
  margin: 0 0 6px;
  font-size: 11px;
  font-weight: 650;
  color: var(--peek-faint);
}

.update-copy p {
  margin: 0;
  max-width: 460px;
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 18px;
}

.update-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
}

.update-button {
  padding: 7px 12px;
  border: 1px solid var(--peek-border);
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-text) 3%, transparent);
  color: var(--peek-text);
  font-size: 11px;
  cursor: pointer;
}

.update-button:hover:not(:disabled) {
  background: var(--peek-hover-bg);
}

.update-button.primary {
  border-color: color-mix(in srgb, var(--peek-accent) 35%, var(--peek-border));
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
  color: var(--peek-accent);
}

.update-button:disabled {
  opacity: 0.65;
  cursor: not-allowed;
}

.product-description {
  max-width: 460px;
  margin: 10px 0 0;
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 19px;
}

.application-section h2,
.privacy-section h2 {
  margin: 0;
  font-size: 11px;
  font-weight: 650;
}

.application-section > h2 {
  margin-bottom: 7px;
  color: var(--peek-faint);
}

.product-meta {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  margin: 0;
  border-top: 1px solid var(--peek-border);
}

.product-meta > div {
  min-width: 0;
  padding: 13px 12px 13px 0;
  border-bottom: 1px solid var(--peek-border);
}

.product-meta > div:nth-child(even) {
  padding-left: 18px;
}
dt {
  color: var(--peek-faint);
  font-size: 10px;
}
dd {
  min-width: 0;
  margin: 5px 0 0;
  overflow-wrap: anywhere;
  color: var(--peek-text);
  font-size: 12px;
  line-height: 17px;
}
.mono {
  font-family: var(--font-mono);
  font-size: 11px;
}

.privacy-section {
  display: flex;
  gap: 11px;
  margin: 0 8px;
  padding: 18px 0 0;
  border-top: 1px solid var(--peek-border);
}
.privacy-icon {
  flex: none;
  color: var(--peek-accent);
}
.privacy-section p {
  max-width: 680px;
  margin: 4px 0 0;
  color: var(--peek-muted);
  font-size: 11px;
  line-height: 17px;
}

@media (max-width: 720px) {
  .about-page {
    padding: 28px 20px 40px;
  }
  .product-overview {
    grid-template-columns: 1fr;
    gap: 24px;
    padding-top: 2px;
  }
  .brand-mark {
    width: 150px;
  }
  .product-header {
    text-align: center;
  }
  .product-title-row {
    justify-content: center;
  }
  .product-description {
    margin-inline: auto;
  }
}

@media (max-width: 480px) {
  .product-meta {
    grid-template-columns: 1fr;
  }
  .product-meta > div:nth-child(even) {
    padding-left: 0;
  }
  .product-title-row {
    align-items: flex-start;
    flex-direction: column;
  }
  .version-badge {
    align-self: center;
  }
}
</style>
