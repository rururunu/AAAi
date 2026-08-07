# -*- coding: utf-8 -*-
"""命令行入口：结构解析 / 门禁 / 对齐。

物化后用法（在工作区根目录）::

    python .anya/bid_tech/cli.py inspect path/to/file.docx
    python .anya/bid_tech/cli.py gate path/to/file.docx --plan .anya/bid_plan.json
    python .anya/bid_tech/cli.py align path/to/file.docx --checklist .anya/bid_align.json
    python .anya/bid_tech/cli.py outline --out .anya/bid_plan_outline.md
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# 允许直接 python path/to/cli.py 运行
_PKG_PARENT = Path(__file__).resolve().parents[1]
if str(_PKG_PARENT) not in sys.path:
    sys.path.insert(0, str(_PKG_PARENT))

from . import align, gate, docx_inspect, planner, reference  # noqa: E402


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="技术标 bid_tech 工具")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_inspect = sub.add_parser("inspect", help="解析 docx 结构")
    p_inspect.add_argument("docx")
    p_inspect.add_argument("--json", action="store_true")

    p_gate = sub.add_parser("gate", help="完成前门禁")
    p_gate.add_argument("docx")
    p_gate.add_argument("--plan")
    p_gate.add_argument("--report")
    p_gate.add_argument(
        "--skip-quality",
        action="store_true",
        help="跳过反灌水/日程抄袭等质量门禁（仅调试）",
    )

    p_align = sub.add_parser("align", help="招标对齐勾选")
    p_align.add_argument("docx")
    p_align.add_argument("--checklist")
    p_align.add_argument("--write-default", help="写入默认清单到该路径")

    p_outline = sub.add_parser("outline", help="写出默认章节大纲")
    p_outline.add_argument("--project", default="技术标-综合评分技术部分")
    p_outline.add_argument("--out", default=".anya/bid_plan_outline.md")
    p_outline.add_argument("--plan-json", default=".anya/bid_plan.json")

    p_ref = sub.add_parser("reference", help="从参考 docx 提取可迁移画像")
    p_ref.add_argument("docx")
    p_ref.add_argument("--out", default=".anya/ref_profile.json")
    p_ref.add_argument("--print", action="store_true", help="同时打印摘要")

    p_pfr = sub.add_parser("plan-from-ref", help="用参考画像生成/校准 bid_plan.json")
    p_pfr.add_argument("--profile", default=".anya/ref_profile.json")
    p_pfr.add_argument("--project", default="技术标-综合评分技术部分")
    p_pfr.add_argument("--out", default=".anya/bid_plan.json")
    p_pfr.add_argument("--outline", default=".anya/bid_plan_outline.md")

    args = parser.parse_args(argv)

    if args.cmd == "inspect":
        structure = docx_inspect.inspect_docx(args.docx)
        if args.json:
            print(json.dumps(structure.to_dict(include_text=False), ensure_ascii=False, indent=2))
        else:
            print(docx_inspect.summarize_for_prompt(structure))
        return 0

    if args.cmd == "gate":
        plan = planner.load_plan(args.plan) if args.plan else planner.build_default_plan()
        report = gate.evaluate_gate(args.docx, plan, skip_quality=args.skip_quality)
        print(report.format_text())
        if args.report:
            gate.save_report(report, args.report)
        return 0 if report.passed else 2

    if args.cmd == "align":
        if args.write_default:
            align.save_checklist(align.default_checklist_for_study_tour(), args.write_default)
            print("wrote", args.write_default)
        items = (
            align.load_checklist(args.checklist)
            if args.checklist
            else align.default_checklist_for_study_tour()
        )
        report = align.check_alignment(
            args.docx,
            items,
            source=args.checklist or "default",
        )
        print(report.format_text())
        return 0 if not report.open_items else 2

    if args.cmd == "outline":
        plan = planner.build_default_plan(args.project)
        planner.write_outline(plan, args.out)
        plan.save(args.plan_json)
        print("outline:", args.out)
        print("plan:", args.plan_json)
        return 0

    if args.cmd == "reference":
        profile = reference.extract_reference_profile(args.docx)
        reference.save_profile(profile, args.out)
        print("profile:", args.out)
        if args.print:
            print(reference.summarize_for_prompt(profile))
        return 0

    if args.cmd == "plan-from-ref":
        profile = reference.load_profile(args.profile)
        plan = reference.build_plan_from_reference(profile, project_name=args.project)
        planner.write_outline(plan, args.outline)
        plan.save(args.out)
        print("plan:", args.out)
        print("outline:", args.outline)
        print(reference.summarize_for_prompt(profile))
        return 0

    return 1


if __name__ == "__main__":
    raise SystemExit(main())
