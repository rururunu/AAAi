# -*- coding: utf-8 -*-
"""完成前硬门禁：不达标不得宣称完成。

检查项：
1. 真表格数量
2. 半小时日程行（HH:MM-HH:MM）数量
3. 各评分章最低字符数
4. 必备关键词 / 路线节点
5. （可选）对齐勾选表未完成项
6. 质量门禁：反灌水、日程模板抄袭、空泛地点、投保日保费信号

用法::
    python .aaai/bid_tech/cli.py gate path/to.docx --plan .aaai/bid_plan.json
或在生成脚本末尾调用 ``assert_gate_passed``。
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence

from .docx_inspect import inspect_docx
from .planner import BidPlan, ChapterSpec, build_default_plan, load_plan
from .quality import evaluate_quality_gates
from .reference import count_day_schedule_rows


@dataclass
class GateIssue:
    code: str
    message: str
    severity: str = "error"  # error | warn


@dataclass
class GateReport:
    passed: bool
    docx: str
    metrics: Dict[str, Any] = field(default_factory=dict)
    issues: List[GateIssue] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "passed": self.passed,
            "docx": self.docx,
            "metrics": self.metrics,
            "issues": [asdict(item) for item in self.issues],
        }

    def format_text(self) -> str:
        lines = [
            f"门禁结果: {'通过' if self.passed else '未通过'}",
            f"文档: {self.docx}",
            "指标:",
        ]
        for key, value in self.metrics.items():
            lines.append(f"  - {key}: {value}")
        if self.issues:
            lines.append("问题:")
            for issue in self.issues:
                lines.append(f"  - [{issue.severity}] {issue.code}: {issue.message}")
        else:
            lines.append("问题: 无")
        if not self.passed:
            lines.append(
                "处置: 继续补真实表格/按日差异化日程；禁止灌水凑字、禁止只润色文案后宣称完成。"
            )
        return "\n".join(lines)


def _chapter_min_chars(plan: BidPlan) -> Dict[str, int]:
    mapping: Dict[str, int] = {}
    for chapter in plan.chapters:
        mapping[chapter.title] = chapter.min_chars
        if "活动物资" in chapter.title:
            mapping["活动物资"] = chapter.min_chars
        if "研学成果" in chapter.title:
            mapping["研学成果"] = chapter.min_chars
    return mapping


def evaluate_gate(
    docx_path: str | Path,
    plan: Optional[BidPlan] = None,
    *,
    align_open_items: Optional[Sequence[str]] = None,
    skip_quality: bool = False,
) -> GateReport:
    plan = plan or build_default_plan()
    structure = inspect_docx(docx_path)
    issues: List[GateIssue] = []

    metrics: Dict[str, Any] = {
        "table_count": structure.table_count,
        "half_hour_row_count": structure.half_hour_row_count,
        "char_count": structure.char_count,
        "section_chars": structure.section_chars,
        "schedule_gate_mode": plan.schedule_gate_mode,
    }

    day_rows = count_day_schedule_rows(docx_path)
    metrics["day_schedule_row_count"] = day_rows

    if structure.table_count < plan.min_real_tables:
        issues.append(
            GateIssue(
                code="tables_too_few",
                message=(
                    f"真表格 {structure.table_count} < 下限 {plan.min_real_tables}；"
                    "禁止用段落冒充表格，必须 Document.add_table。"
                ),
            )
        )

    mode = plan.schedule_gate_mode or "half_hour_dense"
    min_day = plan.min_day_schedule_rows or 0

    if mode == "day_block":
        if min_day and day_rows < min_day:
            issues.append(
                GateIssue(
                    code="schedule_day_too_few",
                    message=(
                        f"按天行程表有效行 {day_rows} < 下限 {min_day}；"
                        "须补充「天数/时段/日程/线路/酒店」类真表。"
                    ),
                )
            )
        elif structure.half_hour_row_count < plan.min_half_hour_rows:
            metrics["half_hour_optional"] = True
    elif mode == "mixed":
        half_ok = structure.half_hour_row_count >= plan.min_half_hour_rows
        day_ok = (not min_day) or day_rows >= min_day
        if not half_ok and not day_ok:
            issues.append(
                GateIssue(
                    code="schedule_too_coarse",
                    message=(
                        f"混合日程模式：半小时行 {structure.half_hour_row_count} "
                        f"< {plan.min_half_hour_rows}，且按天表 {day_rows} < {min_day}；"
                        "至少满足一种日程表深度。"
                    ),
                )
            )
        elif not half_ok:
            metrics["half_hour_below_dense_target"] = structure.half_hour_row_count
        elif min_day and not day_ok:
            issues.append(
                GateIssue(
                    code="schedule_day_too_few",
                    message=f"按天行程表有效行 {day_rows} < 下限 {min_day}。",
                )
            )
    elif structure.half_hour_row_count < plan.min_half_hour_rows:
        issues.append(
            GateIssue(
                code="schedule_too_coarse",
                message=(
                    f"半小时日程行 {structure.half_hour_row_count} < 下限 "
                    f"{plan.min_half_hour_rows}；须补充 HH:MM-HH:MM 且含负责人列。"
                ),
            )
        )

    text = structure.full_text
    for keyword in plan.required_keywords:
        if keyword and keyword not in text:
            issues.append(
                GateIssue(
                    code="missing_keyword",
                    message=f"缺少必备关键词「{keyword}」。",
                )
            )

    for node in plan.route_must_include:
        if node and node not in text:
            issues.append(
                GateIssue(
                    code="route_node_missing",
                    message=f"路线/场馆节点缺失「{node}」；须先以招标+参考为准核对再写日程。",
                )
            )

    mins = _chapter_min_chars(plan)
    for section_name, min_chars in mins.items():
        actual = structure.section_chars.get(section_name)
        if actual is None:
            for key, value in structure.section_chars.items():
                if section_name in key or key in section_name:
                    actual = value
                    break
        if actual is None:
            issues.append(
                GateIssue(
                    code="section_not_found",
                    message=f"未定位到章节「{section_name}」标题，无法核验字数。",
                    severity="warn",
                )
            )
            continue
        if actual < min_chars:
            issues.append(
                GateIssue(
                    code="section_too_short",
                    message=(
                        f"章节「{section_name}」约 {actual} 字 < 下限 {min_chars}；"
                        "应补表格与流程字段，而非堆砌形容词或重复灌水。"
                    ),
                )
            )

    if align_open_items:
        for item in align_open_items:
            issues.append(
                GateIssue(
                    code="align_open",
                    message=f"招标对齐未闭合：{item}",
                )
            )

    if not skip_quality:
        quality_hits, quality_metrics = evaluate_quality_gates(structure)
        metrics.update(quality_metrics)
        for code, message, severity in quality_hits:
            issues.append(GateIssue(code=code, message=message, severity=severity))

    blocking = [item for item in issues if item.severity == "error"]
    return GateReport(
        passed=not blocking,
        docx=str(docx_path),
        metrics=metrics,
        issues=issues,
    )


def assert_gate_passed(
    docx_path: str | Path,
    plan: Optional[BidPlan] = None,
    *,
    align_open_items: Optional[Sequence[str]] = None,
    skip_quality: bool = False,
) -> GateReport:
    """门禁失败时抛出 RuntimeError，供生成脚本末尾调用。"""
    report = evaluate_gate(
        docx_path,
        plan,
        align_open_items=align_open_items,
        skip_quality=skip_quality,
    )
    if not report.passed:
        raise RuntimeError(report.format_text())
    return report


def save_report(report: GateReport, path: str | Path) -> Path:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(
        json.dumps(report.to_dict(), ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return target


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="技术标 docx 完成前门禁")
    parser.add_argument("docx", help="待检查的 .docx 路径")
    parser.add_argument("--plan", help="可选 planner JSON；缺省使用默认十章骨架")
    parser.add_argument("--report", help="可选：把门禁 JSON 报告写到该路径")
    parser.add_argument(
        "--skip-quality",
        action="store_true",
        help="仅跑数量/关键词门禁（调试用；正式交卷勿跳过）",
    )
    args = parser.parse_args(list(argv) if argv is not None else None)

    plan = load_plan(args.plan) if args.plan else build_default_plan()
    report = evaluate_gate(args.docx, plan, skip_quality=args.skip_quality)
    print(report.format_text())
    if args.report:
        save_report(report, args.report)
    return 0 if report.passed else 2


if __name__ == "__main__":
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    raise SystemExit(main())
