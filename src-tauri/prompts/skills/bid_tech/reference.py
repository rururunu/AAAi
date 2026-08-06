# -*- coding: utf-8 -*-
"""从优质参考 .docx 提取「可迁移画像」，而非硬编码某一标书内容。

目标：
- 识别表头范式（流程/日程/物资/应急…）供仿写
- 判断日程粒度（按天块 / 半小时密集 / 混合）
- 给出路线节点、人员配比、关键词等软约束
- 将门禁阈值校准到「参考稿水准 ± 容差」，换项目时可被招标要求覆盖

用法::

    python .aaai/bid_tech/cli.py reference path/to/参考.docx --out .aaai/ref_profile.json
    python .aaai/bid_tech/cli.py plan-from-ref --profile .aaai/ref_profile.json --project 新项目
"""

from __future__ import annotations

import json
import re
import zipfile
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple
from xml.etree import ElementTree as ET

from .docx_inspect import NS, _para_text, inspect_docx
from .planner import BidPlan, ChapterSpec, build_default_plan

ROUTE_CITY_RE = re.compile(
    r"(深圳|北京|上海|广州|喀什|乌鲁木齐|杭州|南京|西安|成都|重庆|武汉|厦门|珠海|东莞|惠州|"
    r"莲花山|大鹏|前海|故宫|天安门|国家博物馆|改革开放|展览馆|龙城|高巨)"
)

STAFFING_RATIO_RE = re.compile(r"1\s*:\s*\d{1,3}")

HEADER_ROLE_RULES: Tuple[Tuple[str, Tuple[str, ...]], ...] = (
    ("schedule_day", ("天数", "日程", "线路", "酒店")),
    ("schedule_timeline", ("时间", "环节", "工作内容")),
    ("schedule_half_hour", ("时间", "活动", "地点", "负责")),
    ("process", ("工作事项", "具体做法", "责任岗位", "形成材料")),
    ("materials", ("物资", "数量", "责任人")),
    ("transport", ("车辆", "核载", "研学线路")),
    ("emergency", ("突发", "应急", "善后")),
    ("risk", ("风险类别", "风险等级", "风险影响")),
    ("staffing", ("岗位", "配比", "人数")),
    ("insurance", ("险种", "保额", "理赔")),
    ("archive", ("归档", "形成", "时点")),
    ("contacts", ("公安", "医院", "地点")),
    ("outcomes", ("成果", "实施方式", "巩固")),
    ("overview", ("天数", "主要活动", "场馆")),
)


@dataclass
class TableArchetype:
    rows: int
    cols: int
    headers: List[str]
    role: str
    signature: str


@dataclass
class ReferenceProfile:
    source_path: str
    char_count: int = 0
    table_count: int = 0
    half_hour_slot_count: int = 0
    time_mention_count: int = 0
    schedule_style: str = "unknown"
    day_schedule_row_count: int = 0
    outline_sections: List[str] = field(default_factory=list)
    table_archetypes: List[TableArchetype] = field(default_factory=list)
    recommended_table_roles: List[str] = field(default_factory=list)
    route_hints: List[str] = field(default_factory=list)
    staffing_hints: List[str] = field(default_factory=list)
    keyword_hints: List[str] = field(default_factory=list)
    suggested_gates: Dict[str, Any] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        data = asdict(self)
        data["table_archetypes"] = [asdict(t) for t in self.table_archetypes]
        return data

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "ReferenceProfile":
        archetypes = [TableArchetype(**item) for item in data.get("table_archetypes") or []]
        return cls(
            source_path=data.get("source_path") or "",
            char_count=int(data.get("char_count") or 0),
            table_count=int(data.get("table_count") or 0),
            half_hour_slot_count=int(data.get("half_hour_slot_count") or 0),
            time_mention_count=int(data.get("time_mention_count") or 0),
            schedule_style=data.get("schedule_style") or "unknown",
            day_schedule_row_count=int(data.get("day_schedule_row_count") or 0),
            outline_sections=list(data.get("outline_sections") or []),
            table_archetypes=archetypes,
            recommended_table_roles=list(data.get("recommended_table_roles") or []),
            route_hints=list(data.get("route_hints") or []),
            staffing_hints=list(data.get("staffing_hints") or []),
            keyword_hints=list(data.get("keyword_hints") or []),
            suggested_gates=dict(data.get("suggested_gates") or {}),
            notes=list(data.get("notes") or []),
        )


def _normalize_header(cells: Sequence[str]) -> str:
    return "|".join(c.strip() for c in cells if c.strip())


def _infer_role(headers: Sequence[str]) -> str:
    joined = " ".join(headers)
    for role, keywords in HEADER_ROLE_RULES:
        if len(keywords) >= 2 and all(kw in joined for kw in keywords[:2]):
            return role
        if len(keywords) == 1 and keywords[0] in joined:
            return role
    if len(headers) >= 3:
        return "generic_table"
    return "title_block"


def _extract_table_headers(docx: Path) -> List[Tuple[int, int, List[str]]]:
    with zipfile.ZipFile(docx) as archive:
        root = ET.fromstring(archive.read("word/document.xml"))
    result: List[Tuple[int, int, List[str]]] = []
    for table in root.findall(".//w:tbl", NS):
        rows = table.findall("./w:tr", NS)
        if not rows:
            continue
        first = rows[0]
        cells: List[str] = []
        for tc in first.findall("./w:tc", NS):
            parts = [
                _para_text(p).strip()
                for p in tc.findall(".//w:p", NS)
                if _para_text(p).strip()
            ]
            cells.append(" ".join(parts))
        if not any(cells):
            continue
        result.append((len(rows), len(cells), cells))
    return result


def _detect_schedule_style(
    *,
    half_hour: int,
    day_rows: int,
    has_day_table: bool,
    has_timeline_table: bool,
) -> str:
    if half_hour >= 60:
        return "half_hour_dense"
    if has_day_table and day_rows >= 10:
        if half_hour >= 15 or has_timeline_table:
            return "mixed"
        return "day_block"
    if half_hour >= 20:
        return "mixed"
    if half_hour <= 12 and day_rows >= 5:
        return "day_block"
    return "sparse"


def count_day_schedule_rows(docx_path: str | Path) -> int:
    """统计「天数×时段×线路」类日程表的有效行数（不含表头）。"""
    target = Path(docx_path)
    best = 0
    for rows, cols, headers in _extract_table_headers(target):
        if cols <= 1:
            continue
        if _infer_role(headers) == "schedule_day":
            best = max(best, rows - 1)
    return best


def extract_reference_profile(docx_path: str | Path) -> ReferenceProfile:
    target = Path(docx_path)
    structure = inspect_docx(target)
    text = structure.full_text
    table_headers = _extract_table_headers(target)

    archetypes: List[TableArchetype] = []
    role_counts: Dict[str, int] = {}
    day_schedule_rows = 0
    has_day_table = False
    has_timeline_table = False

    for rows, cols, headers in table_headers:
        if cols <= 1:
            continue
        role = _infer_role(headers)
        sig = _normalize_header(headers)
        archetypes.append(
            TableArchetype(rows=rows, cols=cols, headers=list(headers), role=role, signature=sig)
        )
        role_counts[role] = role_counts.get(role, 0) + 1
        if role == "schedule_day":
            has_day_table = True
            day_schedule_rows = max(day_schedule_rows, rows - 1)
        if role in ("schedule_timeline", "schedule_half_hour"):
            has_timeline_table = True

    schedule_style = _detect_schedule_style(
        half_hour=structure.half_hour_row_count,
        day_rows=day_schedule_rows,
        has_day_table=has_day_table,
        has_timeline_table=has_timeline_table,
    )

    route_hits: Dict[str, int] = {}
    for match in ROUTE_CITY_RE.finditer(text):
        node = match.group(1)
        route_hits[node] = route_hits.get(node, 0) + 1
    route_hints = [k for k, _ in sorted(route_hits.items(), key=lambda x: -x[1])[:8]]

    staffing_hints = sorted(set(STAFFING_RATIO_RE.findall(text)))

    keyword_candidates = [
        "责任人",
        "形成材料",
        "流程",
        "备用",
        "投保",
        "理赔",
        "导师",
        "教官",
        "半小时",
        "100元",
        "元/人/天",
    ]
    keyword_hints = [k for k in keyword_candidates if k in text]

    recommended_roles = [
        role for role, _ in sorted(role_counts.items(), key=lambda x: -x[1]) if role != "generic_table"
    ]

    gates: Dict[str, Any] = {
        "min_real_tables": max(8, structure.table_count - 3),
        "schedule_gate_mode": schedule_style,
    }
    if schedule_style == "day_block":
        gates["min_day_schedule_rows"] = max(8, day_schedule_rows - 2)
        gates["min_half_hour_rows"] = max(6, structure.half_hour_row_count)
    elif schedule_style == "half_hour_dense":
        gates["min_half_hour_rows"] = max(40, structure.half_hour_row_count - 5)
    else:
        gates["min_half_hour_rows"] = max(6, structure.half_hour_row_count)
        if day_schedule_rows:
            gates["min_day_schedule_rows"] = max(6, day_schedule_rows - 2)

    if route_hints:
        gates["route_must_include"] = route_hints[:4]
    if keyword_hints:
        gates["suggested_keywords"] = keyword_hints
    if staffing_hints:
        gates["staffing_ratios"] = staffing_hints

    notes: List[str] = []
    if schedule_style == "day_block":
        notes.append("参考稿以「天数×时段×线路」日程表为主，勿强行写 80 条半小时槽。")
    if schedule_style == "mixed":
        notes.append("参考稿同时含按天行程表与日流程表，生成时两层都要。")
    if "100元" not in text and "元/人/天" not in text:
        notes.append("参考稿未写具体日保费数字；若招标硬性要求再补，勿默认 100 元。")
    if "形成材料" not in text:
        notes.append("参考稿未强调「形成材料」字段；流程表可用「工作内容/备注」列。")

    outline = [
        item
        for item in structure.outline
        if re.search(r"方案|管理|保障|出行|安全|档案|承诺|投保|活动", item)
    ][:40]

    return ReferenceProfile(
        source_path=str(target),
        char_count=structure.char_count,
        table_count=structure.table_count,
        half_hour_slot_count=structure.half_hour_row_count,
        time_mention_count=structure.time_mention_count,
        schedule_style=schedule_style,
        day_schedule_row_count=day_schedule_rows,
        outline_sections=outline,
        table_archetypes=archetypes,
        recommended_table_roles=recommended_roles,
        route_hints=route_hints,
        staffing_hints=staffing_hints,
        keyword_hints=keyword_hints,
        suggested_gates=gates,
        notes=notes,
    )


def save_profile(profile: ReferenceProfile, path: str | Path) -> Path:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(profile.to_dict(), ensure_ascii=False, indent=2), encoding="utf-8")
    return target


def load_profile(path: str | Path) -> ReferenceProfile:
    return ReferenceProfile.from_dict(json.loads(Path(path).read_text(encoding="utf-8")))


def apply_profile_to_plan(
    plan: BidPlan,
    profile: ReferenceProfile,
    *,
    mode: str = "merge",
) -> BidPlan:
    gates = profile.suggested_gates
    if mode not in ("merge", "replace_gates"):
        raise ValueError(f"unknown mode: {mode}")

    plan.min_real_tables = min(
        plan.min_real_tables, int(gates.get("min_real_tables") or plan.min_real_tables)
    )

    schedule_mode = gates.get("schedule_gate_mode") or profile.schedule_style
    if schedule_mode in ("day_block", "mixed", "sparse", "half_hour_dense"):
        plan.schedule_gate_mode = schedule_mode
    if gates.get("min_half_hour_rows") is not None:
        plan.min_half_hour_rows = min(plan.min_half_hour_rows, int(gates["min_half_hour_rows"]))
    if gates.get("min_day_schedule_rows") is not None:
        plan.min_day_schedule_rows = int(gates["min_day_schedule_rows"])

    if profile.route_hints and mode == "merge":
        plan.route_must_include = tuple(profile.route_hints[: min(6, len(profile.route_hints))])

    if profile.keyword_hints:
        kept = [k for k in plan.required_keywords if k in profile.keyword_hints]
        extras = [
            k
            for k in profile.keyword_hints
            if k in ("责任人", "流程", "备用", "投保", "导师", "教官")
        ]
        plan.required_keywords = tuple(dict.fromkeys([*kept, *extras]))

    plan.reference_profile_path = profile.source_path
    plan.reference_notes = list(profile.notes)
    return plan


def build_plan_from_reference(
    profile: ReferenceProfile,
    *,
    project_name: str = "技术标-综合评分技术部分",
    tender_chapters: Optional[Sequence[ChapterSpec]] = None,
) -> BidPlan:
    plan = build_default_plan(project_name)
    if tender_chapters:
        plan.chapters = list(tender_chapters)
    return apply_profile_to_plan(plan, profile, mode="merge")


def summarize_for_prompt(profile: ReferenceProfile, *, max_archetypes: int = 12) -> str:
    lines = [
        f"参考文件: {profile.source_path}",
        f"体量: {profile.char_count} 字；真表 {profile.table_count} 张",
        (
            f"日程风格: {profile.schedule_style}（半小时槽 {profile.half_hour_slot_count}；"
            f"按天表约 {profile.day_schedule_row_count} 行）"
        ),
        f"推荐表类型: {', '.join(profile.recommended_table_roles) or '—'}",
    ]
    if profile.route_hints:
        lines.append(f"路线节点: {', '.join(profile.route_hints)}")
    if profile.staffing_hints:
        lines.append(f"人员配比: {', '.join(profile.staffing_hints)}")
    if profile.keyword_hints:
        lines.append(f"参考关键词: {', '.join(profile.keyword_hints)}")
    if profile.notes:
        lines.append("注意:")
        for note in profile.notes:
            lines.append(f"- {note}")
    lines.append("表头范式（仿结构不抄内容）:")
    shown = 0
    for arch in profile.table_archetypes:
        if arch.role == "title_block":
            continue
        lines.append(f"- [{arch.role}] {arch.rows}x{arch.cols}: {' | '.join(arch.headers)}")
        shown += 1
        if shown >= max_archetypes:
            break
    if profile.outline_sections:
        lines.append("章节线索（前若干）:")
        for item in profile.outline_sections[:20]:
            lines.append(f"- {item}")
    return "\n".join(lines)
