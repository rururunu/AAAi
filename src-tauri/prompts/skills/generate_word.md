---
name: generate_word
description: Generate .docx Word documents with python-docx. Use when the user asks for Word、docx、Word 文档、导出文档.
---

# Generate Word document

You produce a real `.docx` file in the workspace (not Markdown pretending to be Word).

## Rules
1. Prefer **python-docx**. If missing: `pip install python-docx` via `run_shell`, then continue.
2. Write a short Python script with `write_file`, then run it with `run_shell` (`python path/to/script.py`). Avoid huge one-liners.
3. Save the `.docx` under the workspace (e.g. `docs/` or user-specified path). Create parent folders if needed.
4. Use UTF-8 source files. For East Asian text, set a font that exists on Windows (e.g. `微软雅黑` / `Microsoft YaHei`) on runs that need it.
5. Do not claim success until the file exists on disk. Return the **absolute or workspace-relative path**.
6. Cap tool rounds: about **4–8** calls. Reuse one script; iterate with small edits.

## Document quality
- Clear title + heading hierarchy (`Heading 1/2/3`)
- Short paragraphs; use lists/tables when structure helps
- Page margins ~2.5cm unless the user specifies otherwise
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

def set_run_font(run, name="微软雅黑", size=12):
    run.font.name = name
    run.font.size = Pt(size)
    r = run._element
    r.rPr.rFonts.set(qn("w:eastAsia"), name)

title = doc.add_heading("标题", level=0)
for run in title.runs:
    set_run_font(run, size=18)

p = doc.add_paragraph()
run = p.add_run("正文内容")
set_run_font(run)

doc.save("output.docx")
print("saved: output.docx")
```

## Output
Reply with:
- path to the `.docx`
- 1–3 bullet summary of what the document contains
- any assumptions (filename, sections) if the user left them open
