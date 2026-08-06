---
name: generate_word
description: Generate .docx Word documents with python-docx. Use when the user asks for Word、docx、Word 文档、导出文档. For 技术标/综合评分表/投标技术方案 prefer generate_bid_tech.
---

# Generate Word document

You produce a real `.docx` file in the workspace (not Markdown pretending to be Word).

## When to use which skill
- **General Word / short reports / letters** → this skill (`generate_word`)
- **Edit existing .docx / tracked changes / OOXML / comments** → `#skill:docx`
- **Markdown ↔ DOCX/PDF/HTML conversion** → `#skill:pandoc`
- **Technical bid / 技术标 / 综合评分技术部分** → `generate_bid_tech` (table-first planner + gate)

## Rules
1. Prefer **python-docx**. If it is missing and package installation is within the user's requested workflow, install it with `run_shell`; otherwise report the dependency clearly.
2. Write a short Python script with `write_file`, then run it with `run_shell` (`python path/to/script.py`). Avoid huge one-liners.
3. Save the `.docx` under the workspace (e.g. `docs/` or user-specified path). Create parent folders if needed.
4. Use UTF-8 source files. For East Asian text, set both Latin and East-Asian font names on each run (e.g. `宋体` / `仿宋` / `黑体` / `微软雅黑`).
5. Do not claim success until the file exists on disk. Return the **absolute or workspace-relative path**.
6. Reuse one script and iterate with small edits. Stop only after the document has been opened or rendered enough to verify its structure and readability.
7. **Tables are first-class**: any row/column structure must use `Document.add_table` (or helpers). Never simulate a table with tab-separated paragraphs.
8. **Table borders**: prefer `table.style = "Table Grid"` or bid_tech `style.apply_grid_borders(table)` so cells show full grid lines by default.

## Document quality
- Clear title + heading hierarchy
- Short paragraphs; use lists/tables when structure helps
- Page margins ~2.5cm unless the user specifies otherwise
- Prefer consistent fonts within a block (set font on every run you create)
- No placeholder lorem unless asked

## Minimal script pattern

```python
from docx import Document
from docx.shared import Pt, Cm
from docx.oxml.ns import qn

doc = Document()
section = doc.sections[0]
section.top_margin = Cm(2.5)
section.bottom_margin = Cm(2.5)
section.left_margin = Cm(2.5)
section.right_margin = Cm(2.5)

def set_run_font(run, name="仿宋", size=12):
    run.font.name = name
    run.font.size = Pt(size)
    r = run._element.get_or_add_rPr().get_or_add_rFonts()
    r.set(qn("w:ascii"), name)
    r.set(qn("w:hAnsi"), name)
    r.set(qn("w:eastAsia"), name)

title = doc.add_paragraph()
run = title.add_run("标题")
set_run_font(run, name="黑体", size=18)

p = doc.add_paragraph()
run = p.add_run("正文内容")
set_run_font(run)

# Real table example (with grid borders)
table = doc.add_table(rows=2, cols=2)
table.style = "Table Grid"
table.rows[0].cells[0].text = "列A"
table.rows[0].cells[1].text = "列B"
# Re-apply East-Asian fonts on cell runs after .text assignment if needed.

doc.save("output.docx")
print("saved: output.docx")
```

## Output
Reply with:
- path to the `.docx`
- 1–3 bullet summary of what the document contains
- any assumptions (filename, sections) if the user left them open
