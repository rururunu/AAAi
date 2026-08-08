# Anya 文档索引

<p align="center">
  <a href="./README.md">English</a>
  &nbsp;·&nbsp;
  <a href="./README.zh-CN.md">简体中文</a>
</p>

产品主页：[../README.zh-CN.md](../README.zh-CN.md)

这里是文档地图——产品介绍请看根目录 README；按任务打开对应指南即可。

| 文档                                             | 读者          | 适用场景                                             |
| ------------------------------------------------ | ------------- | ---------------------------------------------------- |
| [技术架构总览](./architecture-overview.zh-CN.md) | 贡献者        | 分层、进程拓扑、Ask/Agent/Plan、时间线、持久化、事件 |
| [发布与远程更新](./release.zh-CN.md)             | 发版负责人    | 签名、`latest.json`、GitHub Releases、CI             |
| 截图资源（`image/`）                             | 用户 / README | 根目录 README 引用的截图                             |

```mermaid
flowchart LR
  User[用户 / README] --> Arch[架构]
  User --> Rel[发布]
  Dev[贡献者] --> Arch
  Dev --> Rel
```

行为变更时，请更新上表对应文档，并保持中英文姊妹篇（`*.zh-CN.md`）结构同步。
