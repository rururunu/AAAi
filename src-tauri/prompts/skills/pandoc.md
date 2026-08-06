---
name: pandoc
description: Convert documents between Markdown, DOCX, PDF, HTML, LaTeX with pandoc. Use for md→docx/pdf, docx→md extraction, reference-doc styling, batch format conversion. For OOXML edit/redline use docx; for 技术标 use generate_bid_tech.
---

# Pandoc document conversion

Playbook adapted from [plinde-pandoc](https://github.com/plinde/pandoc). Universal format conversion — prefer this over hand-rolling when the source is already Markdown or you need PDF/HTML export.

## When to use which skill

| Need | Skill |
|---|---|
| **md → docx / pdf / html** | **this skill (`pandoc`)** |
| **docx → md** (read/extract) | **this skill (`pandoc`)** |
| **Edit existing .docx OOXML / tracked changes** | **`docx`** |
| **Build new Word from Python** | **`generate_word`** |
| **技术标 / 评分表** | **`generate_bid_tech`** |

## Prerequisites

```bash
pandoc --version
```

Windows: `winget install JohnMacFarlane.Pandoc` or https://pandoc.org/installing.html

PDF output needs a LaTeX engine (optional): MiKTeX / TeX Live, or use HTML print-to-PDF fallback below.

## Common conversions

### Markdown → Word (.docx)

```bash
pandoc input.md -o output.docx
pandoc input.md --toc -o output.docx
pandoc input.md --reference-doc=template.docx -o output.docx
pandoc input.md -s --metadata title="Document Title" -o output.docx
```

### Markdown → PDF

```bash
pandoc input.md -o output.pdf
pandoc input.md -s --toc --toc-depth=2 -V geometry:margin=1in -o output.pdf
pandoc input.md --pdf-engine=xelatex -V geometry:margin=1in -o output.pdf
```

Use `xelatex` when Unicode (box-drawing, CJK) fails with pdflatex.

### Markdown → HTML

**Always prefer `-f gfm`** for lists and line breaks:

```bash
pandoc -f gfm -s input.md -o output.html
pandoc -f gfm -s --embed-resources --standalone input.md -o output.html
```

### Word → Markdown

```bash
pandoc input.docx -o output.md
pandoc input.docx --track-changes=accept -o output.md
```

## Useful options

| Option | Description |
|---|---|
| `-s` / `--standalone` | Full document with header/footer |
| `--toc` | Table of contents |
| `--reference-doc=FILE` | Style from template docx |
| `--number-sections` | Number headings |
| `-f gfm` | GitHub Flavored Markdown input |
| `--track-changes=accept\|reject\|all` | Track changes mode |

## Troubleshooting

**Lists merge into one paragraph:** use `-f gfm`.

**Tables not rendering:** use pipe tables:

```markdown
| A | B |
|---|---|
| 1 | 2 |
```

**No LaTeX for PDF:** convert to HTML then print to PDF from browser, or install MiKTeX.

## Workflow with other skills

1. **Draft in Markdown** → `pandoc` → `.docx`
2. **Polish / redline in Word** → `#skill:docx` for OOXML edits
3. **Technical bid** → do not stop at pandoc; use `generate_bid_tech` for table-first delivery

## Output

Return output file path(s), pandoc command used, and any missing dependency if conversion failed.
