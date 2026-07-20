<template>
  <div class="markdown-body" v-html="html" @click="onLinkClick" />
</template>

<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/common";
import markedKatex from "marked-katex-extension";
import { marked } from "marked";
import { computed } from "vue";
import "katex/dist/katex.min.css";

const props = defineProps<{
  content: string;
}>();

const renderer = new marked.Renderer();

marked.use(markedKatex({
  nonStandard: true,
  throwOnError: false,
}));

renderer.code = ({ text, lang }) => {
  const language = lang?.trim().split(/\s+/)[0].toLowerCase();
  const highlighted = language && hljs.getLanguage(language)
    ? hljs.highlight(text, { language }).value
    : hljs.highlightAuto(text).value;
  const languageClass = language ? ` language-${language}` : "";

  return `<pre><code class="hljs${languageClass}">${highlighted}</code></pre>\n`;
};

marked.setOptions({
  breaks: true,
  gfm: true,
  renderer,
});

const html = computed(() => {
  const raw = marked.parse(normalizeLegacyMath(props.content || ""), { async: false }) as string;
  return DOMPurify.sanitize(raw, {
    ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto|tel|file|sms):|[^&#]*?:|data:image\/)/i
  });
});

async function onLinkClick(event: MouseEvent) {
  const target = event.target;
  if (!(target instanceof Element)) return;
  const anchor = target.closest("a");
  if (!(anchor instanceof HTMLAnchorElement)) return;

  const href = anchor.getAttribute("href")?.trim();
  if (!href || href.startsWith("#")) return;
  if (!/^(https?:|mailto:|tel:)/i.test(href)) return;

  event.preventDefault();
  event.stopPropagation();
  try {
    await openUrl(href);
  } catch (error) {
    console.error("failed to open url in default browser:", href, error);
  }
}

function normalizeLegacyMath(content: string) {
  return content
    .split(/(```[\s\S]*?```|~~~[\s\S]*?~~~)/g)
    .map((part, index) => {
      if (index % 2 === 1) {
        return part;
      }

      const escapedBlocks = part.replace(
        /\\\[\s*([\s\S]*?)\s*\\\]/g,
        (match, formula: string) => isLikelyTex(formula) ? asDisplayMath(formula) : match,
      );
      const withBlocks = escapedBlocks.replace(
        /^\s*\[\s*\r?\n([\s\S]*?)\r?\n\s*\]\s*$/gm,
        (match, formula: string) => isLikelyTex(formula) ? asDisplayMath(formula) : match,
      );

      return withBlocks
        .replace(/\\\(\s*([\s\S]*?)\s*\\\)/g, (_match, formula: string) => `$${formula.trim()}$`)
        .replace(/\(\s*([^()\r\n]+?)\s*\)/g, (match, formula: string) =>
          isLikelyTex(formula) ? `$${formula.trim()}$` : match,
        );
    })
    .join("");
}

function asDisplayMath(value: string) {
  let formula = value.trim();
  if (/\\begin\{aligned\}/.test(formula)) {
    formula = formula.replace(/(?<!\\)\\\s*$/gm, "\\\\");
  }
  return `\n$$\n${formula}\n$$\n`;
}

function isLikelyTex(value: string) {
  const text = value.trim();
  return /\\[a-zA-Z]+|[_^=]/.test(text) || /^[a-zA-Z](?:_\{[^}]+\})?$/.test(text);
}
</script>


<style scoped>
.markdown-body {
  font-size: 13px;
  line-height: 1.65;
  color: var(--peek-text);
  overflow-wrap: anywhere;
}.markdown-body :deep(img) {
  max-width: 100%;
  max-height: 280px;
  border-radius: 6px;
  object-fit: contain;
  margin: 8px 0;
  border: 1px solid color-mix(in srgb, var(--peek-border) 40%, transparent);
}

.markdown-body :deep(p) {
  margin: 0 0 0.65em;
}

.markdown-body :deep(p:last-child) {
  margin-bottom: 0;
}

.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3),
.markdown-body :deep(h4) {
  margin: 1em 0 0.45em;
  color: var(--peek-text);
  font-weight: 650;
  line-height: 1.3;
}

.markdown-body :deep(h1:first-child),
.markdown-body :deep(h2:first-child),
.markdown-body :deep(h3:first-child) {
  margin-top: 0;
}

.markdown-body :deep(h1) { font-size: 1.35em; }
.markdown-body :deep(h2) { font-size: 1.2em; }
.markdown-body :deep(h3) { font-size: 1.08em; }
.markdown-body :deep(h4) { font-size: 1em; }

.markdown-body :deep(pre) {
  margin: 0.65em 0;
  padding: 10px 12px;
  border: 1px solid var(--peek-border);
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-input-bg) 82%, transparent);
  overflow-x: auto;
  line-height: 1.55;
  tab-size: 2;
}

.markdown-body :deep(code) {
  font-family: var(--font-mono);
  font-size: 12px;
}

.markdown-body :deep(pre code) {
  display: block;
  min-width: max-content;
  background: transparent;
  padding: 0;
  color: var(--peek-text);
}

.markdown-body :deep(:not(pre) > code) {
  padding: 1px 4px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-text) 10%, transparent);
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  margin: 0.3em 0 0.65em;
  padding-left: 1.45em;
}

.markdown-body :deep(li + li) {
  margin-top: 0.2em;
}

.markdown-body :deep(li > p) {
  margin-bottom: 0.25em;
}

.markdown-body :deep(input[type="checkbox"]) {
  margin: 0 0.4em 0 -1.2em;
  accent-color: var(--peek-accent);
}

.markdown-body :deep(a) {
  color: var(--peek-accent);
  text-decoration: underline;
  text-decoration-color: color-mix(in srgb, var(--peek-accent) 45%, transparent);
  text-underline-offset: 2px;
}

.markdown-body :deep(blockquote) {
  margin: 0.65em 0;
  padding: 0.15em 0 0.15em 0.8em;
  border-left: 3px solid color-mix(in srgb, var(--peek-accent) 55%, transparent);
  color: var(--peek-muted);
}

.markdown-body :deep(blockquote > :last-child) {
  margin-bottom: 0;
}

.markdown-body :deep(hr) {
  margin: 1em 0;
  border: 0;
  border-top: 1px solid var(--peek-border);
}

.markdown-body :deep(table) {
  display: block;
  width: max-content;
  max-width: 100%;
  margin: 0.65em 0;
  border-collapse: collapse;
  overflow-x: auto;
}

.markdown-body :deep(.katex-display) {
  max-width: 100%;
  margin: 0.8em 0;
  overflow-x: auto;
  overflow-y: hidden;
  padding: 0.15em 0;
}

.markdown-body :deep(.katex-display > .katex) {
  min-width: max-content;
  text-align: center;
}

.markdown-body :deep(.katex) {
  font-size: 1.05em;
}

.markdown-body :deep(th),
.markdown-body :deep(td) {
  padding: 5px 9px;
  border: 1px solid var(--peek-border);
  text-align: left;
}

.markdown-body :deep(th) {
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
  font-weight: 650;
}

.markdown-body :deep(.hljs-comment),
.markdown-body :deep(.hljs-quote) { color: #7f8c98; font-style: italic; }
.markdown-body :deep(.hljs-keyword),
.markdown-body :deep(.hljs-selector-tag),
.markdown-body :deep(.hljs-literal),
.markdown-body :deep(.hljs-type) { color: #c792ea; }
.markdown-body :deep(.hljs-string),
.markdown-body :deep(.hljs-regexp),
.markdown-body :deep(.hljs-addition),
.markdown-body :deep(.hljs-attribute) { color: #addb67; }
.markdown-body :deep(.hljs-number),
.markdown-body :deep(.hljs-symbol),
.markdown-body :deep(.hljs-bullet) { color: #f78c6c; }
.markdown-body :deep(.hljs-title),
.markdown-body :deep(.hljs-section),
.markdown-body :deep(.hljs-function .hljs-title) { color: #82aaff; }
.markdown-body :deep(.hljs-variable),
.markdown-body :deep(.hljs-template-variable),
.markdown-body :deep(.hljs-params) { color: #f07178; }
.markdown-body :deep(.hljs-built_in),
.markdown-body :deep(.hljs-meta),
.markdown-body :deep(.hljs-link) { color: #ffcb6b; }
.markdown-body :deep(.hljs-deletion) { color: #ff5370; }
.markdown-body :deep(pre code.language-diff .hljs-addition) {
  display: inline-block;
  min-width: 100%;
  background: color-mix(in srgb, #22c55e 18%, transparent);
}
.markdown-body :deep(pre code.language-diff .hljs-deletion) {
  display: inline-block;
  min-width: 100%;
  background: color-mix(in srgb, #ef4444 18%, transparent);
}

:global([data-theme="light"]) .markdown-body :deep(.hljs-comment),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-comment),
:global([data-theme="light"]) .markdown-body :deep(.hljs-quote),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-quote) { color: #6a737d; }
:global([data-theme="light"]) .markdown-body :deep(.hljs-keyword),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-keyword),
:global([data-theme="light"]) .markdown-body :deep(.hljs-type),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-type) { color: #8250df; }
:global([data-theme="light"]) .markdown-body :deep(.hljs-string),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-string),
:global([data-theme="light"]) .markdown-body :deep(.hljs-addition),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-addition) { color: #116329; }
:global([data-theme="light"]) .markdown-body :deep(.hljs-number),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-number),
:global([data-theme="light"]) .markdown-body :deep(.hljs-variable),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-variable) { color: #953800; }
:global([data-theme="light"]) .markdown-body :deep(.hljs-title),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-title),
:global([data-theme="light"]) .markdown-body :deep(.hljs-section),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-section) { color: #0550ae; }
:global([data-theme="light"]) .markdown-body :deep(.hljs-built_in),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-built_in),
:global([data-theme="light"]) .markdown-body :deep(.hljs-meta),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-meta) { color: #9a6700; }
:global([data-theme="light"]) .markdown-body :deep(.hljs-deletion),
:global([data-theme="cream"]) .markdown-body :deep(.hljs-deletion) { color: #cf222e; }
</style>
