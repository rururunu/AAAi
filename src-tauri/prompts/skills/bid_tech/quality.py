# -*- coding: utf-8 -*-
"""技术标内容质量门禁（反灌水 / 日程抄袭 / 空泛地点）。

与「数量门禁」互补：表够多、字够长仍可能用重复模板/灌水过关。
本模块只产出 ``GateIssue`` 列表与指标，由 ``gate.evaluate_gate`` 汇总。
"""

from __future__ import annotations

import re
from collections import Counter
from typing import Dict, List, Tuple

from .docx_inspect import HALF_HOUR_RE, DocxStructure

# (code, message, severity) — 避免与 gate 循环导入
QualityHit = Tuple[str, str, str]

# --- 灌水 / 模板抄袭启发式 ---
PADDING_PHRASES: Tuple[str, ...] = (
    "扩展内容",
    "组织管理扩展内容",
    "成果转化详述",
    "服务承诺详述",
    "食宿交通细节",
    "详述详述",
    "内容内容内容",
)

GENERIC_ACTIVITY_MARKERS: Tuple[str, ...] = (
    "主题探究活动A",
    "主题探究活动B",
    "互动体验环节",
    "互动实践课程",
    "成果总结展示",
    "行程导入讲解",
    "下午行程动员",
    "晨间集合动员",
    "返程车上互动",
)

GENERIC_PLACE_MARKERS: Tuple[str, ...] = (
    "参观点",
    "实践基地",
    "宿舍/酒店",
    "场馆入口",
    "大厅/广场",
)

DAY_HEADER_RE = re.compile(
    r"(?:^|\n)\s*(?:—+\s*)?第\s*(\d+)\s*天[^\n]{0,40}",
)
ACTIVITY_LINE_RE = re.compile(
    r"(?P<slot>\d{1,2}:\d{2}\s*[-—–~～至到]\s*\d{1,2}:\d{2})\s*\n(?P<act>[^\n]{1,80})"
)


def _jaccard(a: set[str], b: set[str]) -> float:
    if not a and not b:
        return 1.0
    if not a or not b:
        return 0.0
    return len(a & b) / len(a | b)


def extract_day_activity_sets(text: str) -> Dict[str, set[str]]:
    """按「第N天」切分，收集该日半小时槽后的活动文案集合。"""
    headers = list(DAY_HEADER_RE.finditer(text))
    if not headers:
        return {}
    days: Dict[str, set[str]] = {}
    for idx, match in enumerate(headers):
        day_no = match.group(1)
        start = match.end()
        end = headers[idx + 1].start() if idx + 1 < len(headers) else len(text)
        chunk = text[start:end]
        acts = {
            act.strip()
            for _slot, act in ACTIVITY_LINE_RE.findall(chunk)
            if act.strip()
        }
        # 无换行紧邻时退化为：半小时行后同一段落内的短词不够可靠，至少收活动标记
        if not acts:
            acts = {
                line.strip()
                for line in chunk.splitlines()
                if line.strip()
                and not HALF_HOUR_RE.fullmatch(line.strip())
                and 2 <= len(line.strip()) <= 40
                and not line.strip().startswith("第")
            }
        key = f"day-{day_no}"
        # 合并同号天（若文档写了两次第N天）
        days.setdefault(key, set()).update(acts)
    return days


def check_padding(text: str) -> Tuple[List[QualityHit], Dict[str, float | int]]:
    """检测灌水短语与超高重复短句。"""
    issues: List[QualityHit] = []
    metrics: Dict[str, float | int] = {}

    for phrase in PADDING_PHRASES:
        count = text.count(phrase)
        metrics[f"padding:{phrase}"] = count
        if count >= 3:
            issues.append(
                (
                    "padding_phrase",
                    (
                        f"检测到灌水短语「{phrase}」出现 {count} 次；"
                        "禁止用重复废话凑字数，应删掉并补真实表格/流程。"
                    ),
                    "error",
                )
            )

    # 同一段落级短句（8–40字）出现次数过高
    lines = [ln.strip() for ln in text.splitlines() if 8 <= len(ln.strip()) <= 40]
    counts = Counter(lines)
    worst_line, worst_count = ("", 0)
    if counts:
        worst_line, worst_count = counts.most_common(1)[0]
    metrics["max_repeated_short_line"] = worst_count
    activity_like = bool(
        worst_line
        and re.search(
            r"交流|参观|公园|馆|酒店|基地|学校|典礼|足球|创新|展览|动员|分享|研学",
            worst_line,
        )
    )
    if worst_count >= 6 and worst_line and not activity_like:
        issues.append(
            (
                "repeated_boilerplate",
                (
                    f"短句「{worst_line[:24]}…」重复 {worst_count} 次；"
                    "疑似复制灌水，门禁失败。"
                ),
                "error",
            )
        )

    # 连续同一子串（长度≥6）在正文中连环出现
    run_hits = 0
    for m in re.finditer(r"(.{6,24})\1{4,}", text):
        run_hits += 1
        if run_hits == 1:
            sample = m.group(1)
            issues.append(
                (
                    "concatenated_padding",
                    f"发现连续拼接灌水片段「{sample}」；删除后用实质内容替换。",
                    "error",
                )
            )
    metrics["concatenated_padding_hits"] = run_hits
    return issues, metrics


def check_schedule_similarity(
    text: str,
    *,
    max_pair_similarity: float = 0.72,
) -> Tuple[List[QualityHit], Dict[str, float | int]]:
    """不同日日程活动集合相似度过高 → 模板复制。"""
    issues: List[QualityHit] = []
    days = extract_day_activity_sets(text)
    metrics: Dict[str, float | int] = {"schedule_day_count": len(days)}
    if len(days) < 2:
        return issues, metrics

    keys = sorted(days.keys(), key=lambda k: int(k.split("-")[1]))
    high_pairs: List[str] = []
    sims: List[float] = []
    for i in range(len(keys) - 1):
        a, b = keys[i], keys[i + 1]
        sim = _jaccard(days[a], days[b])
        sims.append(sim)
        metrics[f"sim:{a}:{b}"] = round(sim, 3)
        if sim >= max_pair_similarity and len(days[a]) >= 8 and len(days[b]) >= 8:
            high_pairs.append(f"{a}/{b}={sim:.2f}")

    if sims:
        metrics["schedule_mean_adj_similarity"] = round(sum(sims) / len(sims), 3)
    if high_pairs:
        issues.append(
            (
                "schedule_template_clone",
                (
                    "多日半小时日程高度雷同（"
                    + "; ".join(high_pairs[:6])
                    + "）；禁止同一模板改标题，须按日填写真实场馆与活动。"
                ),
                "error",
            )
        )
    return issues, metrics


def check_generic_schedule_language(
    text: str,
    *,
    max_generic_activity_ratio: float = 0.35,
    max_generic_place_hits: int = 24,
) -> Tuple[List[QualityHit], Dict[str, float | int]]:
    """空泛活动/地点占比过高。"""
    issues: List[QualityHit] = []
    half_slots = HALF_HOUR_RE.findall(text)
    slot_count = len(half_slots)
    act_hits = sum(text.count(marker) for marker in GENERIC_ACTIVITY_MARKERS)
    place_hits = sum(text.count(marker) for marker in GENERIC_PLACE_MARKERS)
    ratio = (act_hits / slot_count) if slot_count else 0.0
    metrics: Dict[str, float | int] = {
        "generic_activity_hits": act_hits,
        "generic_place_hits": place_hits,
        "generic_activity_per_slot": round(ratio, 3),
    }
    if slot_count >= 40 and ratio >= max_generic_activity_ratio:
        issues.append(
            (
                "generic_schedule_activities",
                (
                    f"空泛活动用语命中 {act_hits}（相对半小时槽 {slot_count}，"
                    f"比值 {ratio:.2f} ≥ {max_generic_activity_ratio}）；"
                    "请写具体场馆/展项/任务，禁止「主题探究活动A/参观点」占位。"
                ),
                "error",
            )
        )
    if place_hits >= max_generic_place_hits:
        issues.append(
            (
                "generic_schedule_places",
                (
                    f"空泛地点「参观点/实践基地/宿舍/酒店」等合计 {place_hits} 次；"
                    "地点列须写真实场馆或交通节点。"
                ),
                "error",
            )
        )
    return issues, metrics


def check_insurance_response(text: str) -> Tuple[List[QualityHit], Dict[str, float | int]]:
    """研学投保硬响应：须出现人均日保费或明确保额口径。"""
    issues: List[QualityHit] = []
    has_daily = bool(re.search(r"100\s*元", text)) or "每人每天" in text or "元/人/天" in text
    has_insurance = ("投保" in text) or ("保险" in text)
    metrics: Dict[str, float | int] = {
        "has_100_yuan_insurance_signal": int(has_daily),
        "has_insurance_word": int(has_insurance),
    }
    if has_insurance and not has_daily:
        issues.append(
            (
                "insurance_daily_premium_missing",
                (
                    "投保章节未见「100元」或「元/人/天」等招标常见日保费响应；"
                    "请在保险核实表或流程中写明人均每日保费与累计口径。"
                ),
                "warn",  # 不同标书口径不一，默认警告；计划可升级
            )
        )
    return issues, metrics


def evaluate_quality_gates(
    structure: DocxStructure,
) -> Tuple[List[QualityHit], Dict[str, float | int]]:
    """对已解析结构跑完全部质量门禁。"""
    text = structure.full_text
    all_issues: List[QualityHit] = []
    metrics: Dict[str, float | int] = {}

    for checker in (
        check_padding,
        check_schedule_similarity,
        check_generic_schedule_language,
        check_insurance_response,
    ):
        issues, part = checker(text)
        all_issues.extend(issues)
        metrics.update(part)

    return all_issues, metrics
