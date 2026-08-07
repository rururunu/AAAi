# -*- coding: utf-8 -*-
"""技术标（综合评分技术部分）表驱动生成与自检工具包。

本包随 ``generate_bid_tech`` skill 物化到工作区 ``.anya/bid_tech/``。
设计目标：
- 表格一等公民（禁止用段落冒充表格）
- 章节骨架先于正文
- 完成前硬门禁（数量 + 反灌水/日程抄袭等质量项）
- 联网结果只填格，不堆散文

模块分工：
- ``style``         版式与字体预设
- ``tables``        可复用表模板（日程/流程/配比/档案等）
- ``planner``       评分章节骨架与每章最低交付物
- ``docx_inspect``  docx 结构解析（大纲/表/样式）
- ``gate``          完成前自检门禁
- ``quality``       反灌水 / 日程相似度 / 空泛地点
- ``reference``     从优质参考 docx 提取表范式与自适应门禁
- ``align``         招标要求 ↔ 投标响应勾选对齐
"""

from . import align, gate, docx_inspect, planner, quality, reference, style, tables

__all__ = [
    "align",
    "gate",
    "docx_inspect",
    "planner",
    "quality",
    "reference",
    "style",
    "tables",
]

__version__ = "1.0.0"
