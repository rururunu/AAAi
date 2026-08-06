# -*- coding: utf-8 -*-
"""评分章节骨架（planner）：先目录后填空，避免前详后略。

默认骨架对齐常见「综合评分表-技术评分标准」十章结构。
真实项目应优先从招标文件抽取章节与分值，再用 ``override_chapters`` 覆盖。
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence


@dataclass(frozen=True)
class ChapterSpec:
    """单章最低交付规格。"""

    key: str
    title: str
    score: int
    # 评委扫读用的评分点短句
    score_points: Sequence[str]
    # 必须出现的真表类型（逻辑名，对应 tables.py）
    required_tables: Sequence[str]
    # 本章正文（含表内文字）最低汉字/字符数
    min_chars: int
    # 额外硬约束说明（给人看 / 写进门禁报告）
    notes: str = ""


@dataclass
class BidPlan:
    """整份技术标计划。"""

    project_name: str
    chapters: List[ChapterSpec] = field(default_factory=list)
    # 全篇门禁（可被 gate / 参考画像覆盖）
    min_real_tables: int = 15
    min_half_hour_rows: int = 80
    min_day_schedule_rows: int = 0
    # half_hour_dense | day_block | mixed | sparse
    schedule_gate_mode: str = "half_hour_dense"
    required_keywords: Sequence[str] = (
        "深圳",
        "北京",
        "责任人",
        "形成材料",
        "半小时",
    )
    route_must_include: Sequence[str] = ("深圳", "北京")
    reference_profile_path: str = ""
    reference_notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "project_name": self.project_name,
            "min_real_tables": self.min_real_tables,
            "min_half_hour_rows": self.min_half_hour_rows,
            "min_day_schedule_rows": self.min_day_schedule_rows,
            "schedule_gate_mode": self.schedule_gate_mode,
            "required_keywords": list(self.required_keywords),
            "route_must_include": list(self.route_must_include),
            "reference_profile_path": self.reference_profile_path,
            "reference_notes": list(self.reference_notes),
            "chapters": [asdict(c) for c in self.chapters],
        }

    def save(self, path: str | Path) -> Path:
        target = Path(path)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(
            json.dumps(self.to_dict(), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        return target


def default_tech_score_chapters() -> List[ChapterSpec]:
    """默认十章技术评分骨架（分值可被招标文件覆盖）。"""
    return [
        ChapterSpec(
            key="insurance",
            title="投保方案",
            score=3,
            score_points=("投保对象与实施", "保额设置", "与招标契合"),
            required_tables=("insurance_research", "process"),
            min_chars=800,
            notes="须含出单时效与理赔流程；禁止只写口号。",
        ),
        ChapterSpec(
            key="activity",
            title="活动方案",
            score=10,
            score_points=(
                "主题贴合认知",
                "学科结合",
                "互动实践",
                "校内外衔接",
                "适龄创新",
                "半小时日程",
            ),
            required_tables=("overview", "schedule", "backup"),
            min_chars=4000,
            notes="必须：行程总览表 + 半小时日程四列表（含负责人）+ 备用方案表。",
        ),
        ChapterSpec(
            key="organization",
            title="组织与管理方案",
            score=12,
            score_points=(
                "流程管理",
                "物资清单",
                "部门协调",
                "导师表现",
                "安全保障衔接",
                "特色服务",
            ),
            required_tables=("org", "staffing", "process"),
            min_chars=2500,
            notes="必须：架构表 + 人员配比表 + 阶段流程五列表（事项/岗位/形成材料）。",
        ),
        ChapterSpec(
            key="travel",
            title="出行方案",
            score=8,
            score_points=("大交通", "集散候乘", "延误应急"),
            required_tables=("transport_research", "process"),
            min_chars=700,
        ),
        ChapterSpec(
            key="board_lodge_transport",
            title="食宿交通方案",
            score=5,
            score_points=("住宿标准", "餐饮安全", "市内交通"),
            required_tables=("process",),
            min_chars=500,
        ),
        ChapterSpec(
            key="safety",
            title="安全保障方案",
            score=14,
            score_points=("责任体系", "场景应急", "五步闭环"),
            required_tables=("emergency", "process"),
            min_chars=1000,
            notes="必须：场景×响应动作×第一责任人表。",
        ),
        ChapterSpec(
            key="materials_promo_venue",
            title="活动物资、宣传与场馆配套方案",
            score=6,
            score_points=("物资清单", "宣传矩阵", "场馆票务讲解"),
            required_tables=("venue_research", "process"),
            min_chars=600,
        ),
        ChapterSpec(
            key="outcomes",
            title="研学成果转化方案",
            score=3,
            score_points=("成果形态", "展示路径", "归档模板"),
            required_tables=("archive",),
            min_chars=400,
        ),
        ChapterSpec(
            key="archives",
            title="档案管理",
            score=3,
            score_points=("分类归档", "形成时点", "验收配合"),
            required_tables=("archive", "acceptance"),
            min_chars=600,
            notes="必须：分类×内容×形成时点表 + 验收配合表。",
        ),
        ChapterSpec(
            key="commitment",
            title="服务承诺",
            score=3,
            score_points=("投保承诺", "违约责任", "书面承诺格式"),
            required_tables=("process",),
            min_chars=400,
        ),
    ]


def build_default_plan(project_name: str = "技术标-综合评分技术部分") -> BidPlan:
    return BidPlan(project_name=project_name, chapters=default_tech_score_chapters())


def load_plan(path: str | Path) -> BidPlan:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    chapters = [
        ChapterSpec(
            key=item["key"],
            title=item["title"],
            score=int(item["score"]),
            score_points=tuple(item.get("score_points") or ()),
            required_tables=tuple(item.get("required_tables") or ()),
            min_chars=int(item.get("min_chars") or 0),
            notes=item.get("notes") or "",
        )
        for item in data.get("chapters") or []
    ]
    return BidPlan(
        project_name=data.get("project_name") or "技术标",
        chapters=chapters,
        min_real_tables=int(data.get("min_real_tables") or 15),
        min_half_hour_rows=int(data.get("min_half_hour_rows") or 80),
        min_day_schedule_rows=int(data.get("min_day_schedule_rows") or 0),
        schedule_gate_mode=data.get("schedule_gate_mode") or "half_hour_dense",
        required_keywords=tuple(data.get("required_keywords") or ()),
        route_must_include=tuple(data.get("route_must_include") or ()),
        reference_profile_path=data.get("reference_profile_path") or "",
        reference_notes=list(data.get("reference_notes") or []),
    )


def override_chapters(
    plan: BidPlan,
    chapters: Sequence[ChapterSpec],
) -> BidPlan:
    """用招标文件抽出的真实章节替换默认骨架。"""
    plan.chapters = list(chapters)
    return plan


def outline_markdown(plan: BidPlan) -> str:
    """生成可写入任务列表的目录草稿。"""
    lines = [
        f"# {plan.project_name}",
        "",
        "## 章节目录（先填空后写正文）",
        "",
    ]
    for idx, chapter in enumerate(plan.chapters, start=1):
        lines.append(
            f"{idx}. **{chapter.title}（{chapter.score}分）** "
            f"— 最低字数 {chapter.min_chars}；必表：{', '.join(chapter.required_tables)}"
        )
        if chapter.notes:
            lines.append(f"   - 约束：{chapter.notes}")
        for point in chapter.score_points:
            lines.append(f"   - 评分点：{point}")
    lines.extend(
        [
            "",
            "## 全篇门禁",
            f"- 真表格 ≥ {plan.min_real_tables}",
            f"- 日程模式：{plan.schedule_gate_mode}",
            f"- 半小时日程行（HH:MM-HH:MM）≥ {plan.min_half_hour_rows}",
        ]
    )
    if plan.min_day_schedule_rows:
        lines.append(f"- 按天行程表有效行 ≥ {plan.min_day_schedule_rows}")
    lines.extend(
        [
            f"- 关键词：{', '.join(plan.required_keywords)}",
            f"- 路线节点：{', '.join(plan.route_must_include)}",
            "",
        ]
    )
    if plan.reference_notes:
        lines.append("## 参考稿提示")
        for note in plan.reference_notes:
            lines.append(f"- {note}")
        lines.append("")
    return "\n".join(lines)


def write_outline(plan: BidPlan, path: str | Path) -> Path:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(outline_markdown(plan), encoding="utf-8")
    return target
