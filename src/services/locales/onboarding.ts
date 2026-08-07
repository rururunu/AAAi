import type { AppLanguage } from "@/types/setting";

export type OnboardingI18nKey =
  | "onboarding.welcomeTitle"
  | "onboarding.welcomeSubtitle"
  | "onboarding.continue"
  | "onboarding.skip"
  | "onboarding.skipTour"
  | "onboarding.back"
  | "onboarding.providerTitle"
  | "onboarding.providerSubtitle"
  | "onboarding.providerDeepSeekHint"
  | "onboarding.providerGeminiHint"
  | "onboarding.providerCustomHint"
  | "onboarding.providerConfigured"
  | "onboarding.providerLater"
  | "onboarding.hotkeyTitle"
  | "onboarding.hotkeySubtitle"
  | "onboarding.hotkeyHint"
  | "onboarding.hotkeyGesture"
  | "onboarding.finish"
  | "onboarding.customName"
  | "onboarding.customBaseUrl"
  | "onboarding.customApiKey"
  | "onboarding.customModels"
  | "onboarding.saveCustom"
  | "onboarding.stepOf";

export const onboardingEn: Record<OnboardingI18nKey, string> = {
  "onboarding.welcomeTitle": "Welcome to AAAi",
  "onboarding.welcomeSubtitle": "Your Windows overlay AI assistant.",
  "onboarding.continue": "Continue",
  "onboarding.skip": "Skip for now",
  "onboarding.skipTour": "Skip guide",
  "onboarding.back": "Back",
  "onboarding.providerTitle": "Connect a provider",
  "onboarding.providerSubtitle":
    "Add a DeepSeek API key, sign in with Google for Gemini, or configure a custom OpenAI-compatible endpoint.",
  "onboarding.providerDeepSeekHint": "Paste your DeepSeek API key",
  "onboarding.providerGeminiHint": "Sign in with Google via Antigravity",
  "onboarding.providerCustomHint": "OpenAI-compatible base URL + key",
  "onboarding.providerConfigured": "Ready",
  "onboarding.providerLater": "You can change providers anytime in Settings.",
  "onboarding.hotkeyTitle": "Summon with Alt · Alt",
  "onboarding.hotkeySubtitle":
    "Double-tap Alt anywhere to open the AAAi overlay. Ask, paste, and keep working without leaving your flow.",
  "onboarding.hotkeyHint": "Primary shortcut — two quick taps (not a long hold)",
  "onboarding.hotkeyGesture": "Alt",
  "onboarding.finish": "Enter workspace",
  "onboarding.customName": "Provider name",
  "onboarding.customBaseUrl": "Base URL",
  "onboarding.customApiKey": "API Key",
  "onboarding.customModels": "Models",
  "onboarding.saveCustom": "Save provider",
  "onboarding.stepOf": "Step {current} of {total}",
};

export const onboardingLocales: Record<AppLanguage, Partial<Record<OnboardingI18nKey, string>>> = {
  "en-US": {},
  "zh-CN": {
    "onboarding.welcomeTitle": "欢迎您使用 AAAi",
    "onboarding.welcomeSubtitle": "Windows 上的悬浮 AI 助手。",
    "onboarding.continue": "继续",
    "onboarding.skip": "稍后再说",
    "onboarding.skipTour": "跳过引导",
    "onboarding.back": "返回",
    "onboarding.providerTitle": "配置模型提供商",
    "onboarding.providerSubtitle":
      "填写 DeepSeek API Key、使用 Google 登录 Gemini，或添加自定义 OpenAI 兼容接口。",
    "onboarding.providerDeepSeekHint": "粘贴 DeepSeek API Key",
    "onboarding.providerGeminiHint": "通过 Antigravity 使用 Google 登录",
    "onboarding.providerCustomHint": "OpenAI 兼容 Base URL + Key",
    "onboarding.providerConfigured": "已就绪",
    "onboarding.providerLater": "之后可随时在设置中修改提供商。",
    "onboarding.hotkeyTitle": "连按 Alt · Alt 呼出",
    "onboarding.hotkeySubtitle":
      "在任意界面连按两次 Alt，即可唤出 AAAi 悬浮窗，提问与粘贴无需打断当前工作。",
    "onboarding.hotkeyHint": "主快捷键：快速连按两下短按（长按无效）",
    "onboarding.hotkeyGesture": "Alt",
    "onboarding.finish": "进入工作区",
    "onboarding.customName": "提供商名称",
    "onboarding.customBaseUrl": "Base URL",
    "onboarding.customApiKey": "API Key",
    "onboarding.customModels": "模型列表",
    "onboarding.saveCustom": "保存提供商",
    "onboarding.stepOf": "第 {current} / {total} 步",
  },
  "ja-JP": {},
  "ru-RU": {},
  "de-DE": {},
  "fr-FR": {},
  "ko-KR": {},
};
