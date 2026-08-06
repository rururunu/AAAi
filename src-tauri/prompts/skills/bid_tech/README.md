# bid_tech 维护说明

本目录是内置 skill `generate_bid_tech` 的 Python 工具包源码。
运行时由 Rust `bid_tech_assets::materialize_bid_tech_lib` 写入工作区 `.aaai/bid_tech/`。
完成后由只读 skill `review_bid_tech` 评议合理性。

## 模块

| 文件 | 职责 |
|---|---|
| `style.py` | 页边距、页眉、中文字体、表格线框（默认全格线；可选三线表） |
| `tables.py` | 日程/流程/配比/档案等可复用真表模板 |
| `planner.py` | 评分章节骨架与每章最低交付物 |
| `docx_inspect.py` | OOXML 结构解析（勿命名为 `inspect.py`，会遮蔽标准库） |
| `gate.py` | 完成前硬门禁（数量 + 质量汇总） |
| `quality.py` | 反灌水 / 日程模板抄袭 / 空泛地点 / 投保日保费信号 |
| `reference.py` | 从优质参考 docx 提取表范式、日程风格、自适应门禁 |
| `align.py` | 招标要求 ↔ 投标响应勾选 |
| `cli.py` | 命令行入口（`gate` 支持 `--skip-quality` 仅调试） |
| `example_build_demo.py` | 演示稿（预期门禁失败，用于验证 gate） |

## 修改流程

1. 只改本目录源码（不要改用户工作区里物化出来的副本当源）。
2. `cargo test --lib bid_tech` 确认物化路径仍包含全部文件（含 `quality.py`）。
3. 本地：`python .aaai/bid_tech/cli.py …`（先跑一次 generate_bid_tech 或手动复制本目录）。

## 设计约束

- 表格必须 `Document.add_table`，禁止段落冒充。
- 数量门禁通过仍可能因质量门禁失败。
- 门禁失败或未跑 `review_bid_tech` 不得宣称完成。
- 联网结果只填单元格字段。
