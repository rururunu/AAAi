# -*- coding: utf-8 -*-
"""docx 结构解析：大纲线索、真表格、字体样式统计。

不依赖打开 Word 应用；直接读 OOXML（zip + document.xml）。
用于：仿写前读懂参考文件；生成后做门禁输入。
"""

from __future__ import annotations

import re
import zipfile
from collections import Counter
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional
from xml.etree import ElementTree as ET

W_NS = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
NS = {"w": W_NS}
W = f"{{{W_NS}}}"

# 半小时日程常见写法：08:00-08:30 / 08:00—08:30 / 08:00~08:30
HALF_HOUR_RE = re.compile(
    r"(?P<a>\d{1,2}:\d{2})\s*[-—–~～至到]\s*(?P<b>\d{1,2}:\d{2})"
)


@dataclass
class DocxStructure:
    path: str
    paragraph_count: int = 0
    char_count: int = 0
    table_count: int = 0
    table_row_count: int = 0
    table_cell_count: int = 0
    half_hour_row_count: int = 0
    time_mention_count: int = 0
    outline: List[str] = field(default_factory=list)
    fonts: Dict[str, int] = field(default_factory=dict)
    font_sizes_half_points: Dict[str, int] = field(default_factory=dict)
    paragraph_styles: Dict[str, int] = field(default_factory=dict)
    # 按标题线索粗切的章节字符数（仅供门禁参考）
    section_chars: Dict[str, int] = field(default_factory=dict)
    full_text: str = ""

    def to_dict(self, *, include_text: bool = False) -> Dict[str, Any]:
        data = asdict(self)
        if not include_text:
            data.pop("full_text", None)
        return data


def _para_text(paragraph: ET.Element) -> str:
    return "".join(node.text or "" for node in paragraph.findall(".//w:t", NS))


def _para_style(paragraph: ET.Element) -> Optional[str]:
    style = paragraph.find("w:pPr/w:pStyle", NS)
    if style is None:
        return None
    return style.get(f"{W}val")


def inspect_docx(path: str | Path) -> DocxStructure:
    """解析 docx，返回结构摘要。"""
    target = Path(path)
    with zipfile.ZipFile(target) as archive:
        root = ET.fromstring(archive.read("word/document.xml"))

    paragraphs = root.findall(".//w:p", NS)
    tables = root.findall(".//w:tbl", NS)

    texts: List[str] = []
    fonts: Counter[str] = Counter()
    sizes: Counter[str] = Counter()
    styles: Counter[str] = Counter()
    outline: List[str] = []

    for paragraph in paragraphs:
        text = _para_text(paragraph).strip()
        style = _para_style(paragraph) or "None"
        styles[style] += 1
        if not text:
            continue
        texts.append(text)

        # 字体统计
        for run in paragraph.findall("w:r", NS):
            r_pr = run.find("w:rPr", NS)
            if r_pr is None:
                continue
            sz = r_pr.find("w:sz", NS)
            if sz is not None and sz.get(f"{W}val"):
                sizes[sz.get(f"{W}val")] += 1
            r_fonts = r_pr.find("w:rFonts", NS)
            if r_fonts is not None:
                name = r_fonts.get(f"{W}eastAsia") or r_fonts.get(f"{W}ascii")
                if name:
                    fonts[name] += 1

        # 粗大纲：短标题或「一、」「（一）」「①」等
        if len(text) <= 48 and (
            re.match(r"^[一二三四五六七八九十]+[、．.]", text)
            or re.match(r"^第[一二三四五六七八九十\d]+", text)
            or re.match(r"^[①②③④⑤⑥⑦⑧⑨⑩]", text)
            or re.match(r"^[（(][一二三四五六七八九十\d]+[）)]", text)
            or ("方案" in text and len(text) < 36)
            or re.search(r"（\d+分）", text)
        ):
            outline.append(text)

    full_text = "\n".join(texts)
    half_hour = len(HALF_HOUR_RE.findall(full_text))
    time_mentions = len(re.findall(r"\d{1,2}:\d{2}", full_text))

    table_rows = 0
    table_cells = 0
    for table in tables:
        rows = table.findall("./w:tr", NS)
        table_rows += len(rows)
        for row in rows:
            table_cells += len(row.findall("./w:tc", NS))

    section_chars = _estimate_section_chars(full_text)

    return DocxStructure(
        path=str(target),
        paragraph_count=len(texts),
        char_count=len(full_text),
        table_count=len(tables),
        table_row_count=table_rows,
        table_cell_count=table_cells,
        half_hour_row_count=half_hour,
        time_mention_count=time_mentions,
        outline=outline[:120],
        fonts=dict(fonts.most_common(20)),
        font_sizes_half_points=dict(sizes.most_common(20)),
        paragraph_styles=dict(styles.most_common(20)),
        section_chars=section_chars,
        full_text=full_text,
    )


def _estimate_section_chars(full_text: str) -> Dict[str, int]:
    """按常见十章标题粗切字符数（找不到则跳过）。"""
    patterns = [
        ("投保方案", r"[①一]、?\s*投保方案"),
        ("活动方案", r"[②二]、?\s*活动方案"),
        ("组织与管理方案", r"[③三]、?\s*组织与管理方案"),
        ("出行方案", r"[④四]、?\s*出行方案"),
        ("食宿交通方案", r"[⑤五]、?\s*食宿交通方案"),
        ("安全保障方案", r"[⑥六]、?\s*安全保障方案"),
        ("活动物资", r"[⑦七]、?\s*活动物资"),
        ("研学成果", r"[⑧八]、?\s*研学成果"),
        ("档案管理", r"[⑨九]、?\s*档案管理"),
        ("服务承诺", r"[⑩十]、?\s*服务承诺"),
    ]
    hits: List[tuple[int, str]] = []
    for name, pattern in patterns:
        match = re.search(pattern, full_text)
        if match:
            hits.append((match.start(), name))
    hits.sort()
    result: Dict[str, int] = {}
    for idx, (start, name) in enumerate(hits):
        end = hits[idx + 1][0] if idx + 1 < len(hits) else len(full_text)
        result[name] = end - start
    return result


def summarize_for_prompt(structure: DocxStructure, *, max_outline: int = 40) -> str:
    """生成可注入模型上下文的短摘要。"""
    lines = [
        f"文件: {structure.path}",
        f"字符数: {structure.char_count}",
        f"段落: {structure.paragraph_count}",
        f"真表格: {structure.table_count}（行 {structure.table_row_count}）",
        f"半小时日程匹配: {structure.half_hour_row_count}",
        f"时刻提及: {structure.time_mention_count}",
        f"字体: {structure.fonts}",
        "大纲线索:",
    ]
    for item in structure.outline[:max_outline]:
        lines.append(f"- {item}")
    if structure.section_chars:
        lines.append("章节粗估字数:")
        for key, value in structure.section_chars.items():
            lines.append(f"- {key}: {value}")
    return "\n".join(lines)
