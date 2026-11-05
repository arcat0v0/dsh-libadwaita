# dsh-libadwaita 实施计划

日期：2026-11-05
依据：`docs/superpowers/specs/2026-11-05-dsh-libadwaita-bridge-design.md`（设计文档，已定稿）
说明：本环境未安装 `superpowers:writing-plans` 技能，本计划按设计文档与项目工程规范手写。

## 目标与边界

交付一个 Linux 桌面上的 dsh 原生客户端：Rust + GTK4/libadwaita GUI，经 Unix domain socket 上的 NDJSON JSON-RPC 连接 `dsh --profile libadwaita`，由自写的 Cordis bridge bundle 提供全部界面能力。界面布局、导航、交互与展示卡片照抄上游 Web UI，只用 libadwaita 重写。

不做：重写 agent 内核、fork 上游、搬 React/TS 客户端代码、嵌 WebView、macOS/Windows、Flatpak 沙箱、远程访问与多用户。

## 前置条件

- 本机已具备：Node 24.22.1、pnpm 11.22.0、Rust nightly 1.101、GTK 4.24.1、libadwaita 1.10.0、Blueprint 0.22.2、Meson 1.12.1、libsecret 头文件、Wayland。
- 上游 dsh 版本：以开工时 npm 上 `@deepseek-ai/dsh` 最新发布版为准，锁定确切版本并写入 bridge 的 `engines.dsh`。
- 需要补充确认：GtkSourceView 5 开发文件（用于代码块高亮），缺失则安装或在映射表中改用纯 GTK 控件。
- 仓库当前未初始化 git；阶段 0 第一步完成 `git init`。

## 仓库布局

```
app/        Rust crate：GUI、领域对象、bridge 客户端、集成测试
bridge/     TypeScript 包：Cordis 插件 + dsh bundle（dsh.bundle.patch、engines.dsh）
docs/       设计文档、本计划、bridge 协议、Web UI 映射表
packaging/  PKGBUILD、桌面文件、图标、D-Bus 服务文件
```

## 阶段 0：仓库与工具链骨架

任务：
1. `git init`，写 `.gitignore`（Rust target、node_modules、构建产物、本地凭据与临时目录）。
2. 建 `app/`（cargo）与 `bridge/`（pnpm 包）两个最小可构建骨架，接入上述目录布局。
3. 确定 bridge 构建方式（tsc 或 tsdown）与 Rust 构建方式（纯 cargo 或 Meson），只选一种并写进 README。
4. 写 README：项目定位、上游依赖、构建与测试入口。

验证：`cargo check`、`bridge` 的 typecheck/build 均通过；`git status` 干净，产物未入库。

## 阶段 1：一次性 spike（技术假设验证，代码不进产品）

目的：在写任何产品代码之前，验证整条链路上唯一未证的假设。

任务：
1. 手工写一个 base-backed profile：`package.json` 的 `dsh.profile.bundles` 为 `["@deepseek-ai/dsh-base"]`，`cordis.patch.yml` 禁用 `hmr` 行。
2. `dsh --profile libadwaita --dump-config` 确认组合成立，记录 `hmr`、本地凭据 provider 等真实行 id。
3. 最小 TS 插件：监听 Unix socket，用 `@deepseek-ai/dsh-sdk-protocol` 的 `JsonRpcLineTransport` 收发；订阅 `agent/assistant-stream`，把 start/chunk/end 原样转发；另订阅 `session/event` 作对照。
4. 最小 Rust/GTK 窗口：连接 socket，把收到的帧打到窗口。
5. 用 mock model endpoint（不连生产、不用真实 key）跑一个 turn。

验证（全部通过才进入阶段 2）：
- base-backed profile 能启动，stdin EOF 不会导致进程退出，stdout 无杂字节。
- bridge 能作为 `agent/assistant-stream` 的第二个 remote consumer 收到 start/chunk/end。
- socket 上的 NDJSON 帧在 Rust 侧可解析，GTK 能实时显示增量。
- `ctx.credentials` 的 provider 可被自写的内存实现替换（阶段 2 依赖此结论）。

产出：结论写回设计文档"验证策略"一节；spike 代码删除，不进 git 历史的产品路径。若任一假设不成立，停下回到设计文档修订，不在错误假设上继续。

## 阶段 2：bridge 协议与 dsh 进程生命周期（TDD）

先写协议文档 `docs/bridge-protocol.md`：方法表、通知、双向请求、错误码、协议版本协商。词汇沿用 SDK 的 `SessionEvent`、`approval/request`、`user-question/ask`；方法名走独立命名空间。

bridge 侧任务：
1. 连接与握手：协议版本、dsh 版本、profile 名；版本不匹配显式报错。
2. 会话：创建/列表/恢复/关闭、工作目录读写、模型选择读写。
3. 输入与输出：提交输入、`session/event` 转发、`agent/assistant-stream` 转发、状态通知。
4. 交互：审批与提问的双向请求，超时/取消/断线 fail-closed。
5. 控制：取消当前 turn。
6. 设置与插件：设置读写、模型目录、插件列表与启停。
7. 凭据：内存凭据 provider 替换 base 默认的文件 provider，提供注入/轮换/移除方法。
8. 全部日志走 stderr；订阅与 socket 监听经 `ctx.effect()` 可逆卸载。

Rust 侧任务：
1. `HarnessSupervisor`：首启生成/升级用户空间 profile（bridge bundle 用绝对 file URL 指向安装位置）、创建 0600 socket、启动/握手/健康探测/退出/崩溃检测与恢复。不变量：同一时刻最多一个 dsh 进程。
2. `BridgeConnection`：NDJSON 编解码、请求关联、超时、通知分发、断线状态、有类型的错误。
3. `DSH_HOME` 固定 `$XDG_DATA_HOME/dsh-libadwaita`，不读写 `~/.dsh`。

集成测试（先于实现，从公共入口驱动真实 dsh 子进程）：
- 隔离：每用例独立临时 `DSH_HOME`、`XDG_RUNTIME_DIR`、工作区，结束清理；不连生产、不用生产凭据。
- 覆盖：握手与版本不匹配、会话创建/列表/恢复/关闭、提交输入并收到增量与提交块、审批 allow/reject/cancel、提问回答与取消、运行中 turn 取消、进程崩溃后恢复会话、socket 权限、stdout 纯净性、profile 首启生成与升级。
- 统一测试入口并接入 CI（阶段 7）。

验证：上述集成测试全绿；`--dump-config` 的组合与协议文档一致。

## 阶段 3：Web UI 映射表与 GUI 骨架

任务：
1. 读上游 `apps/web` 与 `packages/client/ui-*`，产出 `docs/web-ui-map.md`：页面/导航结构、每个区域的组件与交互、展示卡片清单、对应的 libadwaita 控件、所需 bridge 能力。这是移植清单，不是重新设计。
2. `AdwApplication`、主窗口、会话侧栏、消息区、composer、设置页骨架，布局与导航对照映射表。
3. D-Bus 单实例与激活；应用 id、图标、桌面文件占位。

验证：手动运行，窗口结构与上游 Web UI 一致；映射表覆盖阶段 4–6 要用的全部界面元素；领域对象不依赖 GTK 类型（`cargo test`）。

## 阶段 4：会话与消息链路

任务：
1. `Session` 与 `ConversationTranscript`：`SessionEvent` 折叠为渲染块，live 临时块在 end frame 被提交块替换或丢弃，不留残影。
2. 消息渲染：Markdown 用 GTK 原生控件，代码块 GtkSourceView 高亮，不嵌 WebView。
3. 审批/提问原生对话框，路由到对应会话，fail-closed。
4. 停止按钮、会话列表/恢复/关闭、Portal 工作目录选择、模型切换。

验证：集成测试覆盖 transcript 与交互边界；手动对照 Web UI 的同一 prompts，布局与交互一致；运行中停止后无残留临时块。

## 阶段 5：首启向导、凭据与设置

任务：
1. 首启向导：无凭据时 dsh 仍可启动并握手，GUI 进入向导而非阻塞。
2. Secret Service 存取 + bridge 注入；设置页轮换/新增 provider 走同一路径，不重启 dsh。
3. 设置页与插件管理页，对照映射表。

验证：无 key 启动进向导；保存后立即可用；轮换不重启、会话不中断；`$DSH_HOME` 下无明文凭据文件；日志与仓库中无 secret。

## 阶段 6：展示卡片移植

按 `docs/web-ui-map.md` 逐个移植计划、todo、diff、终端等卡片，用 libadwaita 控件重写；每张卡片先在映射表登记所需 bridge 能力，缺则在 bridge 补齐并加集成测试。

验证：每张卡片与 Web UI 同场景对照；相应 bridge 能力有集成测试。

## 阶段 7：桌面集成、分发与 CI

任务：
1. 通知、关闭窗口后任务继续、系统默认程序打开结果与终端、深浅色/强调色/缩放/减少动效跟随、快捷键与无障碍。
2. `packaging/`：PKGBUILD（依赖 `nodejs>=24.9`、`gtk4`、`libadwaita`、`libsecret`，锁定 dsh 与 bridge 版本）、桌面文件、图标、D-Bus 服务文件。
3. CI：构建 + 阶段 2 集成测试入口 + 协议/映射表文档检查。

验证：PKGBUILD 在干净环境构建、安装、启动；桌面手动验证清单全过；CI 绿。

## 阶段 8：上游跟踪

每次 bump dsh：`--dump-config` 比对行 id 与服务名 → 跑全部集成测试 → 必要时改 bridge 适配 → 更新 `engines.dsh` 与锁版本 → 更新文档。

验证：一次真实上游升级走通该流程并记录结果。

## 风险与对策

- `agent/assistant-stream` 是否允许第二个 remote consumer：阶段 1 先验证，不通过则回到设计文档改方案，不硬写。
- base 行 id 与配置随上游变化：不硬编码猜测，以 `--dump-config` 实际输出为准。
- 凭据 provider 替换的 seam 细节：阶段 1 验证，若不可替换则改为"启动环境注入 + 冷重启"并回写设计文档。
- Web UI 组件数量大：用映射表控制范围，按阶段 4→6 推进，不在阶段 3 一次性做完全部卡片。
- GtkSourceView 可用性：前置条件确认，缺失则用纯 GTK 渲染并在映射表注明。

## 交付物清单

- `app/`：可运行的 libadwaita 客户端与领域层、集成测试。
- `bridge/`：Cordis 插件与 bundle、协议文档。
- `docs/`：设计文档、本计划、`bridge-protocol.md`、`web-ui-map.md`。
- `packaging/`：Arch PKGBUILD 与桌面集成文件。
- CI 入口与上游升级流程记录。
