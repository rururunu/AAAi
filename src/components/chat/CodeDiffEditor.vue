<template>
  <div class="code-diff-editor" :class="`is-${viewMode}`">
    <div v-if="loading" class="code-diff-loading" />
    <div v-else-if="error" class="code-diff-error">{{ error }}</div>
    <template v-else-if="document">
      <div v-if="viewMode === 'split'" class="split-editors">
        <div ref="leftHost" class="editor-pane" />
        <div ref="rightHost" class="editor-pane editor-pane-right" />
      </div>
      <div v-else ref="unifiedHost" class="editor-pane unified-editor" />
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { Compartment, EditorState, RangeSetBuilder, type Extension } from "@codemirror/state";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { Decoration, EditorView, keymap, lineNumbers } from "@codemirror/view";
import { defaultKeymap } from "@codemirror/commands";
import { tags } from "@lezer/highlight";
import { cpp } from "@codemirror/lang-cpp";
import { css } from "@codemirror/lang-css";
import { go } from "@codemirror/lang-go";
import { html } from "@codemirror/lang-html";
import { java } from "@codemirror/lang-java";
import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { php } from "@codemirror/lang-php";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { sql } from "@codemirror/lang-sql";
import { xml } from "@codemirror/lang-xml";
import { yaml } from "@codemirror/lang-yaml";
import { buildCodeDiff, type CodeDiffDocument, type CodeDiffLine, type CodeDiffLineKind } from "@/services/chat/codeDiff";

type DiffViewMode = "split" | "unified";

const props = defineProps<{
  oldText?: string | null;
  newText?: string | null;
  unifiedDiff: string;
  language: string;
  viewMode: DiffViewMode;
  wrapLines: boolean;
}>();

const leftHost = ref<HTMLElement | null>(null);
const rightHost = ref<HTMLElement | null>(null);
const unifiedHost = ref<HTMLElement | null>(null);
const document = ref<CodeDiffDocument | null>(null);
const loading = ref(false);
const error = ref("");
const wrapCompartment = new Compartment();
let leftView: EditorView | null = null;
let rightView: EditorView | null = null;
let unifiedView: EditorView | null = null;
let requestVersion = 0;
let syncingScroll = false;

const requestKey = computed(() => JSON.stringify({
  oldText: props.oldText,
  newText: props.newText,
  unifiedDiff: props.unifiedDiff,
}));

watch(requestKey, loadDocument, { immediate: true });
watch(
  () => [loading.value, document.value, props.viewMode, props.language] as const,
  async ([isLoading, currentDocument]) => {
    if (isLoading || !currentDocument) {
      destroyEditors();
      return;
    }
    await nextTick();
    rebuildEditors();
  },
  { flush: "post" },
);
watch(() => props.wrapLines, updateWrap);

onBeforeUnmount(destroyEditors);

async function loadDocument() {
  const version = ++requestVersion;
  loading.value = true;
  error.value = "";
  try {
    const next = await buildCodeDiff({
      oldText: props.oldText,
      newText: props.newText,
      unifiedDiff: props.unifiedDiff,
    });
    if (version !== requestVersion) return;
    document.value = next;
  } catch (cause) {
    if (version === requestVersion) {
      error.value = cause instanceof Error ? cause.message : String(cause);
      document.value = null;
      destroyEditors();
    }
  } finally {
    if (version === requestVersion) loading.value = false;
  }
}

function rebuildEditors() {
  destroyEditors();
  if (!document.value) return;
  if (props.viewMode === "split") {
    if (!leftHost.value || !rightHost.value) return;
    leftView = createEditor(leftHost.value, splitSide("left"));
    rightView = createEditor(rightHost.value, splitSide("right"));
    linkVerticalScroll(leftView, rightView);
    linkVerticalScroll(rightView, leftView);
    return;
  }
  if (unifiedHost.value) unifiedView = createEditor(unifiedHost.value, unifiedLines());
}

function destroyEditors() {
  leftView?.destroy();
  rightView?.destroy();
  unifiedView?.destroy();
  leftView = null;
  rightView = null;
  unifiedView = null;
}

function updateWrap(wrapLines: boolean) {
  const extension = wrapLines ? EditorView.lineWrapping : [];
  for (const view of [leftView, rightView, unifiedView]) {
    view?.dispatch({ effects: wrapCompartment.reconfigure(extension) });
  }
}

function createEditor(parent: HTMLElement, lines: DisplayLine[]) {
  const lineNumbersByDisplayLine = lines.map((line) => line.lineNumber);
  const state = EditorState.create({
    doc: lines.map((line) => line.text).join("\n"),
    extensions: [
      keymap.of(defaultKeymap),
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
      lineNumbers({ formatNumber: (line) => String(lineNumbersByDisplayLine[line - 1] ?? "") }),
      wrapCompartment.of(props.wrapLines ? EditorView.lineWrapping : []),
      languageExtension(props.language),
      syntaxHighlighting(diffHighlightStyle),
      diffTheme,
      lineClasses(lines),
    ],
  });
  const view = new EditorView({ state, parent });
  view.scrollDOM.classList.add("peek-scrollbar");
  return view;
}

type DisplayLine = { text: string; lineNumber?: number; kind?: CodeDiffLineKind };

function splitSide(side: "left" | "right"): DisplayLine[] {
  return document.value!.rows.map((row) => displayLine(row[side]));
}

function unifiedLines(): DisplayLine[] {
  return document.value!.rows.flatMap((row) => {
    if (row.left?.kind === "deletion") return [displayLine(row.left), displayLine(row.right)];
    if (row.right?.kind === "addition") return [displayLine(row.right)];
    return [displayLine(row.left ?? row.right)];
  });
}

function displayLine(line: CodeDiffLine | null | undefined): DisplayLine {
  return line ? { text: line.text, lineNumber: line.lineNumber, kind: line.kind } : { text: "" };
}

function lineClasses(lines: DisplayLine[]): Extension {
  const builder = new RangeSetBuilder<Decoration>();
  let position = 0;
  for (const line of lines) {
    if (line.kind) builder.add(position, position, Decoration.line({ attributes: { class: `diff-${line.kind}` } }));
    position += line.text.length + 1;
  }
  return EditorView.decorations.of(builder.finish());
}

function linkVerticalScroll(source: EditorView, target: EditorView) {
  source.scrollDOM.addEventListener("scroll", () => {
    if (syncingScroll || target.scrollDOM.scrollTop === source.scrollDOM.scrollTop) return;
    syncingScroll = true;
    target.scrollDOM.scrollTop = source.scrollDOM.scrollTop;
    requestAnimationFrame(() => { syncingScroll = false; });
  });
}

function languageExtension(language: string): Extension {
  switch (language) {
    case "javascript": return javascript({ jsx: true });
    case "typescript": return javascript({ jsx: true, typescript: true });
    case "json": return json();
    case "xml": return xml();
    case "css": case "scss": return css();
    case "rust": return rust();
    case "python": return python();
    case "yaml": return yaml();
    case "markdown": return markdown();
    case "sql": return sql();
    case "go": return go();
    case "java": return java();
    case "cpp": case "c": return cpp();
    case "php": return php();
    case "html": return html();
    default: return [];
  }
}

const diffHighlightStyle = HighlightStyle.define([
  { tag: tags.comment, color: "var(--peek-syntax-comment, var(--peek-muted))" },
  { tag: [tags.keyword, tags.operator, tags.controlKeyword], color: "var(--peek-syntax-keyword, var(--peek-accent))" },
  { tag: [tags.string, tags.special(tags.string)], color: "var(--peek-syntax-string, var(--peek-code-fg, var(--peek-text)))" },
  { tag: [tags.number, tags.bool, tags.null], color: "var(--peek-syntax-number, var(--peek-code-fg, var(--peek-text)))" },
  { tag: [tags.function(tags.variableName), tags.labelName], color: "var(--peek-syntax-function, var(--peek-accent))" },
  { tag: [tags.typeName, tags.className], color: "var(--peek-syntax-type, var(--peek-accent))" },
  { tag: tags.variableName, color: "var(--peek-syntax-variable, var(--peek-code-fg, var(--peek-text)))" },
]);

const diffTheme = EditorView.theme({
  "&": { height: "100%", backgroundColor: "transparent", color: "var(--peek-code-fg, var(--peek-text))" },
  ".cm-scroller": { overflow: "auto", fontFamily: "var(--font-mono)", fontSize: "11px", lineHeight: "1.65" },
  ".cm-content": { padding: "0 0 18px", minHeight: "100%", caretColor: "transparent" },
  ".cm-line": { padding: "0 12px", minHeight: "20px" },
  ".cm-gutters": { border: "0", backgroundColor: "color-mix(in srgb, var(--peek-text) 1.8%, transparent)", color: "var(--peek-code-muted, var(--peek-faint))" },
  ".cm-gutterElement": { padding: "0 8px 0 4px", minWidth: "36px" },
  ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "transparent" },
  ".cm-selectionBackground, ::selection": { backgroundColor: "var(--peek-code-selection, var(--peek-list-active)) !important" },
  ".cm-line.diff-addition": { backgroundColor: "color-mix(in srgb, #2ea043 15%, transparent)", boxShadow: "inset 3px 0 0 #2ea043" },
  ".cm-line.diff-deletion": { backgroundColor: "color-mix(in srgb, #f85149 14%, transparent)", boxShadow: "inset 3px 0 0 #f85149" },
});
</script>

<style scoped>
.code-diff-editor { flex: 1; min-height: 0; position: relative; overflow: hidden; background: transparent; }
.split-editors { box-sizing: border-box; width: 100%; height: 100%; display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 6px; padding: 0 6px 6px; }
.editor-pane { min-width: 0; min-height: 0; overflow: hidden; border-radius: 5px; background: color-mix(in srgb, var(--peek-text) 1.2%, transparent); }
.code-diff-loading { position: absolute; inset: 0; background: transparent; }
.code-diff-error { padding: 16px; color: var(--peek-muted); font: 11px/1.5 var(--font-mono); white-space: pre-wrap; }
</style>
