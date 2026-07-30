import { loadVscodeTheme } from "@/services/ipc";

export interface VscodeThemeSummary {
  id: string;
  label: string;
  extensionName: string;
  kind: "dark" | "light" | "high-contrast";
}

export interface ResolvedVscodeTheme extends VscodeThemeSummary {
  colors: Record<string, string>;
  tokenColors: Record<string, string>;
}

const THEME_VARIABLES = [
  "--peek-bg", "--peek-sidebar", "--peek-surface", "--peek-input-bg", "--peek-text",
  "--peek-muted", "--peek-faint", "--peek-accent", "--peek-border", "--peek-list-bg",
  "--peek-icon", "--peek-active-fg", "--peek-strong-border",
  "--peek-list-active", "--peek-placeholder", "--peek-send-bg", "--peek-send-fg",
  "--peek-send-active-bg", "--peek-send-active-fg", "--peek-user-bubble-bg",
  "--peek-user-bubble-border", "--peek-user-bubble-text", "--peek-warning", "--peek-danger",
  "--peek-code-bg", "--peek-code-fg", "--peek-code-muted", "--peek-code-icon", "--peek-code-border",
  "--peek-code-toolbar-bg", "--peek-code-hover-bg", "--peek-code-selection",
  "--peek-syntax-comment", "--peek-syntax-keyword", "--peek-syntax-string",
  "--peek-syntax-regexp", "--peek-syntax-number", "--peek-syntax-literal",
  "--peek-syntax-function", "--peek-syntax-variable", "--peek-syntax-type",
  "--peek-syntax-property", "--peek-syntax-attribute", "--peek-syntax-tag",
  "--peek-syntax-selector", "--peek-syntax-meta", "--peek-syntax-operator",
] as const;

let applySequence = 0;

function first(colors: Record<string, string>, ...keys: string[]) {
  for (const key of keys) {
    const value = colors[key];
    if (value) return value;
  }
  return undefined;
}

type Rgb = { r: number; g: number; b: number; a: number };

function parseColor(value: string | undefined): Rgb | undefined {
  if (!value || value === "transparent") return undefined;
  const hex = value.trim().match(/^#([0-9a-f]{3,8})$/i)?.[1];
  if (hex) {
    const expanded = hex.length <= 4 ? [...hex].map((part) => part + part).join("") : hex;
    if (expanded.length === 6 || expanded.length === 8) {
      return {
        r: Number.parseInt(expanded.slice(0, 2), 16),
        g: Number.parseInt(expanded.slice(2, 4), 16),
        b: Number.parseInt(expanded.slice(4, 6), 16),
        a: expanded.length === 8 ? Number.parseInt(expanded.slice(6, 8), 16) / 255 : 1,
      };
    }
  }
  const rgba = value.trim().match(/^rgba?\(\s*([\d.]+)[, ]+([\d.]+)[, ]+([\d.]+)(?:\s*[,/]\s*([\d.]+%?))?\s*\)$/i);
  if (!rgba) return undefined;
  return {
    r: Number(rgba[1]),
    g: Number(rgba[2]),
    b: Number(rgba[3]),
    a: rgba[4] === undefined
      ? 1
      : Math.min(1, rgba[4].endsWith("%") ? Number(rgba[4].slice(0, -1)) / 100 : Number(rgba[4])),
  };
}

function composite(color: Rgb, background: Rgb): Rgb {
  const alpha = color.a + background.a * (1 - color.a);
  if (alpha === 0) return { r: 0, g: 0, b: 0, a: 0 };
  return {
    r: (color.r * color.a + background.r * background.a * (1 - color.a)) / alpha,
    g: (color.g * color.a + background.g * background.a * (1 - color.a)) / alpha,
    b: (color.b * color.a + background.b * background.a * (1 - color.a)) / alpha,
    a: alpha,
  };
}

function luminance(color: Rgb) {
  const channel = (value: number) => {
    const normalized = value / 255;
    return normalized <= 0.04045 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b);
}

function contrastRatio(foreground: string | undefined, background: string | undefined) {
  const fg = parseColor(foreground);
  const bg = parseColor(background);
  if (!fg || !bg) return 0;
  const resolved = fg.a < 1 ? composite(fg, bg) : fg;
  const lighter = Math.max(luminance(resolved), luminance(bg));
  const darker = Math.min(luminance(resolved), luminance(bg));
  return (lighter + 0.05) / (darker + 0.05);
}

function ensureContrast(
  candidate: string | undefined,
  background: string | undefined,
  fallback: string,
  minimum: number,
) {
  return contrastRatio(candidate, background) >= minimum ? candidate! : fallback;
}

export function clearVscodeThemeOverrides() {
  const root = document.documentElement;
  for (const variable of THEME_VARIABLES) root.style.removeProperty(variable);
  delete root.dataset.vscodeTheme;
}

function setVariable(name: string, value: string | undefined) {
  if (value) document.documentElement.style.setProperty(name, value);
}

function applyResolvedVscodeTheme(theme: ResolvedVscodeTheme) {
  const c = theme.colors;
  const t = theme.tokenColors;
  const root = document.documentElement;
  const light = theme.kind === "light";
  const background = first(c, "editor.background", "panel.background") ?? (light ? "#ffffff" : "#1f1f1f");
  const absoluteForeground = light ? "#202020" : "#ededed";
  const foreground = ensureContrast(first(c, "foreground", "editor.foreground"), background, absoluteForeground, 4.5);
  const muted = ensureContrast(first(c, "descriptionForeground", "sideBar.foreground"), background, foreground, 3);
  const accent = ensureContrast(first(c, "focusBorder", "textLink.foreground", "button.background"), background, foreground, 2.5);
  const themeBorder = first(c, "contrastBorder", "panel.border", "sideBar.border", "editorGroup.border");
  const border = contrastRatio(themeBorder, background) >= 1.2
    ? themeBorder
    : `color-mix(in srgb, ${foreground} 22%, transparent)`;
  const strongBorder = `color-mix(in srgb, ${foreground} 30%, transparent)`;
  const listBackground = first(c, "dropdown.background", "quickInput.background", "editorWidget.background") ?? background;
  const listActive = first(c, "list.activeSelectionBackground", "list.focusBackground", "editor.selectionBackground");
  const safeListActive = contrastRatio(listActive, listBackground) >= 1.12
    ? listActive
    : `color-mix(in srgb, ${accent} 18%, ${listBackground})`;
  const activeForeground = ensureContrast(first(c, "list.activeSelectionForeground"), safeListActive, foreground, 4.5);

  clearVscodeThemeOverrides();
  root.dataset.theme = light ? "light" : "dark";
  root.dataset.vscodeTheme = theme.id;
  root.classList.toggle("dark", !light);
  root.style.colorScheme = light ? "light" : "dark";
  setVariable("--peek-bg", background);
  setVariable("--peek-sidebar", first(c, "sideBar.background", "activityBar.background", "panel.background") ?? background);
  setVariable("--peek-surface", first(c, "editorWidget.background", "panel.background", "editorGroupHeader.tabsBackground") ?? background);
  setVariable("--peek-input-bg", first(c, "input.background", "quickInput.background", "dropdown.background") ?? background);
  setVariable("--peek-text", foreground);
  setVariable("--peek-muted", muted);
  setVariable("--peek-faint", ensureContrast(first(c, "disabledForeground", "editorLineNumber.foreground"), background, muted, 2.2));
  setVariable("--peek-icon", ensureContrast(first(c, "icon.foreground"), background, foreground, 3));
  setVariable("--peek-active-fg", activeForeground);
  setVariable("--peek-accent", accent);
  setVariable("--peek-border", border);
  setVariable("--peek-strong-border", strongBorder);
  setVariable("--peek-list-bg", listBackground);
  setVariable("--peek-list-active", safeListActive);
  setVariable("--peek-placeholder", first(c, "input.placeholderForeground", "descriptionForeground"));
  setVariable("--peek-send-bg", first(c, "button.secondaryBackground", "input.background"));
  setVariable("--peek-send-fg", first(c, "button.secondaryForeground", "descriptionForeground") ?? foreground);
  const activeButtonBackground = first(c, "button.background") ?? accent;
  setVariable("--peek-send-active-bg", activeButtonBackground);
  setVariable("--peek-send-active-fg", ensureContrast(first(c, "button.foreground"), activeButtonBackground, absoluteForeground, 4.5));
  setVariable("--peek-user-bubble-bg", first(c, "list.inactiveSelectionBackground", "editor.selectionBackground"));
  setVariable("--peek-user-bubble-border", first(c, "list.activeSelectionBackground", "focusBorder") ?? border);
  setVariable("--peek-user-bubble-text", foreground);
  setVariable("--peek-warning", first(c, "editorWarning.foreground", "notificationsWarningIcon.foreground"));
  setVariable("--peek-danger", first(c, "errorForeground", "editorError.foreground"));
  const codeBackground = first(c, "editor.background") ?? background;
  const codeForeground = ensureContrast(first(c, "editor.foreground"), codeBackground, foreground, 4.5);
  const codeToolbarBackground = first(c, "editorGroupHeader.tabsBackground", "breadcrumb.background", "editor.background") ?? codeBackground;
  const themeCodeBorder = first(c, "editorWidget.border", "contrastBorder", "panel.border");
  setVariable("--peek-code-bg", codeBackground);
  setVariable("--peek-code-fg", codeForeground);
  setVariable("--peek-code-muted", ensureContrast(first(c, "editorLineNumber.foreground", "descriptionForeground"), codeBackground, codeForeground, 2.5));
  setVariable("--peek-code-icon", ensureContrast(first(c, "icon.foreground", "editor.foreground"), codeToolbarBackground, codeForeground, 3));
  setVariable("--peek-code-border", contrastRatio(themeCodeBorder, codeBackground) >= 1.2 ? themeCodeBorder : strongBorder);
  setVariable("--peek-code-toolbar-bg", codeToolbarBackground);
  setVariable("--peek-code-hover-bg", first(c, "toolbar.hoverBackground", "list.hoverBackground", "editor.selectionBackground"));
  setVariable("--peek-code-selection", first(c, "editor.selectionBackground"));
  for (const name of [
    "comment", "keyword", "string", "regexp", "number", "literal", "function", "variable",
    "type", "property", "attribute", "tag", "selector", "meta", "operator",
  ] as const) {
    setVariable(`--peek-syntax-${name}`, t[name]);
  }
}

export async function applyVscodeTheme(themeId: string): Promise<boolean> {
  const sequence = ++applySequence;
  try {
    const theme = await loadVscodeTheme(themeId);
    if (sequence !== applySequence) return false;
    applyResolvedVscodeTheme(theme);
    return true;
  } catch (error) {
    if (sequence === applySequence) console.warn("VS Code theme unavailable; using built-in fallback", error);
    return false;
  }
}

export function invalidatePendingThemeLoad() {
  applySequence += 1;
}
