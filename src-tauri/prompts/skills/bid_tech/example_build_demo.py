# -*- coding: utf-8 -*-
"""最小可运行示例：演示表模板 + 门禁（内容为占位，真实标书须写满日程行）。

物化到工作区后可复制为 scripts/build_tech_bid.py 再扩写。
"""

from __future__ import annotations

import sys
from pathlib import Path

# cli / 示例与 package 同级时：父目录加入 path，从而 `import bid_tech`
_HERE = Path(__file__).resolve().parent
_AAAI = _HERE.parent
if str(_AAAI) not in sys.path:
    sys.path.insert(0, str(_AAAI))

from bid_tech import align, gate, planner, style, tables  # noqa: E402


def build_demo(out: Path) -> Path:
    plan = planner.build_default_plan("演示项目-综合评分技术部分")
    out.parent.mkdir(parents=True, exist_ok=True)

    doc = style.configure_document(header_text=plan.project_name)
    style.add_heading_cn(doc, plan.project_name, level=1)
    style.add_body(doc, "本文件为工具包演示稿，正式投标须按招标文件补全半小时日程与各章必表。", first_line_indent=False)

    style.add_heading_cn(doc, "一、投保方案（3分）", level=2)
    style.add_score_lead(doc, "投保方案", ["投保对象", "保额", "理赔流程"])
    tables.add_insurance_research_table(
        doc,
        [("人身意外", "按招标", "出发前5个工作日", "待核实", "全员覆盖")],
    )
    tables.add_process_table(
        doc,
        [("1", "名单采集", "交叉核对身份证", "综合协调组", "投保名单表")],
    )

    style.add_heading_cn(doc, "二、活动方案（10分）", level=2)
    style.add_score_lead(doc, "活动方案", ["半小时日程", "互动实践"])
    tables.add_overview_table(doc, [("第1天", "深圳", "开班", "—"), ("第5天", "北京", "升旗与场馆", "天安门")])
    # 演示仅两行；门禁会失败——正式稿必须写满 ≥80 条 HH:MM-HH:MM
    tables.add_schedule_table(
        doc,
        [
            ("06:00-06:30", "集合", "喀什", "教官"),
            ("06:30-07:00", "出发", "机场", "交通岗"),
        ],
    )
    tables.add_backup_table(doc, [("暴雨", "户外", "室内备用场馆")])

    style.add_heading_cn(doc, "三、组织与管理方案（12分）", level=2)
    style.add_score_lead(doc, "组织与管理方案", ["架构", "配比", "形成材料"])
    tables.add_org_table(doc, [("项目指挥部", "项目经理等", "总调度")])
    tables.add_staffing_table(doc, [("研学导师", "1:25", "指导师证", "≥4")])
    tables.add_process_table(
        doc,
        [("1", "需求调研", "对接学校", "项目经理", "调研记录")],
    )

    style.add_heading_cn(doc, "六、安全保障方案（14分）", level=2)
    tables.add_emergency_table(doc, [("走失", "5分钟内定点清点并报警", "教官")])

    style.add_heading_cn(doc, "九、档案管理（3分）", level=2)
    tables.add_archive_table(doc, [("过程", "签到表", "每日")])
    tables.add_acceptance_table(doc, [("验收", "提交全套档案", "纸质+电子")])

    style.add_heading_cn(doc, "十、服务承诺（3分）", level=2)
    tables.add_process_table(doc, [("1", "投保承诺", "先投保后出行", "项目经理", "保单")])

    doc.save(str(out))
    return out


def main() -> int:
    root = Path.cwd()
    out = root / "docs" / "bid_tech_demo.docx"
    build_demo(out)
    plan = planner.build_default_plan("演示项目-综合评分技术部分")
    checklist = align.default_checklist_for_study_tour()
    align_report = align.check_alignment(out, checklist)
    report = gate.evaluate_gate(out, plan, align_open_items=align_report.open_items)
    print(report.format_text())
    print(align_report.format_text())
    print("demo saved:", out)
    # 演示稿预期门禁失败（日程行不足），返回 2 便于调用方理解门禁行为
    return 0 if report.passed else 2


if __name__ == "__main__":
    raise SystemExit(main())
