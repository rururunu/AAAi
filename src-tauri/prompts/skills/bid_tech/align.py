# -*- coding: utf-8 -*-
"""招标要求 ↔ 投标响应 双文档对齐检查。

用法：
1. 从招标/参考文件人工或半自动抽出「必须响应」清单（JSON）
2. 对生成的技术标全文做勾选
3. 未勾选项进入门禁 ``align_open_items``，阻止宣称完成

清单项示例::
    {
      "id": "route-sz-bj",
      "requirement": "行程须覆盖深圳与北京研学节点",
      "keywords_any": ["深圳"],
      "keywords_all": ["北京"]
    }
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence

from .docx_inspect import inspect_docx


@dataclass
class AlignItem:
    id: str
    requirement: str
    keywords_any: Sequence[str] = ()
    keywords_all: Sequence[str] = ()
    # 可选：要求正文出现某类表头片段
    table_hints: Sequence[str] = ()


@dataclass
class AlignResult:
    item_id: str
    requirement: str
    matched: bool
    detail: str


@dataclass
class AlignReport:
    source: str
    response_docx: str
    results: List[AlignResult] = field(default_factory=list)

    @property
    def open_items(self) -> List[str]:
        return [
            f"{item.item_id}: {item.requirement} ({item.detail})"
            for item in self.results
            if not item.matched
        ]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "source": self.source,
            "response_docx": self.response_docx,
            "open_count": len(self.open_items),
            "results": [asdict(item) for item in self.results],
            "open_items": self.open_items,
        }

    def format_text(self) -> str:
        lines = [
            f"对齐报告: {self.response_docx}",
            f"清单来源: {self.source}",
            f"未闭合: {len(self.open_items)}",
        ]
        for item in self.results:
            mark = "OK" if item.matched else "GAP"
            lines.append(f"[{mark}] {item.item_id} — {item.requirement}: {item.detail}")
        return "\n".join(lines)


def default_checklist_for_study_tour() -> List[AlignItem]:
    """研学类技术标常用勾选（可被项目清单覆盖；不含具体项目专有场馆名时可删改）。"""
    return [
        AlignItem(
            id="route-shenzhen",
            requirement="路线须含深圳段（以招标/参考为准）",
            keywords_all=("深圳",),
        ),
        AlignItem(
            id="route-beijing",
            requirement="路线须含北京段（以招标/参考为准）",
            keywords_all=("北京",),
        ),
        AlignItem(
            id="half-hour-schedule",
            requirement="行程精确到半小时并体现负责人",
            keywords_all=("半小时", "负责"),
            table_hints=("时间", "负责人员"),
        ),
        AlignItem(
            id="process-artifacts",
            requirement="流程须有形成材料字段",
            keywords_all=("形成材料",),
            table_hints=("责任岗位", "形成材料"),
        ),
        AlignItem(
            id="insurance-100",
            requirement="响应人均每日投保额度要求（正文须出现投保/保险表述）",
            keywords_any=("投保", "保险"),
        ),
        AlignItem(
            id="archive-timing",
            requirement="档案须有形成/更新时点",
            keywords_any=("形成/更新时点", "形成时点", "归档"),
        ),
        AlignItem(
            id="emergency-owner",
            requirement="应急须定第一责任人",
            keywords_all=("责任人",),
            table_hints=("第一责任人",),
        ),
    ]


def load_checklist(path: str | Path) -> List[AlignItem]:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    items = data.get("items") if isinstance(data, dict) else data
    result: List[AlignItem] = []
    for raw in items or []:
        result.append(
            AlignItem(
                id=str(raw["id"]),
                requirement=str(raw["requirement"]),
                keywords_any=tuple(raw.get("keywords_any") or ()),
                keywords_all=tuple(raw.get("keywords_all") or ()),
                table_hints=tuple(raw.get("table_hints") or ()),
            )
        )
    return result


def save_checklist(items: Sequence[AlignItem], path: str | Path) -> Path:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    payload = {"items": [asdict(item) for item in items]}
    target.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    return target


def check_alignment(
    response_docx: str | Path,
    items: Sequence[AlignItem],
    *,
    source: str = "inline",
) -> AlignReport:
    structure = inspect_docx(response_docx)
    text = structure.full_text
    results: List[AlignResult] = []

    for item in items:
        missing_all = [kw for kw in item.keywords_all if kw not in text]
        any_ok = (not item.keywords_any) or any(kw in text for kw in item.keywords_any)
        hints_ok = (not item.table_hints) or all(hint in text for hint in item.table_hints)

        if missing_all:
            matched = False
            detail = f"缺少必现词: {', '.join(missing_all)}"
        elif not any_ok:
            matched = False
            detail = f"keywords_any 均未出现: {', '.join(item.keywords_any)}"
        elif not hints_ok:
            matched = False
            detail = f"表头线索缺失: {', '.join(item.table_hints)}"
        else:
            matched = True
            detail = "已覆盖"

        results.append(
            AlignResult(
                item_id=item.id,
                requirement=item.requirement,
                matched=matched,
                detail=detail,
            )
        )

    return AlignReport(
        source=source,
        response_docx=str(response_docx),
        results=results,
    )
