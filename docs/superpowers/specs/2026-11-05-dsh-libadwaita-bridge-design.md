# dsh-libadwaita 设计：原生 GUI 与 dsh bridge

日期：2026-11-05
状态：待批准

## 意图与为什么

`dsh-libadwaita` 是 [deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness)（`dsh`）的下游 Linux 桌面客户端。agent 内核、插件体系、模型适配、会话持久化、工具执行与版本节奏全部来自上游；本项目不重写、不 fork、不复制其中的业务逻辑。

上游已经有 `web`（HTTP + React 浏览器客户端）、`desktop`（Electron 壳包同一套 Web UI）、`sdk`（stdio JSON-RPC）、`acp`（标准 ACP v1）。在 Linux 桌面上，现有形态的问题不是能力不足，而是体验不贴系统：Electron 与浏览器都不跟随 GTK 主题与系统设置，文件选择、默认程序打开、凭据存储、通知与后台任务、单实例激活、GNOME 快捷键与无障碍都要绕一层。

**最终目标：让 dsh 在 Linux 桌面上更易用。** 判断一个改动是否值得做，看它是否减少 Linux 用户完成任务的步骤、是否符合 GNOME HIG、是否降低常驻资源占用。功能与 Web UI 对齐不是目标。

"专门针对 Linux 优化"具体指：原生 GTK4/libadwaita 窗口与控件；跟随系统深浅色、强调色、缩放、减少动效；Wayland 原生；Portal 文件选择与目录选择；用系统默认程序打开结果文件与终端；Secret Service 保存 API 凭据；桌面通知与关闭窗口后任务继续；D-Bus 单实例激活；GTK 原生快捷键与无障碍树；不常驻浏览器引擎。

## Goals

- 一个 Rust 写的 GTK4/libadwaita 应用，作为 dsh 的本地客户端，可创建/恢复/关闭会话、提交输入、看到增量输出与工具活动、审批操作、回答问题、切换模型与工作目录。
- 一个随应用分发的 Cordis 插件 bundle，作为 dsh 侧的薄 bridge，投影上游服务与事件到一条我们自有的稳定本地协议。
- 跟随上游：升级 dsh 时，改动集中在 bridge 这一层与版本声明，不改 agent 逻辑，不逆向浏览器客户端。
- 完整的 Linux 桌面集成（上一节列举的各项）。
- 界面以 dsh Web UI 为蓝本：布局、导航结构、交互流程与展示卡片照抄上游，用 GTK4/libadwaita 控件重新实现；不搬 React/TS 代码，不嵌 WebView。
- 可重复、隔离的集成测试，覆盖握手、流式、审批、取消、断线与进程崩溃。

## Non-goals

- 不做 macOS / Windows 支持。
- 不 fork 上游源码，不把上游包内联进本仓库。
- 不把上游的 React/TypeScript 客户端实现搬进来；只移植布局、交互与信息架构，用 libadwaita 重写。
- 不实现远程访问、多用户、账号体系、自动更新服务端。
- 不做 Flatpak 沙箱打包，第一阶段也不做 AppImage / deb。
- 不引入 WebView 渲染消息内容。

## Anti-goals（技术上做成了也算失败）

- 为了"兼容性"保留第二条与 Web Host 私有 RPC 通信的代码路径。
- 把上游的 `SessionEvent` 词汇翻译成 GUI 自建的另一套事件模型，导致上游升级时两处都要改。
- bridge 往 stdout 写任何非协议字节。
- 把 API key 写进明文配置、环境变量日志或仓库。
- 为对齐 Web UI 而在 GTK 里塞一个浏览器引擎。
- 用 `flatpak-spawn --host` 或 `filesystem=host` 绕过沙箱后仍自称沙箱应用。

## 约束

**上游约束**

- dsh 处于 developer preview，明确会有破坏性变更。bridge 包必须通过 `engines.dsh` 声明兼容范围，应用锁 `@deepseek-ai/dsh` 的确切版本；每次上游升级是一次显式的兼容验证与版本 bump。
- dsh 应用只能通过 `dsh --profile <name>` 启动；自定义能力来自 profile、bundle 与有序 patch 文件，不是新的可执行文件或进程内挂载。profile 以 `@deepseek-ai/dsh-base` 为第一层 bundle（`dsh.profile.bundles`），bridge bundle 排在其后；不从 `sdk`/`web` 模板派生，因此不挂载 `sdk-jsonrpc-server` 与 `sdk-app-startup`，dsh 进程的 stdin/stdout 不被 SDK server 占用，也没有 stdin EOF 触发退出的行为。
- bundle 在 `package.json` 用 `dsh.bundle.patch` 指向 patch 文件；patch 通过 `insert` 增加 bridge 服务行。插件名可为绝对路径、file URL 或包标识符。
- bridge 是 socket 上唯一的 JSON-RPC 服务端，承载 GUI 需要的全部方法；诊断与日志只走 stderr，stdout 不写任何字节。base 默认启用 config-only HMR、默认挂载写明文 `.credentials.yaml` 的 `dsh-credentials-local`，两者都在 profile patch 里显式处理（见"profile 组合"）。
- 上游的 single-exe SDK runtime（`deepseek-harness-runtime-bin`）是**封闭插件集**，尚不支持注入外部插件并共享 exe 内的 Cordis 实例。因此本项目走 npm 包形态的 `@deepseek-ai/dsh` + Node，不能依赖该 exe。
- 架构文档把 "Add UI or editor integration → drive `ctx.agents` and render from `session/event`" 列为正式扩展点。`agent/assistant-stream` 的 start/chunk/end 是 process-local live 事件，目前 Web 的 Session-follow adapter 是它唯一的 remote consumer；bridge 成为第二个 remote consumer 需要在 spike 中先验证。

**运行时约束**

- Node.js >= 24.9（本机 24.22.1），pnpm 11（本机 11.22.0）。
- `DSH_HOME` 固定为 `$XDG_DATA_HOME/dsh-libadwaita`（应用私有）。每个 GUI 实例独占其中的 profile 目录与一个 Unix domain socket；socket 位于 `$XDG_RUNTIME_DIR/dsh-libadwaita/`，权限 0600，不监听 TCP 端口。
- API 凭据由 GUI 经 libsecret / Secret Service 存取，通过 bridge RPC 注入 dsh 进程内存；不写明文文件、不进 `git`、命令参数、日志或交付内容。dsh 与 agent 工具进程以同一 OS 用户运行，内存注入只能避免磁盘明文，不能对 agent 隔离 secret。
- GUI 与 dsh 是两个进程：GUI 拥有子进程生命周期（启动、握手、健康探测、退出、崩溃后的会话恢复），dsh 拥有全部 agent 状态。

**本机基线**：Arch Linux、GTK 4.24.1、libadwaita 1.10.0、Wayland（niri）、Rust nightly 1.101、`gtk4-rs 0.10.3` / `libadwaita 0.8.1` / `tokio` / `zbus` / `ashpd` 可用、Blueprint 0.22.2、Meson 1.12.1、GNOME Flatpak runtime 45/48 已安装、libsecret 头文件在位。

## Approaches

**淘汰：libadwaita 薄壳 + WebKitGTK 承载 Web UI。** 启动 `dsh --profile web`，GTK 只做外壳，内容区是网页。最快、功能最全、最贴上游；但界面本身不是 libAdwaita，Linux 优化只能落在外围（窗口、通知、Portal），消息区仍是 HTML，且仍常驻一个浏览器引擎。与"做 Linux GUI 上的优化"和"libAdwaita GUI 版本"都冲突。

**淘汰：原生 UI 直连 Web Host 私有 RPC。** `web` profile 是唯一带 token 级增量流式的形态，但 `/api` 是 Typert 构建时生成的内部契约，认证是启动 URL 里的 process token 换签名 cookie，加上 boot injection 与 `/api/remote.mux` WebSocket。第三方客户端要复刻整个 React 客户端的协议面，且该契约没有兼容性承诺。追上游成本最高。

**候选 A：原生 UI + 标准 ACP v1。** `AdwApplicationWindow → stdio NDJSON → dsh --profile acp`，Rust 直接用官方 `agent-client-protocol` crate。契约最稳、起步最快，cancel / session list / resume / close / model 选择现成。代价：上游已有明确决策把 ACP 降级为 automation-only，live chunks、plans、todos、titles、diffs、terminal views、human elicitation 全部不上线，`session/request_permission` 是机器策略通道而非人工审批 UI。能做出"能聊起来的客户端"，做不出"更好用的 dsh"。

**候选 B：原生 UI + SDK JSON-RPC。** `AdwApplicationWindow → stdio NDJSON → dsh --profile sdk`，Rust 用 `serde_json` + `tokio` 实现 `dsh-sdk-protocol` 方法表。零 dsh 侧代码、跟随公开包、能渲染完整 `SessionEvent`、原生审批与提问。代价：协议明确没有 cancel、没有 session list/close、没有标题、没有设置/模型目录/插件管理面，也没有 token 级流式；停止一个运行中的 turn 只能杀进程再 `session/resume`。缺的恰好是 GUI 的基本盘。

**候选 C（已选择）：原生 UI + 自有薄 bridge。** `AdwApplicationWindow → Unix socket NDJSON → dsh --profile libadwaita`，profile patch 挂我们写的 Cordis 插件 bundle。复用 SDK 已定义的 NDJSON framing 与 `SessionEvent` / 审批 / 提问词汇，只补上游没给的方法：增量流、会话列表、取消、设置与模型目录、插件管理。能力上限不受别人的协议限制；"追上游"变成维护一层薄适配，而不是逆向浏览器客户端。代价：多一份 TypeScript 插件与构建链，多一个需要锁版本的兼容面。

用户选择 **C**，理由是目标为 Linux 上更好用，而"好用"的关键（停止、流式、原生审批、会话侧栏、设置）恰好被 A、B 切掉；同时 C 是与上游耦合最可控的形式。

## 选定方案的构成

### 进程边界

```mermaid
flowchart LR
  A[GTK4 / libadwaita UI<br/>Rust 单二进制] -->|NDJSON JSON-RPC<br/>Unix socket 0600| B[dsh --profile libadwaita<br/>Node 24.9+]
  B --> C[@dsh-libadwaita/bridge<br/>Cordis 插件 bundle]
  C --> D[ctx.agents / ctx.sessions<br/>ctx.settings / ctx.llm<br/>agent/assistant-stream]
  A -->|libsecret / Portal<br/>D-Bus / 通知| E[Linux 桌面服务]
```

GUI 与 dsh 之间只有一个字节流连接、一个 JSON-RPC 服务端（bridge）；不存在第二条到 Web Host 的通路，也不存在并行的 stdio SDK 通道。socket 路径由 GUI 生成并通过 `--patch` 或环境变量传给 bridge，不写死、不对外暴露。

### dsh 侧：bridge bundle

bridge 是 workspace 内的 TypeScript 包（`bridge/`）：既是一个 Cordis 插件，也是一个可随应用分发的 bundle。

**profile 组合**（由 GUI 首启时在用户空间生成，见"首启引导与凭据"）：

- profile 的 `package.json`：`dsh.profile.bundles` 为 `["@deepseek-ai/dsh-base", <bridge bundle>]`。
- bridge bundle 的 `package.json` 声明 `dsh.bundle.patch` 与 `engines.dsh`；其 patch 文件用 `insert` 挂载 bridge 服务行。
- profile 的 `cordis.patch.yml` 做两件事：禁用 base 的 `hmr` 行（config-only HMR，行 id 以 `dsh --profile libadwaita --dump-config` 实际输出为准），让运行中的连接不会观察到被热替换的服务；把本地文件凭据 provider 换成 bridge 的内存凭据 provider（`ctx.credentials` seam 的 Service Provider 替换），阻止默认 provider 落明文 `.credentials.yaml`。
- base 已带 `session-title`（含 LLM 生成标题）、`settings`、`plugin-manager`、`user-questions`、`config-editor`，在 profile 启动时默认启用；GUI 直接复用，不自建标题与设置存储。
- 不叠加 Web server、前端资产或 SDK stdio server。

bridge 是一个表示层适配器，职责边界：

- 从 `ctx.agents`、`ctx.sessions`、`ctx.settings`、`ctx.llm` 及对应事件订阅状态，不实现任何 agent 行为、工具或模型适配。
- 协议帧复用 `@deepseek-ai/dsh-sdk-protocol` 的 `JsonRpcLineTransport`（它接受调用方自有的字节流）与 JSON-RPC 2.0 语义；传输是 Unix socket，不使用 stdio。
- 词汇沿用 SDK 已定义的 `SessionEvent`、`approval/request`、`user-question/ask`，不自造第二套事件模型。
- 方法表由 bridge 自有，覆盖 Web UI 界面元素所需的全部能力：会话创建/列表/恢复/关闭、输入提交、事件与状态通知、审批与提问应答、增量流通知、取消当前 turn、模型与设置查询/修改、插件列表与启停、凭据注入，以及计划、todo、diff、终端等展示数据。方法名走独立命名空间；`session/wait` 这类只服务自动化的方法不实现，除非 GUI 确实需要。
- 所有日志与诊断写 stderr；stdout 保持干净。
- 插件通过 `ctx.effect()` 注册/注销订阅与 socket 监听，卸载时可逆。

**跟随上游的契约面**只有三处：bundle 的 `engines.dsh` 范围、patch 中插入行的 id/name、bridge 里引用的 `ctx.*` 服务与事件名。上游升级时的验证顺序是先 `--dump-config` 看组合是否仍成立，再跑集成测试，最后 bump 版本声明。

### GUI 侧：领域模型

按业务概念划分对象，状态与不变量封装在对象内，协议细节不扩散到 UI：

- `HarnessSupervisor` — 拥有 dsh 子进程、profile 目录与 socket 路径；负责首启引导（见"首启引导与凭据"）、启动、握手、健康探测、退出、崩溃检测与恢复。不变量：同一时刻最多一个活着的 dsh 进程，profile 与 socket 不被第二个实例同时使用。
- `BridgeConnection` — JSON-RPC 请求关联、超时、通知分发、断线状态；把线协议错误映射为有类型的错误。
- `Session` — 一个会话的标识、标题、工作目录、模型选择、运行状态与已提交事件；`ConversationTranscript` 把 `SessionEvent` 折叠成可渲染块（文本、工具调用、审批、思考），并持有未提交的 live stream 临时块。不变量：临时块在 end frame 到达时被提交块替换或丢弃，不留残影。
- `ApprovalBroker` / `UserQuestionBroker` — 把 server→client 请求路由到对应会话的原生对话框；超时、取消、连接丢失一律 fail-closed（拒绝）。
- `SettingsStore` / `CredentialStore` / `WorkspaceChooser` — 分别封装设置读写、Secret Service 凭据存取与向 bridge 的注入、Portal 目录选择。
- UI 层：`AdwApplication` → 主窗口（会话侧栏 + 消息区 + 输入框）、首启向导（API key）、设置页、审批/提问对话框，布局、导航与交互照抄上游 Web UI（`apps/web` 与 `packages/client/ui-*`）。消息内容用 GTK4 原生控件渲染，代码块走 GtkSourceView 语法高亮，不嵌入 WebView。

依赖方向：UI → 领域对象 → `BridgeConnection` → 字节流。`gtk4-rs` / `libadwaita` 类型不越过领域层。

实施时先列一张"Web UI 组件 → libadwaita 控件"的映射表作为移植清单，再按清单逐个实现；这张表是移植对照，不是重新设计。

### 首启引导与凭据

系统级安装（PKGBUILD）只写 `/usr`，不碰用户 `$HOME`。profile 的创建与升级全部由 GUI 在首次运行时于用户空间完成：

- 探测 profile 目录是否存在、版本是否与应用携带的 bridge bundle 一致；缺失或过期时生成/更新 `package.json` 与 `cordis.patch.yml`，bridge bundle 以绝对 file URL 指向系统安装位置，不在用户目录复制或 `npm install` 一份。
- 生成 socket 路径并以 0600 创建；随后启动 `dsh --profile libadwaita` 并完成握手。
- 启动不依赖凭据已就绪：没有 API key 时 dsh 仍可启动并握手，模型调用在未配置时返回"未配置"，GUI 据此进入首启向导，而不是阻塞进程启动。
- 凭据流程：首启向导/设置页 → Secret Service 写入 → bridge RPC 注入 dsh 内存。轮换与新增 provider 走同一条路，不重启 dsh 进程。

### 分发

第一阶段 Arch PKGBUILD，显式依赖 `nodejs>=24.9`、`gtk4`、`libadwaita`、`libsecret`，并锁定 `@deepseek-ai/dsh` 与 bridge bundle 的版本。应用不捆绑 Node；后续若 Debian/Ubuntu 的 Node 版本跟不上，再评估自带 Node 运行时。Metadata 提供应用 id、图标、D-Bus 激活名与桌面文件。

不采用 Flatpak：coding agent 必须读写任意工作区并执行 shell，沙箱需要 `filesystem=host` 或 `flatpak-spawn --host`，两者都使沙箱失去意义。

### 验证策略

步骤 → 验证方式：

1. **一次性 spike（不进产品代码）** — 空 GTK 窗口 + 最小 bridge，订阅 `agent/assistant-stream` 并把 chunk 打到窗口/终端。验证：base-backed 自定义 profile 能启动且 stdin/stdout 不被绑架、Unix socket 上的 `JsonRpcLineTransport` 可用、bridge 能作为第二个 remote consumer 收到 live 事件、GTK 能实时渲染。spike 结论写回本文档，不保留 spike 代码。
2. **集成测试（CI 入口）** — 从应用公共入口启动真实 dsh 子进程与隔离的 `DSH_HOME` / `XDG_RUNTIME_DIR`，用受控的 mock model endpoint（不连生产、不用生产凭据）。覆盖：握手与版本不匹配、创建/列出/恢复/关闭会话、提交输入并收到增量与提交块、审批 allow/reject/cancel、用户提问回答与取消、运行中 turn 的取消、进程崩溃后恢复会话、socket 权限与 stdout 纯净性。每个用例自建并清理临时目录。
3. **单元测试** — transcript 折叠与 live 块替换、JSON-RPC 编解码与错误映射、broker 的 fail-closed 边界、状态机非法转移。
4. **桌面手动验证** — Wayland 下窗口与深浅色/缩放跟随、Portal 目录选择、系统默认程序打开结果、通知、单实例 D-Bus 激活、凭据存进 Secret Service、关闭窗口后任务继续。
5. **上游升级验证** — 每次 bump `engines.dsh`：`--dump-config` + 上述集成测试全绿才算兼容。

## 用户已决定的部分

- 走自有薄 bridge（候选 C）。
- 不重写 agent 内核；跟随上游版本节奏；项目定位就是 dsh 的 libAdwaita GUI 版本。
- 目标是 Linux 桌面易用性；Web UI 是界面蓝本，不是要搬运的技术栈。
- 界面布局、导航结构与交互流程照抄 dsh Web UI，只用 libadwaita 重写实现，不另起一套界面设计。
- `DSH_HOME` 用应用私有目录 `$XDG_DATA_HOME/dsh-libadwaita`：GUI 与终端里的 `dsh` 完全隔离，不读取也不修改 `~/.dsh`；终端会话历史与设置在 GUI 里不可见。

## 我做出的决定（供复核）

- UI 语言与工具链：Rust + `gtk4-rs 0.10` / `libadwaita 0.8` + Blueprint + Meson（或纯 cargo，实施计划定）；`tokio` 做异步桥、`ashpd` 走 Portal、`zbus` 做 D-Bus 单实例、`libsecret` 存取凭据。理由：单二进制、GTK 生态最完整、本机 crate 已齐、异步与长连接模型贴合。
- bridge 传输用 Unix domain socket 而非 stdio，复用 `dsh-sdk-protocol` 的 framing 与类型，并由 bridge 承载全部方法（单通道，不并行 SDK stdio server）。理由：一条连接一套状态机；stdio 若同时给 SDK server 用会争帧，且 `sdk-app` 把 stdin EOF 绑定为进程退出。
- 自定义 profile 以 `dsh-base` 为第一层，不从 `sdk` 模板派生；在 profile patch 里禁用 HMR、替换掉默认的明文文件凭据 provider。理由：base 已提供模型/工具/持久化/审批，派生自 `sdk` 只会多带一个必须禁用的 stdio server；禁用 HMR 保证单连接生命周期内服务不被热替换。
- 分发先做 Arch PKGBUILD、依赖系统 Node、不做 Flatpak 沙箱。理由：本机即 Arch + Wayland；沙箱与 agent 的执行需求冲突。
- 消息内容用 GTK 原生控件渲染，不引入 WebView。理由：Anti-goal。
- 第一个里程碑是一次性 spike 验证流式链路，而非先搭完整 UI。理由：整条链路上唯一未验证的假设是 live 事件与 socket 传输。
- 凭据走"Secret Service + bridge RPC 内存注入"，不用启动环境变量。理由：满足不落明文，且支持热轮换，避免改 key 就要冷重启 dsh 并恢复所有会话。
- 首启 profile 由 GUI 在用户空间生成，bridge bundle 用绝对 file URL 引用系统安装位置。理由：系统级打包不能写 `$HOME`，也不需要 npm 安装与符号链接。

## 留给 builder 决定的部分

- bridge 方法表的确切名称、payload 与错误码；以"复用 SDK 词汇 + 独立命名空间"为原则自行设计，写进 bridge 的协议文档。
- 主窗口的视觉细节、图标与文案；布局、导航结构与交互流程不重新设计，以 dsh Web UI 为蓝本照抄。
- Markdown 解析与 GTK 控件映射的具体库选择（建议 `pulldown-cmark` + GtkSourceView）。
- Blueprint 与纯 Rust 构建 widget 的比例、构建系统是 Meson 还是纯 cargo。
- 应用 id、包名、i18n 方案、CI 平台与发布流程。
- 上游 dsh 版本与 `engines.dsh` 范围的初始值（以写计划时上游最新发布版为准）。
- 展示卡片（计划、todo、diff、终端视图等）随 Web UI 一并移植，用 libadwaita 控件重写；具体移植顺序在实施计划里按对 bridge 能力的依赖排。
