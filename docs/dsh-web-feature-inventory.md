# DSH Web 功能清单（权威基线与最终验收对照表）

- **上游仓库**：`https://github.com/deepseek-ai/deepseek-harness`（默认分支 `master`）
- **参考克隆提交 SHA**：`fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159`
- **npm `@deepseek-ai/dsh` 最新版本**：`0.2.2-rc.1`
- **用途**：
  1. 驱动阶段 3 `docs/web-ui-map.md` 的控件映射与阶段 4–6 GUI 实现顺序；
  2. 作为 `dsh-libadwaita` 对比上游 DSH Web 的**最终验收清单（Final Acceptance Checklist）**。
- **架构事实纠偏（源码核实）**：
  - `apps/web` 无传统前端路由（无 React Router / URL 页面路由表），仅有两个 HTML 构建入口：常规生产入口 `apps/web/index.html`（`apps/web/src/main.ts` → `new AppWebEntry(#root).run()`）与开发/构建预览入口 `apps/web/src/preview.ts`（`preview.html`，挂载 WebWorker VFS 预览源选择器，发布包中排除）。HTTP 路径 `/api`、`/connect`、`/plugins` 均为后端传输与静态/动态 bundle 端点而非前端页面路由。
  - Web 客户端通过 `packages/client/ui-layout` 的根级 `main` keyed slot 切换顶层主视图（默认 `'conversation'` 会话视图，或 `'plugins'`、`'schedules'` 等全局面板），通过 `shell.overlay` 承载模态设置面板与全局对话框，通过 `rightbar` 承载按会话隔离的右侧停靠栏。
  - 上游 Web **不存在** `session/close` 或删除持久会话的 RPC；“关闭当前会话”仅释放客户端 `mainView` 引用（`clearMain()`），会话生命周期管理采用**归档/取消归档**（`workspace/archiveSession`、`workspace/unarchiveSession`）。
  - 上游 Web **不存在**直接修改既有会话创建元数据 `cwd` 的通用 RPC；选择/切换工作目录通过 `directoryPicker/*` + `workspace/create` + `session/create` 在目标工作区打开空白会话或创建新会话。

---

## 1. 会话与导航

| 功能点 | 上游位置 | 依赖的后端能力 | 备注 |
|---|---|---|---|
| 常规 Web/Desktop 应用入口与两阶段插件启动 | `apps/web/index.html`；`apps/web/src/main.ts`；`packages/client/web/src/boot.ts`；`packages/client/web/src/boot-client.ts`；`packages/client/web/src/mount.ts` | Host 注入 `window.__DSH_BOOT__`、`window.__ModuleLoader__`；`internal/status`；`ctx.uiRenderer.mount(container)` | 校验必需插件（`modules`、`session-controller`、`ui-renderer`、`ui-layout`、`ui-session`、`ui-chat`、`ui-sidebar`），通过后从启动页切换至 React 根视图 |
| 无框架启动进度页与必需插件失败诊断页 | `packages/client/web/src/boot-page.ts`；`packages/client/web/src/loader-status.ts` | `internal/status`；`ctx.modules.entries` | 原生 DOM 显示 `HARNESS` 与旋转圆弧进度；必需插件失败时列出插件名与原因（缺服务/导入失败/激活报错），可选插件失败仅打控制台警告 |
| WebWorker 预览入口与数据源选择器 | `apps/web/src/preview.ts`；`packages/experimental/webworker-runtime/src/client/index.ts`；`packages/experimental/webworker-runtime/src/client/source-chooser.ts` | Worker 隧道 `fetch/openStream/loadBundle/resolveAsset`；`preview/vfs-image.tar.gz` | 仅构建预览使用，npm 发布排除 `dist/preview*`；GTK 客户端无需实现 Worker 预览，列此以穷尽 `apps/web` 入口 |
| 三栏主框架（AppFrame）与响应式抽屉/折叠栏 | `packages/client/ui-layout/src/client/AppFrame.tsx`；`packages/client/ui-layout/src/client/service.ts` | `ctx.layout`（`selectPanel`/`toggleSidebar`/`openRightbar`/`closeRightbar`/`panelInfo`/`layoutInfo`）；root slots `sidebar`/`main`/`rightbar`/`shell.bottom`/`shell.leading`/`shell.overlay` | 左栏默认 280px（264–420px），≥768px 收起保留 56px 控制轨，<768px 变为模态抽屉；保证中栏至少 400px，空间不足先压缩/关闭右栏；`Mod+B` 切换左栏 |
| 窗口标题同步與平台窗口拖拽区 | `packages/client/ui-layout/src/client/AppFrame.tsx`；`packages/client/web/src/base.css`；`packages/client/web/src/window-drag/regions.ts`；`packages/client/web/src/window-drag/recall.ts` | 当前选中会话 `title` 投影；构建变量 `DSH_CLIENT_TITLE` | 会话视图可见时窗口标题跟随当前会话标题，否则回退产品名；macOS/Windows Electron 维护 `data-window-drag` 与顶栏避让变量 |
| 左侧边栏骨架、折叠轨、新会话按钮与底部设置座 | `packages/client/ui-sidebar/src/client/SidebarRoot.tsx`；`packages/client/ui-sidebar/README.zh.md` | root slots `sidebar.brand.*`、`sidebar.workspaces`、`sidebar.panellist`、`sidebar.panellist.actions`、`sidebar.settings`、`sidebar.toggle.badge`；`ctx.shortcuts` | 容纳品牌头、New Session、全局面板入口（插件/自动化任务等）、工作区与会话树、底部账户与设置按钮 |
| 官方品牌标识与空白会话首页深海鲸鱼动画 | `packages/client/ui-brand-official/src/client/index.ts`；`packages/client/ui-brand-official/src/client/home-whale/` | slots `sidebar.brand.*`、`conversation.hero.frame`；`DSH_CLIENT_BUILD_PROFILE` | 填充侧栏品牌位与空白会话 Hero 区的 Canvas 鲸鱼动画 |
| 导航区域“更多”菜单、固定/取消固定入口与创建插件草稿 | `packages/client/ui-entry-customization/src/client/index.ts`；`packages/client/ui-entry-customization/src/client/EntryMenuContent.tsx` | `ctx.uiWorkspace.startSession`；`ctx.get('pluginNavigation')`；slots `sidebar.panellist.actions` | 在侧栏等区域提供“更多”菜单以固定/收起低频插件入口，或启动创建插件的预设草稿（仅填草稿不自动发送） |
| 会话目录状态适配、主视图绑定、未读完成提醒与待处理交互仲裁 | `packages/client/ui-session/src/client/index.ts` | `ctx.sessions`（`list`/`binding`/`retainInfo`）；`ctx.remote.$on('api-session/status')`；`ctx.uiSession.registerPendingInteraction`；`ctx.remoteAccess` | 将 `api/session-controller` 适配为 `useSessions`/`useSession`/`useProjection`/`useAccess`；后台会话运行结束转为停止时标记 `completionUnread`，按优先级仲裁同会话待处理交互 |
| 工作区与会话树（分组/平铺视图、水位线、置顶、手动拖拽排序） | `packages/client/ui-workspace/src/client/rows/WorkspaceBrowser.tsx`；`packages/client/ui-workspace/src/client/rows/Rows.tsx`；`packages/client/ui-workspace/src/client/tree.ts`；`packages/client/ui-workspace/src/client/pin-order.ts` | `ctx.workspaces`（`workspace-controller` follow 流：`upsert`/`remove`/`order`/`archived`/`pinned`）；`ctx.sessions.list` | 支持按工作区分组或平铺浏览、置顶会话、手动拖拽调整工作区与置顶顺序、显示运行中/待审批/未读完成状态点 |
| 新建会话与空白会话复用（含启动自动恢复） | `packages/client/ui-workspace/src/client/navigation.ts`；`packages/api/session-controller/src/commands.ts` | `ctx.uiWorkspace.startSession/openWorkspace` → `ctx.sessions.create` → RPC `session/create`；`workspaces.initializeDefault` | 同工作区同 preset 已有可复用空白会话时优先复用；启动时从 `localStorage`（`dsh.sessions.current`）恢复上次会话或最近工作区 |
| 打开并恢复历史会话 / 关闭当前会话视图 | `packages/client/ui-workspace/src/client/navigation.ts`；`packages/api/session-controller/src/client/sessions/session.ts` | `ctx.sessions.retain(id, { source: 'mainView' })` → RPC `session/follow({ activate: true, assistantStream: true })`；`clearMain()` | 列表冷读不激活 Agent，点击打开时才通过 `session/follow(activate: true)` 恢复 Agent；关闭视图仅释放 `mainView` 引用，不销毁持久日志 |
| 会话重命名（手动修改标题）与 LLM 自动生成标题 | `packages/client/ui-workspace/src/client/session-actions/RenameSession.tsx`；`packages/client/ui-workspace/src/client/rows/WorkspaceDialogs.tsx` | RPC `session/setTitle`（或 `ctx.sessions` 标题更新）；Host `session-title` 插件投影 `title` via `session/follow` / `api-session/status` | 首轮对话后由后端 `session-title` 自动生成标题并推送更新；用户可通过行右键菜单或对话框手动重命名 |
| 会话归档、取消归档、归档筛选器与运行中归档确认 | `packages/client/ui-workspace/src/client/session-actions/ArchiveSession.tsx`；`packages/client/ui-workspace/src/client/index.ts` | RPC `workspace/archiveSession`（支持 `{ stopActivity: true }`）、`workspace/unarchiveSession`；错误码 `workspace/session-active` | 归档保留持久日志；若会话正在运行，先弹确认框，确认后停止活动并清空排队消息再归档；侧栏支持隐藏/显示/仅看已归档 |
| 会话列表搜索（标题/工作区元数据 + 会话正文全文搜索） | `packages/client/ui-workspace/src/client/rows/WorkspaceBrowser.tsx`；`packages/api/session-controller/src/list.ts` | RPC `session/list`、`session/search`；Host `ctx.sessionQuery.searchSessions`（`session-query-sqlite`） | 支持在侧栏展开搜索框过滤会话标题与工作区，并在后端启用全文索引时检索历史消息内容 |
| 工作区目录选择（空白页 WorkspacePicker、切换工作区、重命名/移除工作区） | `packages/client/ui-workspace/src/client/WorkspacePicker.tsx`；`packages/client/ui-workspace/src/client/rows/WorkspaceDialogs.tsx` | RPC `workspace/create`、`workspace/rename`、`workspace/remove`；`ctx.uiWorkspace.openWorkspace` | 切换工作区会打开目标工作区的空白会话并迁移当前未提交草稿；未选定工作区时输入框保持禁用（inert） |
| 原生 OS 目录选择器集成 | `packages/client/ui-directory-picker-native/src/client/index.ts`；`packages/client/ui-directory-picker-native/src/client/flow.ts` | `ctx.remoteAccess.whenAccess('directoryPicker/pick')`；RPC `directoryPicker/pick`；Desktop `desktop.pick()` | 本机或 Desktop 宿主可用时直接调起系统原生目录选择对话框 |
| 应用内目录浏览器（Miller 分栏浏览与新建文件夹） | `packages/client/ui-directory-picker-browse/src/client/index.ts`；`packages/client/ui-directory-picker-browse/src/client/flow.ts` | RPC `directoryPicker/list`、`directoryPicker/createDirectory` | 远程 Web 或无原生选择器环境下的应用内分栏目录浏览与新建目录弹窗 |
| 右侧停靠栏（Right Sidebar）与多标签分栏/浮窗管理 | `packages/client/ui-sidebar-right/src/client/service.ts`；`packages/client/ui-dockkit/src/index.ts` | `ctx.sidebarRight`（`openResource`/`openTab`/`close`/`hide`/`focus`/`split`/`float`/`dock`/`toggleFullscreen`）；`ctx.sidebarRightTabs.register` | 基于 `ui-dockkit` 为每个会话维护独立的停靠分栏、浮动窗口、全屏切换与内存撤销栈，持久化至 `localStorage` |
| 右侧栏：交互式终端标签页（多 Shell 选择、重命名、断线恢复） | `packages/client/ui-sidebar-terminal/src/client/terminal.tsx`；`packages/client/ui-sidebar-terminal/src/client/index.ts` | `ctx.sidebarRightTabs.register(kind='terminal')`；`terminal-controller` RPC（创建/输入/resize/流式输出/关闭）；快捷键 `terminal.new` | 支持选择 Shell、重命名终端标签、跟随主题字号与配色；刷新页面可重连存活终端，关闭标签页终止对应进程 |
| 右侧栏：工作区文件树标签页与预览联动目录树 | `packages/client/ui-sidebar-files/src/client/FileTree.tsx`；`packages/client/ui-sidebar-files/src/client/workspace-tree.ts` | `ctx.sidebarRightTabs.register(kind='files')`；RPC `workspaceFiles.list`；快捷键 `workspace.files` | 浏览当前会话工作目录树、刷新、在文档预览旁开关同分栏目录导航并定位当前打开文件 |
| 右侧栏：多格式文档预览标签页（Markdown/代码/图片/PDF/Office/HTML/CSV/Excel/纯文本） | `packages/client/ui-sidebar-documentpreview/src/client/index.ts`；`packages/client/ui-sidebar-documentpreview/src/client/rpc.ts` | `ctx.sidebarRightTabs.register(kind='text')`；`ctx.documentPreviews.register`；RPC `workspaceFiles.read`/`readBytes`、`officeToPdf` | 支持源码与渲染视图切换、行号跳转高亮（`params.line`）、大文件有界读取、Office 转 PDF 预览及沙箱 HTML 预览 |
| 右侧栏：内嵌沙箱浏览器标签页 | `packages/client/ui-sidebar-browser/src/client/index.ts`；`packages/client/ui-sidebar-browser/src/client/pages.ts` | `ctx.sidebarRight.openTab('browser', { params: { url } })`；快捷键 `browser.new` | 提供地址栏、前进/后退/刷新、外部打开与沙箱开关；上游在 `desktop` profile 默认启用，普通 `web` profile 默认禁用 |
| 子代理（Subagent）侧栏目录树、父子血缘面包屑、子会话视图与手动续接/中断 | `packages/client/ui-subagent/src/client/slots.ts`；`packages/client/ui-subagent/src/client/sidebar-chat/` | `ctx.sessions.subagentCatalog`；投影 `subagent`；RPC `subagents/prompt`、`subagents/interruptByUser`；资源 `dsh-resource://subagentchat/...` | 在侧栏会话行下展示子代理树与运行状态；支持在主视图（带父子面包屑）或右栏打开子代理 transcript，并直接向子代理发消息或中断 |
| Agent Team 协作开关与共享任务板（Team Board） | `packages/client/ui-agent-team/src/client/mount.ts`；`packages/client/ui-agent-team/src/client/TeamBoard.tsx` | RPC `agentTeams.setEnabled`；会话投影 `useProjection('agentTeam')` | 在子代理树区域提供 Team 开关（关闭需确认）及共享团队任务状态看板 |
| 会话顶栏后台任务（Background Jobs）名册、流式输出面板与两击停止 | `packages/client/ui-jobs/src/client/JobListAction.tsx`；`packages/client/ui-jobs/src/client/index.ts` | `ctx.jobs`（`job-controller` RPC `job.list`、`job.follow`、`job.kill`）；slot `conversation.session.header` | 当会话存在后台任务时在会话顶栏显示任务按钮，点击展开实时输出流并支持两次点击确认杀死任务 |
| 自动化计划任务（Schedules）全局管理页、右栏详情页、会话顶栏时钟与行标记 | `packages/client/ui-schedule/src/client/TaskManagerPage.tsx`；`packages/client/ui-schedule/src/client/ScheduleTaskTab.tsx` | RPC `schedule.*`（`catalog`/`create`/`edit`/`run`/`delete`/`deliveryHistory`/`deliveryLimits`）；main panel `'schedules'` | 侧栏打开“自动化任务”全局面板或右栏标签页，支持创建/编辑/立即运行/暂停/恢复/删除定时任务，查看投递历史，并在目标会话显示时钟标记 |

---

## 2. 消息与流式

| 功能点 | 上游位置 | 依赖的后端能力 | 备注 |
|---|---|---|---|
| 会话事件流跟随、增量块折叠、断线重连窗口替换与历史向前分页 | `packages/client/ui-conversation/src/client/conversation/assembly.ts`；`packages/client/ui-conversation/src/client/conversation/assembler.ts`；`packages/api/session-controller/src/client/sessions/session.ts` | RPC 流 `session/follow({ activate, assistantStream })`；分页 RPC `session/page`；事件 `session/event`、实时帧 `agent/assistant-stream`（客户端映射为 `assistant/live-chunk`） | 首帧返回 header、尾部事件页与完整投影基线；流式 chunk 按动画帧合批渲染，收到持久 `assistant/message` 时执行 `settle-assistant` 原子替换临时流式块，不留残影 |
| 对话视图切换（Chat 视图 vs Trajectory 轨迹检查器视图）与正文宽度拖拽 | `packages/client/ui-conversation/src/client/skeleton/ConversationSession.tsx`；`packages/client/ui-conversation/src/client/skeleton/ConversationWidthControls.tsx` | `ctx.configForms.developerTools.enabled`（`ui-settings.enabled`）；`localStorage` | 开启“代码工作工具”时在会话顶栏显示 Chat / Trajectory 视图切换；主对话区两侧可拖拽调整正文列宽 |
| Lexical 富文本输入框（Composer）、跨会话草稿持久化与结构化 Chip | `packages/client/ui-conversation/src/client/input/facade.ts`；`packages/client/ui-conversation/src/client/input/editor/runtime.ts`；`packages/client/ui-conversation/src/client/skeleton/InputBar.tsx` | `ctx.conversation`；scoped 事件 `slash/input-begin-command`、`slash/input-insert-reference`、`slash/input-insert-text`；`localStorage` | 按会话独立保存未发送文本、引用 Chip 与引用块（Quote）；命令 Chip 首次退格选中、二次退格删除；点击引用 Chip 在右栏打开对应预览 |
| 消息提交、发送中锁定、本地回显（Echo）与失败恢复草稿 | `packages/client/ui-conversation/src/client/service.ts`；`packages/client/ui-conversation/src/client/input/machine.ts`；`packages/client/ui-chat/src/client/conversation-nodes/message.ts` | `session.beginSubmission()` → RPC `session/prompt(content, mode, signal, requestId)`；`pendingSubmissions` | 发送期间编辑器只读；Host 确认接收或带相同 `requestId` 的 Inbox/消息事件到达后清空草稿并转正回显；若被拒或传输断开则恢复原草稿 |
| 运行中繁忙提交策略（Queue 排队 vs Steer 插话）与空草稿一键全量插话 | `packages/client/ui-conversation/src/client/input/submission-policy.ts`；`packages/client/ui-conversation/src/client/input/editor/keymap.ts` | Host 设置 `ui-conversation.busyEnter`；RPC `session/prompt(..., 'queue' \| 'steer')`、`session/updateQueue(..., { kind: 'steer' })` | 会话运行中默认 Enter 为排队（Queue），`Ctrl/Cmd+Enter` 触发互补模式（Steer 插话），可在设置互换默认行为；空草稿下按加速键将全部排队项按 FIFO 转为立即插话 |
| 待发队列停靠栏（QueueDock）：排队预览、行内编辑、删除与单条转 Steer | `packages/client/ui-conversation/src/client/queue/QueueDock.tsx`；`packages/client/ui-conversation/src/client/service.ts` | 会话投影 `inbox['next-turn']`；RPC `session/updateQueue`（`edit`/`remove`/`steer`）；`session.readAttachment` | 输入框上方展示排队中的下一轮消息，支持纯文本行重新编辑、移除或立即插话 |
| 停止当前轮次按钮与双击 `Esc Esc` 快速中断 | `packages/client/ui-conversation/src/client/stop-shortcut.ts`；`packages/client/ui-conversation/src/client/stop-sequence.ts` | `ctx.conversation.cancel()` → RPC `session/cancel`（`keepInbox: true`）；`useAccess('session/cancel')` | 点击停止按钮或 500ms 内连续按两次 `Esc` 取消当前运行轮次，保留 QueueDock 中尚未消费的排队消息（模态框、审批、终端聚焦或 IME 输入中不误触） |
| 异常中断后“继续生成”（Continuation）按钮 | `packages/client/ui-conversation/src/client/skeleton/InputBar.tsx`；`packages/api/session-controller/src/continuation.ts` | 会话投影 `conversationContinuation`；RPC `session/prompt([], 'queue')`（记录 `source.kind: 'retry'`） | 当上一轮因 `RATE_LIMIT`、`SERVER`、`TIMEOUT`、`TRANSPORT`、`EMPTY_RESPONSE`、`STREAM_CLOSED` 中断且当前空闲无草稿时，显示“继续”按钮直接续写 |
| 输入触发器管线（`/` 与 `@` 浮动建议菜单） | `packages/client/ui-input-trigger/src/client/service.ts`；`packages/client/ui-input-trigger/src/core/` | `ctx.inputTriggers`；slot `conversation.input.overlay` | 监听光标处 `/` 与 `@` token，弹出分组候选面板，支持键盘上下导航、Tab/Enter 选择、子级下钻与过滤 |
| 斜杠命令（`/`）发现、参数补全、弹窗选择器与执行 | `packages/client/ui-commands/src/client/service.ts`；`packages/client/ui-commands/src/client/contract.ts` | RPC `commands.list`、`commands.execute`；`ctx.commandUi`；事件 `commands/change`、`agent-preset/selected` | 支持三类交互：行首命令 Chip（`leadingInput`）、弹窗选择（`popupSelect`，如 `/model`、`/permission`）、即时动作/远端执行（如 `/compact`、`/export`、`/plan`、`/feedback`） |
| `@file` / `@folder` / `@session` 联想引用与校验 | `packages/client/ui-reference/src/client/index.ts` | RPC `fileReferences.list`、`sessionReferenceResolver.candidates`；`ctx.sidebarRight.openResource` | 输入 `@` 检索工作区文件/目录（支持目录逐层下钻）与历史会话引用，插入为原子 Chip |
| 技能（`/skill`）斜杠调用与内联建议 | `packages/client/ui-skill/src/client/index.ts` | RPC `skills.list`；`ctx.uiConversation.userText` | 将可用 Skill 注入 `/` 命令菜单，选中后插入技能调用并随消息提交 |
| 附件选择、剪贴板粘贴、全窗口文件拖放与分块流式上传 | `packages/client/ui-attachment/src/client/ComposerAttachments.tsx`；`packages/client/file-upload/src/client/index.ts`；`packages/client/ui-file-drag/src/client/index.ts` | `ctx.fileUpload.upload`/`uploadImage`（独立流式上传端点）；`ctx.fileDrag`；投影 `imageLimits`；`useAccess('fileUploads/upload')` | 支持 PNG/JPEG/WebP/GIF 图片与通用文件附件、全视口拖放遮罩、上传进度与失败重试；应用内可将文件树条目拖入输入框 |
| 目标管理条（GoalBar）：显示、编辑、暂停、恢复与清除当前会话目标 | `packages/client/ui-goal/src/client/index.ts`；`packages/client/ui-goal/README.zh.md` | 会话投影 `useProjection('goal')`；RPC `goals.get`、`goals.edit`、`goals.pause`、`goals.resume`、`goals.clear`；事件 `goal/activation-changed` | 在输入区上方展示当前活跃 Goal，支持行内弹窗编辑、暂停/恢复或清除，以及 `/goal` 命令引导 |
| 上下文占用率圆环（ContextMeter）与组成明细弹窗 | `packages/client/ui-conversation/src/client/skeleton/ContextMeter.tsx` | 会话投影 `contextPressure`、`contextBreakdown` | 在输入栏显示上下文已用百分比圆环，点击弹出 System / Tools / Messages 各自 Token 估算占比 |
| Assistant 流式渲染（正文增量、思考过程 `reasoning-delta`、工具参数流、重试状态） | `packages/client/ui-chat/src/client/conversation-nodes/assistant.ts`；`packages/client/ui-chat/src/client/chat/ReasoningRow.tsx` | `step/start`、`assistant/live-chunk`（`text-delta`/`reasoning-delta`/`tool-call-delta`/`block-start`/`block-end`/`usage`）、`assistant/message`、`llm/retry` | 实时渲染思考折叠行与回复正文；发生 `llm/retry` 时清空当前残缺尝试并显示重试提示，定稿后转为持久消息块 |
| GFM Markdown 渲染、宽表格横向滚动、安全链接过滤与行内文件链接解析 | `packages/client/ui-chat/src/client/chat/AssistantMarkdown.tsx`；`packages/client/ui-primitives/src/markdown/MarkdownText.tsx`；`packages/client/ui-primitives/src/markdown/file-link.ts` | 会话投影 `workingDirectory`；Host 设置 `ui-chat.linkOpening`；`ctx.sidebarRight.openResource`/`openTab` | 禁用原始 HTML 与危险协议；流式期间仅重解析尾部块；本地工作区路径（含 `#L24`、`:24:5` 行列号）点击在右栏预览对应行，HTTP(S) 链接按设置在内嵌浏览器或外部打开 |
| 代码块语法高亮、流式未闭合 Fence 增量高亮、一键复制与自动换行切换 | `packages/client/ui-primitives/src/markdown/CodeBlock.tsx`；`packages/client/ui-primitives/src/markdown/highlight.ts` | 前端 Shiki 按需语法高亮；无后端 RPC | 支持流式输出中途的未闭合代码块实时高亮、复制纯代码（不含行号）、切换长行折行与可选行号显示（单行超 1000 字符自动跳过高亮防卡顿） |
| KaTeX 数学公式渲染与选中带 TeX 源码复制 | `packages/client/ui-primitives/src/markdown/katex.tsx`；`packages/client/ui-primitives/src/markdown/copy-tex.ts`；`apps/web/katex-fonts.ts` | 定稿 Assistant Markdown；WOFF2 KaTeX 字体；无后端 RPC | 流式期间保持公式字面文本防抖动，定稿后用 KaTeX 渲染行内/块级公式；复制选区时自动将公式还原为 `$...$` / `$$...$$` TeX 文本 |
| 消息图片画廊、历史附件懒加载缓存与全屏原图灯箱（Lightbox） | `packages/client/ui-attachment/src/client/MessageImages.tsx`；`packages/client/ui-conversation/src/client/conversation/historical-images.ts` | `session.readAttachment(attachment)`；`ctx.uiConversation.imageUrl` | 用户与助手消息中的图片以网格画廊展示，按会话缓存 Blob URL；点击打开原图灯箱，按 `Esc` 关闭 |
| 已发送用户文本中的敏感凭据自动遮挡与单值显隐切换 | `packages/client/ui-secret-mask/src/client/index.ts`；`packages/client/ui-secret-mask/src/client/SecretText.tsx` | `ctx.uiConversation.userText.register()`；slot `conversation.user.text.node`；无后端 RPC | 自动识别已发送用户消息中的 API Key、GitHub/npm/云厂商 Token、JWT、PEM 私钥与 URI 密码并打码（保留前 8 位），提供独立“眼睛”按钮切换显隐，不影响复制原文 |
| 过程分组折叠（简洁/标准/详细/完全展开四档）与轮次折叠策略 | `packages/client/ui-chat/src/client/conversation-nodes/process-groups.ts`；`packages/client/ui-chat/src/client/conversation-nodes/turn-process.ts` | Host 设置 `ui-chat.transcriptView`、`ui-chat.collapseTiming`、`ui-chat.foldedReplies` | 将思考与连续工具调用折叠为过程摘要组；支持设置四档详略模式、完成时/下次输入时自动收起，失败或中断轮次保持展开 |
| 历史轮次快速导航轨（Turn Navigator Rail）、保持语义滚动锚点与“回到底部”按钮 | `packages/client/ui-chat/src/client/chat/TurnNavigator.tsx`；`packages/client/ui-chat/src/client/chat/use-chat-scroll.ts` | 会话投影 `turnOutline`；`session.loadOlder()`、`session.loadThrough(seq)` | 右侧悬浮轮次大纲条，展示每轮提问/回复摘要，点击未加载的早期轮次自动按 seq 分页加载并精准定位；滚离底部时显示回到底部按钮 |
| 消息操作栏（复制全文、时间戳、从该轮次创建分支 Fork Session） | `packages/client/ui-chat/src/client/chat/MessageIconActions.tsx`；`packages/client/ui-chat/src/client/chat/TurnTailNodeView.tsx` | RPC `ctx.sessions.fork({ sessionId, atSeq, increaseTitle: true })`；`useAccess('session/fork')`；`ctx.uiWorkspace.openSession` | 用户消息与收尾助手消息提供复制按钮；已完成轮次末尾提供“分支（Fork）”按钮，按真实 `turn/end.seq` 派生新会话并自动跳转 |
| 选区浮动引用（Quote）与消息文本右键操作 | `packages/client/ui-chat/src/client/chat/text-interaction.ts`；`packages/message-quotes` | `ctx.uiConversation.interactions`；`inputActions.insertQuote()`；slot `conversation.chat.text-interaction` | 在用户消息或已定稿助手消息中划选文本或右键，可一键将选区作为结构化 Quote 插入当前输入框 |
| 消息赞/踩评价与会话反馈弹窗（含 `/feedback`） | `packages/client/ui-message-feedback/src/client/MessageFeedbackActions.tsx`；`packages/client/ui-message-feedback/src/client/FeedbackDialog.tsx` | RPC `messageFeedback.list`/`put`/`delete`（带 `ifVersion` CAS）、`sessionFeedback.record`；`ctx.commandUi.decorate({ name: 'feedback' })` | 收尾助手消息支持点赞/点踩、撤回评价，点踩或执行 `/feedback` 打开七分类反馈与备注弹窗 |
| 会话与单轮 Token 用量、生成速度、缓存命中统计胶囊（StatsPills） | `packages/client/ui-chat/src/client/chat/StatsPills.tsx`；`packages/client/ui-chat/src/client/chat/TurnUsagePanel.tsx` | 会话投影 `sessionStats`、`tokenUsage`；`turn/start`/`turn/end`、`assistant/attempt`/`assistant/message`；设置 `ui-chat.performanceUsage` | 显示会话累计轮数/步数、单轮输入/输出/思考/缓存 Token 明细、生成速率（tok/s）与耗时弹窗 |
| 上下文压缩（Compaction）节点、命令执行回执、轮次错误行与最大输出截断提示 | `packages/client/ui-chat/src/client/conversation-nodes/compaction.ts`；`packages/client/ui-chat/src/client/conversation-nodes/command.ts`；`packages/client/ui-chat/src/client/conversation-nodes/turn-error.ts`；`packages/client/ui-chat/src/client/conversation-nodes/turn-max-tokens.ts` | 事件 `compaction/start`/`summary`/`end`、`command/run`/`done`、`turn/end`（error / max_tokens） | 在对话流中渲染上下文自动/手动压缩摘要卡片、斜杠命令回执、模型报错行及达到 `max_tokens` 的续写提示 |
| 运行中状态指示器（鲸鱼图标 + 已用时间计时器） | `packages/client/ui-chat/src/client/chat/RunningStatus.tsx` | Session `running` 状态；`ctx.connection.state`；`turn/start` 时间戳 | 会话运行期间在底部显示动态鲸鱼与耗时读秒；连接断开期间本地读秒暂停，重连后按真实起点恢复 |

---

## 3. 工具与展示卡片

| 功能点 | 上游位置 | 依赖的后端能力 | 备注 |
|---|---|---|---|
| 工具调用树（根工具调用 + PTC 编程工具调用子树）与通用兜底卡片 | `packages/client/ui-tool/src/client/tool/ToolCallTree.tsx`；`packages/client/ui-tool/src/client/tool/toolviews/GenericToolCard.tsx` | `assistant/live-chunk`（`tool-call-delta`）；事件 `tool/call`、`tool/result`、`tool/ptc-dispatch-start`、`tool/ptc-dispatch`；slot `tool.call.toolview` | 按工具名路由专用展示卡片；未知工具或畸形数据自动回退到可展开查看原始输入/输出的 `GenericToolCard`，并支持跳转到 Trajectory 检查器 |
| Auto-review 安全策略自动拒绝卡片 | `packages/client/ui-tool/src/client/models/auto-review-denial.ts`；`packages/client/ui-tool/src/client/tool/ToolCallTree.tsx` | `tool/result` 中携带的 auto-review 拒绝元数据 | 当工具调用被自动安全审查拦截时，优先渲染拒绝原因卡片并折叠原始参数 |
| 文件读取卡片（`read`）与行号定位预览 | `packages/client/ui-tool/src/client/tool/toolviews/read-row.tsx`；`packages/client/ui-tool/src/client/models/read-card-model.ts` | `tool/call`（`file_path`/`offset`/`limit`）；`tool/result` `meta.path`/`lines`/`totalLines`/`lang` | 展示读取文件路径与行范围，展开显示带语法高亮与行号的代码片段，点击路径在右栏打开对应文件并定位到起始行 |
| 图片读取卡片（`read_image`） | `packages/client/ui-tool/src/client/tool/toolviews/read-image-row.tsx`；`packages/client/ui-tool/src/client/models/image-card-model.ts` | `tool/result` 中的图片附件引用与 `meta.path`；`session.readAttachment` | 展示工具读取的图片缩略图、分辨率、文件大小与路径，点击可打开灯箱 |
| 文件写入/编辑/补丁差异卡片（`write` / `edit` / `apply_patch`） | `packages/client/ui-tool/src/client/tool/toolviews/file-mutation-row.tsx`；`packages/client/ui-tool/src/client/models/diff-card-model.ts` | 流式工具参数；`tool/result` `meta.diffs`、`meta.path`、`meta.stopped` | 参数生成阶段即可实时预览预期 Diff，执行完成后展示权威增删行数与统一/分栏语法高亮 Diff，支持一键在右栏打开目标文件 |
| 终端命令执行卡片（`bash` / `pwsh` / `terminal_send`） | `packages/client/ui-tool/src/client/tool/toolviews/bash-sample.tsx`；`packages/client/ui-tool/src/client/models/terminal-card-model.ts` | `tool/call`（`command`/`workdir`/`run_in_background`）；`tool/result` 输出文本、退出码/信号与 `meta.cwd` | 展示执行的命令、工作目录、终端等宽输出流以及非零退出码/信号警告胶囊 |
| Todo 写入差异卡片（`todo_write`）与输入框顶部常驻 Todo 进度面板（TodoPanel） | `packages/client/ui-tool/src/client/tool/toolviews/todo-row.tsx`；`packages/client/ui-tool/src/client/models/todo-diff-model.ts`；`packages/client/ui-conversation/src/client/skeleton/TodoPanel.tsx` | 事件 `todo/write`、`tool/call`（`todo_write`）；会话投影 `todos` | 对话流中 `todo_write` 卡片展示完成度（`done/total`）与本次新增/状态变更/删除的 Todo Diff；输入框上方常驻可折叠 `TodoPanel` 展示当前全量待办进度 |
| 搜索工具卡片（`grep` / `glob`） | `packages/client/ui-tool/src/client/tool/toolviews/search-row.tsx`；`packages/client/ui-tool/src/client/models/search-card-model.ts` | `tool/call`（`pattern`/`include`/`path`）；`tool/result` `meta.shape`/`files`/`paths`/`total`/`truncated` | 结构化展示匹配文件数、匹配行内容或路径列表，点击结果项直接在右栏打开对应文件与行号 |
| 网页搜索与网页抓取卡片（`web_search` / `web_fetch`） | `packages/client/ui-tool/src/client/tool/toolviews/web-row.tsx`；`packages/client/ui-tool/src/client/models/web-card-model.ts` | `tool/call`（`queries`/`url`）；`tool/result` `meta.answer`/`sources`/`url`/`statusCode`/`truncated` | 展示搜索查询词、来源站点列表与摘要，或抓取 URL 与 HTTP 状态码 |
| 技能加载卡片（`skill` 工具行） | `packages/client/ui-skill/src/client/SkillRow.tsx` | `tool/call`（`skill`）；`ctx.sidebarRight.openResource`；slot `tool.call.toolview` | 展示加载的 Skill 名称与可展开的 `Instructions` 说明卡片，支持在右栏预览 `SKILL.md` |
| 控制类与检查类结构化工具详情卡（Subagent/Job/Schedule/Goal/Workflow/LSP/SessionQuery 等） | `packages/client/ui-tool/src/client/tool/toolviews/details-row.tsx`；`packages/client/ui-tool/src/client/models/control-details-model.ts`；`packages/client/ui-tool/src/client/models/inspection-details-model.ts` | 对应工具的 `tool/call` 参数与 `tool/result` `meta` | 为子代理派发、后台任务控制、定时任务创建、目标更新、LSP 查询等提供专属紧凑回执卡片 |
| Cordis 动态插件定义卡片（`cordis_define`）与运行/停止开关 | `packages/extensions/ui-cordis/package.json`；`packages/bundle/web-app/cordis.patch.yml` | `tool-cordis` 事件与 Host inspect providers | 在对话流中渲染 `cordis_define` 动态插件卡片并提供启停开关 |
| 持久化工作流运行折叠面板（WorkflowRunPanel） | `packages/client/ui-workflow-run/src/client/WorkflowRunPanel.tsx`；`packages/client/ui-workflow-run/src/client/workflow-definition.ts` | 会话事件 `tool-workflow/run-start` 等工作流生命周期事件；slot `conversation.chat.node` | 将多阶段工作流（如 Ralph / PTC workflow）渲染为独立的嵌套对话折叠节点，支持查看阶段状态并跳转到成员子会话 |
| 轮次末尾交付物卡片（Changed Files 改动文件汇总卡 + Review Diff 标签页 + Present 显式交付文件卡） | `packages/client/ui-deliverables/src/client/index.ts`；`packages/client/ui-deliverables/src/present-open.ts` | RPC `workspaceChanges.summary`、`workspaceChanges.diff`、`workspaceFiles.stat`；事件 `workspace-changes`、`tool-present`；`ctx.sidebarRight.openResource` | 每个顶层轮次收尾消息下方展示本轮 Git 工作树改动文件汇总卡（可一键在右栏打开完整多文件对比 Review Tab）与 `present` 工具交付的成果文件卡片 |
| 计划模式（Plan Mode）输入栏徽章、轮次末尾计划卡片（PlanCard）与右栏完整计划预览 | `packages/client/ui-plan/src/client/PlanModeControl.tsx`；`packages/client/ui-plan/src/client/PlanCard.tsx`；`packages/client/ui-plan/src/client/PlanPreview.tsx` | 会话投影 `plan`；RPC `commands.execute(sessionId, '/plan off', [])`；`exit_plan_mode` 工具调用 `arguments.plan`；`ctx.sidebarRight.openResource` | 处于 Plan Mode 时在输入栏显示状态徽章与退出按钮；`exit_plan_mode` 生成可持久回看的计划卡片，并在右栏自动打开完整计划 Markdown 预览 |
| 轨迹检查器（Trajectory View）：全量事件表、`run_code` 程序检查、TTFT/吞吐时间线与实时工具原始结果流 | `packages/client/ui-trajectory/src/client/TrajectoryView.tsx`；`packages/client/ui-trajectory/src/client/TrajectoryTable.tsx`；`packages/client/ui-trajectory/src/client/TrajectoryTimeline.tsx` | 会话全量事件（含 `request/header`、`system/message`、`assistant/attempt`）；RPC 流 `toolResults.watch({ sessionId, seq })` | 面向开发调试的平行对话视图：支持按轮次/步骤/子调用三级折叠、搜索、时间轴区间框选缩放、查看 Prompt Header / Tool Schema / 原始 JSON 与 `run_code` 代码高亮 |

---

## 4. 审批与提问

| 功能点 | 上游位置 | 依赖的后端能力 | 备注 |
|---|---|---|---|
| 工具执行审批面板（ApprovalPanel）：接管 Composer、允许一次 / 拒绝与关联命令预览 | `packages/client/ui-approval/src/client/index.ts`；`packages/client/ui-approval/src/client/ApprovalPanel.tsx`；`packages/client/ui-approval/src/client/ApprovalCard.tsx`；`packages/client/ui-chat/src/client/chat/ApprovalCommand.tsx` | Scoped Remote waterfall 事件 `approval/request`（携带 `toolName`/`callId`/`reason`/`displayReason`/`signal`）；`ctx.uiSession.registerPendingInteraction` | 待审批时接管会话底部输入区，展示审批原因与待执行 Shell 命令预览；响应返回 `'allowed-once'` 或 `'rejected'`，支持可选 `onAlways` 扩展与风险说明浮层 |
| 审批快捷键（`Enter` 允许 / `Esc` 拒绝）与一次性防抖锁 | `packages/client/ui-approval/src/client/index.ts`；`packages/client/ui-approval/src/client/ApprovalCard.tsx` | `ctx.shortcuts.registerFixed`（`approval.allow`、`approval.reject`） | 焦点在审批区时按 `Enter` 允许、`Esc` 拒绝；若焦点显式移到“拒绝”按钮上按 `Enter` 则遵循按钮原语义执行拒绝 |
| 会话权限模式切换（`/permission` 弹窗与 Composer 权限位）与默认权限预设 | `packages/client/ui-permission-presets/src/client/index.ts`；`packages/client/ui-permission-presets/README.zh.md` | 会话投影 `permissions`；权限预设目录与切换 RPC；`ctx.configForms.get('permission')`；`ctx.commandUi.decorate` | 支持在通用设置配置新会话默认权限预设，并在当前会话通过 `/permission` 或输入栏控件实时切换会话访问授权模式 |
| 结构化用户提问面板（QuestionComposer）：单选/多选/推荐项、自定义输入、多题翻页与跳过 | `packages/client/ui-user-questions/src/client/index.ts`；`packages/client/ui-user-questions/src/client/QuestionComposer.tsx` | Scoped Remote waterfall 事件 `user-questions/request`（对应工具 `ask_user_question`）；RPC `userQuestions.answer`、`userQuestions.attachWait`；会话投影 `userQuestions` | 接管底部输入区展示单道或多道问题，支持 GFM/TeX 详情说明、推荐项预选、自由文本回答、单题跳过与整批提交；草稿持久化至 `localStorage` |
| 提问倒计时、聚焦自动暂停、“慢慢回答”无限等待与超时后补答（Late Reply） | `packages/client/ui-user-questions/src/client/QuestionComposer.tsx`；`packages/client/ui-tool/src/client/tool/toolviews/ask-question-row.tsx` | RPC `userQuestions.attachWait(sessionId, callId)`；waterfall 超时错误 `ASK_TIMED_OUT`；后续 `userQuestions.answer` 转为 steer 消息 | 用户聚焦或点击“慢慢回答”即通知后端取消倒计时改为无限等待；若倒计时归零放行 Agent，用户仍可从工具行重开卡片补交回答（作为插话消息送达） |
| 已回答问题右对齐回显气泡（QuestionReplyView）与只读回看 | `packages/client/ui-user-questions/src/client/QuestionReplyView.tsx`；`packages/client/ui-user-questions/src/client/question-reply.ts` | `tool/call` + `tool/result` 或 `user/message`（`source.kind: 'user-question-reply'`） | 用户提交的答案在对话流中合并渲染为右对齐问答回显气泡，点击工具行也可打开只读面板核对所选答案 |
| 计划审查专用卡片（PlanReviewPanel：“同意执行” vs “要求修改”） | `packages/client/ui-user-questions/src/client/PlanReviewPanel.tsx`；`packages/client/ui-user-questions/src/client/contract/slots.ts` | `user-questions/request` 中 `intent.kind === 'plan-review'`；slot `conversation.plan-review.actions`（由 `ui-plan` 注入“查看完整计划”） | 当 `exit_plan_mode` 发起计划审批时渲染专用审查面板：“同意执行”回传批准选项标签，“要求修改”以 `ASK_CANCELLED` 退回普通输入框供用户输入修改意见 |

---

## 5. 设置与模型

| 功能点 | 上游位置 | 依赖的后端能力 | 备注 |
|---|---|---|---|
| 设置领域底座：响应式配置表单、乐观锁并发保护与 Schema 驱动绑定 | `packages/client/ui-settings/src/client/index.ts`；`packages/client/ui-settings/README.zh.md` | `ctx.configForms`（`get`/`describe`/`whileServed`）；`ctx.settingsSchema`；RPC `settings.describe`、`settings.mutate`（带 `expectedRevision`）；事件 `settings/document-updated` | 维护客户端与 Host 设置文档的同步镜像；写入冲突时保留用户草稿并刷新远端基线 |
| 设置模态窗口外壳、分区导航与快捷键（`Mod+,`） | `packages/client/ui-settings-general/src/client/index.ts`；`packages/client/ui-settings-general/README.zh.md` | slots `settings.section`、`settings.section.icon`、`settings.onboarding`、`settings.launcher`、`shell.overlay`；`ctx.shortcuts` | 桌面端为 800×800 模态窗口（窄屏铺满视口），支持 `Mod+,` 打开/关闭、`Esc` 退出 |
| 常规设置：界面语言（Locale）切换与多语言词典服务 | `packages/client/locale/src/client/index.ts`；`packages/client/ui-settings-general/src/client/index.ts` | `ctx.locale`；Host 设置命名空间 `locale`；事件 `locale/change` | 支持中/英/德/西/法/日/葡/俄 8 种语言词典，修改后持久化到 Host 设置并立即重绘全界面文案 |
| 常规设置：主题配色（浅色/深色/跟随系统 + 多套内置与扩展主题）、字号与字体族设置 | `packages/client/ui-theme/src/client/index.ts`；`packages/client/ui-theme/src/styles/` | `ctx.theme`；Host 设置命名空间 `ui-theme`；Desktop 字号缩放快捷键 | 管理 `--dsw-*` 颜色 Token、正文/代码/终端字号与字体列表，实时投影到 `document` |
| 常规设置：对话显示偏好（过程折叠档位、折叠时机、性能统计显隐、繁忙回车模式、链接打开方式） | `packages/client/ui-settings-general/README.zh.md`；`packages/client/ui-chat/src/chat-settings.ts`；`packages/client/ui-conversation/src/submission-settings.ts` | `ctx.configForms.get('ui-chat')`、`ctx.configForms.get('ui-conversation')` | 控制对话过程分组默认档位、Enter/Ctrl+Enter 队列或插话行为、链接在右栏或新标签页打开 |
| 常规设置：代码工作工具开关（Developer Tools）、打开原始配置文件与版本信息 | `packages/client/ui-settings-general/src/client/index.ts` | `ctx.configForms.developerTools`（`ui-settings.enabled`）；RPC `settings/openSettingsDocument`；`DSH_CLIENT_VERSION` | 控制 Trajectory 视图、改动文件卡、自定义 Preset 等专业特性显隐；本机连接时支持一键用系统编辑器打开配置文件 |
| 常规设置：API 会话日志上传开关 | `packages/client/ui-settings-session-log/src/client/index.ts` | `ctx.configForms.get('session-log-deepseek')`；slot `shell.overlay` | 当 Host 挂载 `session-log-deepseek` 时在常规设置显示日志上传授权开关与说明弹窗 |
| 常规设置：新会话默认 Agent Preset 选择、查看 Preset YAML 与创建自定义 Preset 草稿 | `packages/client/ui-settings-general/README.zh.md`；`packages/client/ui-agent-preset/src/client/index.ts` | RPC `agentPresets/list`、`agentPresets/read`；Host 设置 `agent-preset-registry`；`ctx.uiWorkspace.startSession` | 展示 `standard`、`ptc`、`minimal`、`cordis` 等预设卡片，支持设为新会话默认、只读预览 YAML 定义或启动 `cordis` 预设创建新预设 |
| 会话内模型选择器（Composer 模型位 + `/model` 弹窗 + 推理强度档位选择） | `packages/client/ui-model-selection/src/client/index.ts`；`packages/client/ui-model-selection/README.zh.md` | RPC `session.models` / `session.modelCatalog`、`session.selectModel`；`ctx.commandUi`；slot `conversation.input.model`；事件 `llm/adapters-updated`、`llm/models-updated` | 在输入框底部与 `/model` 命令提供当前会话模型切换及推理等级（Reasoning Effort）调整；切换对下一轮或新步骤生效 |
| 模型设置页（Models）：官方与目录提供商列表、自定义 Provider（OpenAI/Anthropic 等协议 via `llm-pi-ai`）增删改 | `packages/client/ui-settings-models/src/client/ModelsSection.tsx`；`packages/client/ui-settings-models/src/client/ProviderEditor.tsx`；`packages/client/ui-settings-models/src/client/CustomProviderCard.tsx` | RPC `llm.listProviders`、`llm.listConfigurableProviders`、`session.modelCatalog`、`settings.mutate`；事件 `llm/adapters-updated` | 支持配置 Base URL、协议类型、上下文容量、输入模态（文本/图片）、推理档位，以及拖拽/键盘调整模型显示顺序 |
| 模型设置页：从 Provider 端点在线拉取可用模型列表（Discover Models） | `packages/client/ui-settings-models/src/client/ModelListEditor.tsx`；`packages/client/ui-settings-models/src/client/operations.ts` | RPC `llm.discoverModels(settingsNs, { provider, baseURL, api, apiKey? })` | 支持携带已存或未保存草稿 API Key 探测远端 `/v1/models` 等端点，弹窗搜索、多选并导入模型列表 |
| 内置插件设置壳（Settings → Built-in Plugins）与横向标签栏 | `packages/client/ui-settings-plugins/src/client/index.ts`；`packages/client/ui-settings-plugins/src/client/PluginsSettingsSection.tsx` | slots `settings.section`、`settings.plugins.tab` | 作为内置插件设置项的容器分区，汇总各内置能力配置页与只读 Loader 清单页 |
| 内置插件设置：Agent 循环设置（并行工具调用上限） | `packages/client/ui-settings-agent-loop/src/client/index.ts` | `ctx.configForms.whileServed(['agent-loop'])`；`ctx.configForms.get('agent-loop')` | 配置单步骤最大并行工具调用数等 Agent Loop 参数 |
| 内置插件设置：子智能体（Subagent）递归深度、并发容量与可用模型授权 | `packages/client/ui-settings-subagent/src/client/index.ts` | `ctx.configForms.get('subagent')`、`ctx.configForms.get('subagent-model-selection')`；RPC `session.modelCatalog()` | 独立配置子代理递归深度/并发上限，以及允许子代理选用的模型白名单 |
| 内置插件设置：网页搜索与抓取（Web Search）Provider 与结果数配置 | `packages/client/ui-settings-web-search/src/client/index.ts` | `ctx.configForms.get('web' / 'web-search-deepseek' / 'web-deepseek-api')`；RPC `web.listProviders`；事件 `web/providers-changed` | 选择网页搜索后端提供方、配置单次搜索结果上限等参数 |
| 内置插件设置：终端沙箱与命令执行限制（Shell Settings） | `packages/client/ui-settings-shell/src/client/index.ts` | `ctx.configForms.whileServed`；`ctx.configForms.get('bash-sandbox' / 'pwsh-sandbox' / 'shell')` | 配置终端命令默认超时时间、单流输出字节上限与沙箱策略 |
| 内置插件设置：Host Loader 只读插件清单（Plugin Inventory）与页面同步重试 | `packages/client/ui-settings-plugin-inventory/src/client/index.ts`；`packages/client/ui-settings-plugin-inventory/src/client/PluginInventorySettingsTab.tsx` | RPC `pluginInventory.list()`；`ctx.modules.entries.retry()` | 按会话 Preset 与全局 Host 分组只读展示全部 Cordis Loader 条目、启用条件与运行状态，支持搜索及对同步失败的前端模块执行重试 |

---

## 6. 插件与凭据

| 功能点 | 上游位置 | 依赖的后端能力 | 备注 |
|---|---|---|---|
| 侧栏“插件”管理面板（Plugin Manager）：已安装 Bundle 列表、内置扩展、状态诊断与手动刷新 | `packages/client/ui-plugin-manager/src/client/index.ts`；`packages/client/ui-plugin-manager/src/client/PluginManagerPage.tsx`；`packages/client/ui-plugin-manager/src/client/manager-store.ts` | `ctx.remoteAccess.whenAccess('pluginManager/listBundles')`；RPC `pluginInventory.list`、`pluginManager.listBundles`、`pluginManager.listPlugins`；事件 `plugin-manager/changed` | 展示运行中/已停用/异常/实验性/可更新状态，支持查看 Bundle 详情 README（`pluginManager.readBundleDetail`） |
| 插件 Bundle 启停、内部组件行（Plugin Row）细粒度启停与配置表单挂载 | `packages/client/ui-plugin-manager/src/client/PluginManagerPage.tsx`；`packages/client/ui-plugin-manager/src/client/manager-store.ts` | RPC `pluginManager.setBundleEnabled(name, enabled)`、`pluginManager.setPluginEnabled(entryId, enabled)`；`ctx.configForms`；slots `plugins.bundle.config`、`plugins.row.config` | 切换启停后区分“已热生效”、“需重启”或“被高层覆盖”；支持在插件详情页直接渲染该插件注册的配置表单 |
| 添加/安装插件（npm 包规格、Git 仓库、压缩包、本地绝对路径）与镜像源自动测速推荐 | `packages/client/ui-plugin-manager/src/client/PluginManagerPage.tsx`；`packages/client/ui-plugin-manager/src/index.ts` | RPC `pluginManager.inspect(spec, { registry })`、`pluginManager.installBundle`、`pluginManager.registries`、`pluginRegistryProbe.fastest` | 自动探测 npm / npmmirror 延迟并推荐最快源；安装前展示安全风险提示；GitHub 超时提供切换镜像引导 |
| 插件安装实时进度弹窗、pnpm 终端日志流、取消安装、断线状态恢复与安装脚本审批（`approvedBuilds`） | `packages/client/ui-plugin-manager/src/client/manager-store.ts`；`packages/client/ui-plugin-manager/src/client/PluginManagerPage.tsx` | RPC `pluginManager.installBundle`、`pluginManager.cancelInstall`、`pluginManager.waitForInstall`；事件 `plugin-manager/install-state`、`plugin-manager/install-log` | 实时展示 pnpm 输出流；若包带构建脚本被拦截，列出包名供用户显式批准“允许这些脚本并重试” |
| 插件版本检查、目标版本选择更新与回滚、卸载确认弹窗 | `packages/client/ui-plugin-manager/src/client/versions.tsx`；`packages/client/ui-plugin-manager/src/client/PluginManagerPage.tsx` | RPC `pluginManager.listVersions`、`pluginManager.updateBundle`、`pluginManager.removeBundle`；事件 `plugin-manager/update-state` | 检查远端可用版本并标记推荐稳定版；更新失败自动尝试恢复旧版；卸载前弹窗确认 |
| 插件市场“发现”标签页（Plugin Market）：索引源管理、搜索/分类/标签过滤、兼容性检查与一键安装 | `packages/client/ui-plugin-market/src/client/index.ts`；`packages/client/ui-plugin-market/src/client/DiscoverSection.tsx`；`packages/client/ui-plugin-market/src/client/market-store.ts` | RPC `pluginMarket.listIndexes`、`pluginMarket.setIndexes`、`pluginMarket.search`、`pluginMarket.readEntryDetail`；事件 `plugin-market/changed` | 支持增删第三方 HTTP(S) 市场索引 URL、250ms 防抖搜索、按标签多选过滤、查看详情文档并调用插件管理器安装 |
| API Key 只写凭据输入、ASCII 格式校验、脱敏状态检测与随 Provider 删除清理 | `packages/client/ui-settings-models/src/client/ApiKeyInput.tsx`；`packages/client/ui-settings-models/src/client/apiKey.ts`；`packages/client/ui-settings-models/src/client/ProviderEditor.tsx` | RPC `credentials.describe(refs)`、`credentials.set(ref, value)`、`credentials.unset(ref)`；事件 `credentials/record-updated`、`credentials/reference-updated` | 已存密钥绝不回读到前端；`describe` 仅返回是否已配置/来源（文件或环境变量）/是否可写；编辑留空保留原密钥，无单独清除按钮，仅在删除自管 Provider 时可一并 `unset` |
| 首启欢迎说明（WelcomeNotice）与 DeepSeek 官方凭据引导弹窗（Onboarding） | `packages/client/ui-settings-models/src/client/WelcomeNotice.tsx`；`packages/client/ui-settings-models/src/client/DeepSeekOnboardingDialog.tsx` | Host 启动注入 `credentialOnboarding`；`ui-settings-general.welcomeNoticeVersion`；RPC `credentials.set`、`settings.mutate` | 当未配置任何可用模型提供商且凭据可写时，首启自动引导用户登录 DeepSeek 账号或填入 API Key（可点“稍后配置”跳过） |
| DeepSeek 账号登录/退出、个人资料与余额展示、充值跳转、登录失效与欠费恢复弹窗 | `packages/client/ui-settings-account/src/client/index.ts`；`packages/client/ui-settings-account/src/client/AccountSection.tsx`；`packages/client/ui-settings-account/src/client/SignInDialog.tsx`；`packages/client/ui-settings-account/src/client/AccountQuotaNotice.tsx` | RPC 流 `account.watch`；RPC `account.startSignIn`、`account.cancelSignIn`、`account.signOut`、`account.getProfile`、`account.getBalance`、`session.initializeDefaultModel`；事件 `deepseek-account/session-expired`、`deepseek-account/model-sign-in-required` | 支撑侧栏底部账号菜单与设置账号页；登录成功后自动初始化默认模型；遇到 `ACCOUNT_QUOTA` 欠费或会话过期时弹出全局恢复提示 |

---

## 7. 其他外围（快捷键、文件动作、连接状态、桌面集成与客户端支撑底座）

| 功能点 | 上游位置 | 依赖的后端能力 | 备注 |
|---|---|---|---|
| 全局快捷键注册与物理按键分发引擎 | `packages/client/shortcuts/src/client/index.ts`；`packages/client/shortcuts/README.zh.md` | `ctx.shortcuts`（命令注册、平台默认键位、冲突检测、模态焦点隔离）；无后端 RPC | 管理全局与作用域快捷键（如 `Mod+B` 侧栏、`Mod+,` 设置、`Esc Esc` 停止、终端/审批键位优先级） |
| 快捷键速查弹窗（Shortcuts Cheat Sheet） | `packages/client/ui-shortcuts/src/client/index.ts`；`packages/client/ui-shortcuts/README.zh.md` | `ctx.shortcuts.catalog`；slot `shell.overlay` | 分类展示当前已激活功能的键盘快捷键与输入操作说明 |
| 统一右键上下文菜单系统（Context Menu） | `packages/client/ui-context-menu/src/client/index.ts`；`packages/client/ui-context-menu/src/client/registry.ts` | `ctx.contextMenus`；`ctx.shortcuts.catalog`；slot `shell.overlay` | 支持各插件按目标类型异步贡献右键菜单项（1000ms 超时保护）并自动标注关联快捷键 |
| “在应用中打开”（Open in App）：会话顶栏拆分按钮与文件/目录外部程序启动 | `packages/client/ui-open-in-app/src/client/index.ts`；`packages/client/ui-open-in-app/src/client/session-open-with.ts` | RPC `session.workspacePathApplications`、`session.openWorkspacePath`、`workspaceFiles.stat`；`ctx.contextMenus` | 探测本机已安装的编辑器/终端/文件管理器（带应用图标），支持在会话顶栏、文件树、工具路径或交付卡片中一键用外部应用打开工作区或定位文件 |
| 文件动作（下载/另存为原始文件、复制图片为 PNG、复制路径） | `packages/client/ui-file-actions/src/client/index.ts`；`packages/client/ui-file-actions/src/client/files.ts`；`packages/client/ui-file-actions/src/client/context-menus.tsx` | `ctx.fileActions`；RPC `workspaceFiles.readBytes`、`workspaceFiles.stat`、`session.openWorkspacePath` | 为文件树、文档预览和消息图片提供右键下载/另存为、复制 PNG 到剪贴板等操作 |
| 连接状态管理、指数退避自动重连、连接恢复药丸提示与 Host 版本变更提示 | `packages/client/connection/src/client/connection.ts`；`packages/client/ui-settings-general/src/client/index.ts`；`packages/client/modules/src/client/release-change.ts` | `ctx.connection`（`state`/`generation`/`reconnect`）；内部 `$events` 流首帧 `ready { clientId, host.home }`；`ctx.modules.entries.releaseChange` | 断线时显示离线/重试状态药丸，重连成功后驻留 2s 恢复提示；若重连后发现 Host 版本（release/commit）变化则弹窗提示刷新 |
| 客户端动态模块加载器与开发期客户端热重载（Client HMR） | `packages/client/modules/src/index.ts`；`packages/client/modules/src/client/system.ts`；`packages/client/hmr/src/index.ts`；`packages/client/hmr/src/client/index.ts` | `ctx.clientModules` / `ctx.modules`；HTTP `/plugins/*` 与 SSE `/plugins/events` | 浏览器插件 bundle 按需加载与开发期修改热替换底座；GTK 原生客户端无需实现浏览器 CJS 模块加载器 |
| 统一资源订阅模型（`useResource`） | `packages/client/resources/src/client/index.ts`；`packages/client/resources/README.zh.md` | `ctx.resources` | 为右栏文档预览、计划预览等提供基于 URI 的统一缓存、流式更新与生命周期管理底座 |
| 响应式快照存储原语（Client Store）與类型化插槽注册表（UI Slots / UI Renderer） | `packages/client/store/src/index.ts`；`packages/client/ui-slots/src/index.ts`；`packages/client/ui-renderer/src/client/index.ts` | 纯前端状态与组件组合底座；无后端 RPC | 提供 `defineStore`/`createSnapshotStore`（支持 rAF 合批）与 Slot 树注册绑定；GTK 客户端以 Rust 领域层 + Relm4/GTK 控件替代 |
| 共享基础控件与图标库（UI Primitives） | `packages/client/ui-primitives/src/index.ts`；`packages/client/ui-primitives/README.zh.md` | 纯前端组件库（依赖 `ui-theme` `--dsw-*` token）；无后端 RPC | 提供按钮、菜单（`MenuSurface`）、弹窗（`Modal`）、Toast、排序列表、图标与 Markdown/代码/Diff/终端输出基础渲染器 |
| 桌面端产品遥测采集（Product Analytics）与系统通知边界 | `packages/client/product-analytics/src/index.ts`；`apps/desktop/src/update-attention.ts` | `ctx.productAnalytics` → `@deepseek-ai/dsh-host-product-telemetry-otel`（仅 `desktop` profile 启用） | `packages/client/*` 内部无 Web Notification API 调用（仅维护 `completionUnread` 与页内 Toast）；系统级通知仅在 `apps/desktop` Electron 壳更新提醒中使用，Linux 客户端由原生桌面通知增强实现 |

---

## 8. `packages/client/*` 全包覆盖索引（共 63 个客户端包）

为确保无遗漏，下表核对 `/tmp/dsh-upstream-ref/packages/client/` 下全部 63 个子目录在本清单中的归属章节：

| 客户端包目录 | 包名 | 归属章节 |
|---|---|---|
| `packages/client/connection` | `@deepseek-ai/dsh-client-connection` | 7. 其他外围 |
| `packages/client/file-upload` | `@deepseek-ai/dsh-client-file-upload` | 2. 消息与流式 |
| `packages/client/hmr` | `@deepseek-ai/dsh-client-hmr` | 7. 其他外围 |
| `packages/client/locale` | `@deepseek-ai/dsh-client-locale` | 5. 设置与模型 |
| `packages/client/modules` | `@deepseek-ai/dsh-client-modules` | 7. 其他外围 |
| `packages/client/product-analytics` | `@deepseek-ai/dsh-client-product-analytics` | 7. 其他外围 |
| `packages/client/resources` | `@deepseek-ai/dsh-client-resources` | 7. 其他外围 |
| `packages/client/shortcuts` | `@deepseek-ai/dsh-client-shortcuts` | 7. 其他外围 |
| `packages/client/store` | `@deepseek-ai/dsh-client-store` | 7. 其他外围 |
| `packages/client/ui-agent-preset` | `@deepseek-ai/dsh-client-ui-agent-preset` | 5. 设置与模型 |
| `packages/client/ui-agent-team` | `@deepseek-ai/dsh-client-ui-agent-team` | 1. 会话与导航 |
| `packages/client/ui-approval` | `@deepseek-ai/dsh-client-ui-approval` | 4. 审批与提问 |
| `packages/client/ui-attachment` | `@deepseek-ai/dsh-client-ui-attachment` | 2. 消息与流式 |
| `packages/client/ui-brand-official` | `@deepseek-ai/dsh-client-ui-brand-official` | 1. 会话与导航 |
| `packages/client/ui-chat` | `@deepseek-ai/dsh-client-ui-chat` | 2. 消息与流式 |
| `packages/client/ui-commands` | `@deepseek-ai/dsh-client-ui-commands` | 2. 消息与流式 |
| `packages/client/ui-context-menu` | `@deepseek-ai/dsh-client-ui-context-menu` | 7. 其他外围 |
| `packages/client/ui-conversation` | `@deepseek-ai/dsh-client-ui-conversation` | 2. 消息与流式 / 3. 工具与展示卡片 |
| `packages/client/ui-deliverables` | `@deepseek-ai/dsh-client-ui-deliverables` | 3. 工具与展示卡片 |
| `packages/client/ui-directory-picker-browse` | `@deepseek-ai/dsh-client-ui-directory-picker-browse` | 1. 会话与导航 |
| `packages/client/ui-directory-picker-native` | `@deepseek-ai/dsh-client-ui-directory-picker-native` | 1. 会话与导航 |
| `packages/client/ui-dockkit` | `@deepseek-ai/dsh-client-ui-dockkit` | 1. 会话与导航 |
| `packages/client/ui-entry-customization` | `@deepseek-ai/dsh-client-ui-entry-customization` | 1. 会话与导航 |
| `packages/client/ui-file-actions` | `@deepseek-ai/dsh-client-ui-file-actions` | 7. 其他外围 |
| `packages/client/ui-file-drag` | `@deepseek-ai/dsh-client-ui-file-drag` | 2. 消息与流式 |
| `packages/client/ui-goal` | `@deepseek-ai/dsh-client-ui-goal` | 2. 消息与流式 |
| `packages/client/ui-input-trigger` | `@deepseek-ai/dsh-client-ui-input-trigger` | 2. 消息与流式 |
| `packages/client/ui-jobs` | `@deepseek-ai/dsh-client-ui-jobs` | 1. 会话与导航 |
| `packages/client/ui-layout` | `@deepseek-ai/dsh-client-ui-layout` | 1. 会话与导航 |
| `packages/client/ui-message-feedback` | `@deepseek-ai/dsh-client-ui-message-feedback` | 2. 消息与流式 |
| `packages/client/ui-model-selection` | `@deepseek-ai/dsh-client-ui-model-selection` | 5. 设置与模型 |
| `packages/client/ui-open-in-app` | `@deepseek-ai/dsh-client-ui-open-in-app` | 7. 其他外围 |
| `packages/client/ui-permission-presets` | `@deepseek-ai/dsh-client-ui-permission-presets` | 4. 审批与提问 |
| `packages/client/ui-plan` | `@deepseek-ai/dsh-client-ui-plan` | 3. 工具与展示卡片 / 4. 审批与提问 |
| `packages/client/ui-plugin-manager` | `@deepseek-ai/dsh-client-ui-plugin-manager` | 6. 插件与凭据 |
| `packages/client/ui-plugin-market` | `@deepseek-ai/dsh-client-ui-plugin-market` | 6. 插件与凭据 |
| `packages/client/ui-primitives` | `@deepseek-ai/dsh-client-ui-primitives` | 2. 消息与流式 / 7. 其他外围 |
| `packages/client/ui-reference` | `@deepseek-ai/dsh-client-ui-reference` | 2. 消息与流式 |
| `packages/client/ui-renderer` | `@deepseek-ai/dsh-client-ui-renderer` | 7. 其他外围 |
| `packages/client/ui-schedule` | `@deepseek-ai/dsh-client-ui-schedule` | 1. 会话与导航 |
| `packages/client/ui-secret-mask` | `@deepseek-ai/dsh-client-ui-secret-mask` | 2. 消息与流式 |
| `packages/client/ui-session` | `@deepseek-ai/dsh-client-ui-session` | 1. 会话与导航 |
| `packages/client/ui-settings` | `@deepseek-ai/dsh-client-ui-settings` | 5. 设置与模型 |
| `packages/client/ui-settings-account` | `@deepseek-ai/dsh-client-ui-settings-account` | 6. 插件与凭据 |
| `packages/client/ui-settings-agent-loop` | `@deepseek-ai/dsh-client-ui-settings-agent-loop` | 5. 设置与模型 |
| `packages/client/ui-settings-general` | `@deepseek-ai/dsh-client-ui-settings-general` | 5. 设置与模型 / 7. 其他外围 |
| `packages/client/ui-settings-models` | `@deepseek-ai/dsh-client-ui-settings-models` | 5. 设置与模型 / 6. 插件与凭据 |
| `packages/client/ui-settings-plugin-inventory` | `@deepseek-ai/dsh-client-ui-settings-plugin-inventory` | 5. 设置与模型 |
| `packages/client/ui-settings-plugins` | `@deepseek-ai/dsh-client-ui-settings-plugins` | 5. 设置与模型 |
| `packages/client/ui-settings-session-log` | `@deepseek-ai/dsh-client-ui-settings-session-log` | 5. 设置与模型 |
| `packages/client/ui-settings-shell` | `@deepseek-ai/dsh-client-ui-settings-shell` | 5. 设置与模型 |
| `packages/client/ui-settings-subagent` | `@deepseek-ai/dsh-client-ui-settings-subagent` | 5. 设置与模型 |
| `packages/client/ui-settings-web-search` | `@deepseek-ai/dsh-client-ui-settings-web-search` | 5. 设置与模型 |
| `packages/client/ui-shortcuts` | `@deepseek-ai/dsh-client-ui-shortcuts` | 7. 其他外围 |
| `packages/client/ui-sidebar` | `@deepseek-ai/dsh-client-ui-sidebar` | 1. 会话与导航 |
| `packages/client/ui-sidebar-browser` | `@deepseek-ai/dsh-client-ui-sidebar-browser` | 1. 会话与导航 |
| `packages/client/ui-sidebar-documentpreview` | `@deepseek-ai/dsh-client-ui-sidebar-documentpreview` | 1. 会话与导航 |
| `packages/client/ui-sidebar-files` | `@deepseek-ai/dsh-client-ui-sidebar-files` | 1. 会话与导航 |
| `packages/client/ui-sidebar-right` | `@deepseek-ai/dsh-client-ui-sidebar-right` | 1. 会话与导航 |
| `packages/client/ui-sidebar-terminal` | `@deepseek-ai/dsh-client-ui-sidebar-terminal` | 1. 会话与导航 |
| `packages/client/ui-skill` | `@deepseek-ai/dsh-client-ui-skill` | 2. 消息与流式 / 3. 工具与展示卡片 |
| `packages/client/ui-slots` | `@deepseek-ai/dsh-client-ui-slots` | 7. 其他外围 |
| `packages/client/ui-subagent` | `@deepseek-ai/dsh-client-ui-subagent` | 1. 会话与导航 |
| `packages/client/ui-theme` | `@deepseek-ai/dsh-client-ui-theme` | 5. 设置与模型 |
| `packages/client/ui-tool` | `@deepseek-ai/dsh-client-ui-tool` | 3. 工具与展示卡片 |
| `packages/client/ui-trajectory` | `@deepseek-ai/dsh-client-ui-trajectory` | 3. 工具与展示卡片 |
| `packages/client/ui-user-questions` | `@deepseek-ai/dsh-client-ui-user-questions` | 4. 审批与提问 |
| `packages/client/ui-workflow-run` | `@deepseek-ai/dsh-client-ui-workflow-run` | 3. 工具与展示卡片 |
| `packages/client/ui-workspace` | `@deepseek-ai/dsh-client-ui-workspace` | 1. 会话与导航 |
| `packages/client/web` | `@deepseek-ai/dsh-client-web` | 1. 会话与导航 |

---

## 9. 使用说明（阶段 3–6 消费方式与最终验收规程）

### 9.1 阶段 3（`docs/web-ui-map.md`）与阶段 4–6 如何消费本清单

1. **阶段 3（Web UI 映射表与 GUI 骨架）**：
   - 以本清单第 1–7 章的每一行为输入，在 `docs/web-ui-map.md` 中逐行建立 **「DSH Web 功能点 → GTK4/libadwaita 原生控件与模块 → 所需 Bridge RPC/通知」** 的一对一映射。
   - 凡属浏览器专有内核机制的行（如 `packages/client/modules`、`packages/client/hmr`、`apps/web/src/preview.ts`、React `ui-slots`/`ui-renderer`/`store`），在映射表中标注为 **「前端框架底座（Rust 领域层/GTK 原生替代）」**，不生搬浏览器加载逻辑。
2. **阶段 4（会话与消息链路）**：
   - 优先消费 **第 1 章（会话与导航）**、**第 2 章（消息与流式）** 与 **第 4 章（审批与提问）** 的核心行：完成工作区与会话树、空白会话复用、历史恢复与归档、流式正文/思考增量与 `settle-assistant` 原子替换、Queue/Steer/Stop（含 `Esc Esc`）、Markdown + GtkSourceView 代码高亮、工具审批面板、结构化用户提问与计划审查面板、会话内模型切换与 Portal 工作目录选择。
3. **阶段 5（首启向导、凭据与设置）**：
   - 消费 **第 5 章（设置与模型）** 与 **第 6 章（插件与凭据）**：完成首启 API Key 引导、Secret Service 凭据存取与向 Bridge 的内存注入、Provider 与模型目录管理（含在线拉取模型）、常规设置与内置插件设置页、插件管理器（列表、启停、安装、更新、卸载）与插件市场。
4. **阶段 6（展示卡片与右侧栏工具移植）**：
   - 消费 **第 3 章（工具与展示卡片）** 及 **第 1/7 章的右栏与外围行**：逐一移植 `read`、`read_image`、`write`/`edit`/`apply_patch` Diff、`bash` 终端卡、`todo_write` 与常驻 `TodoPanel`、`grep`/`glob` 搜索卡、`web_search`/`web_fetch`、`skill`、`exit_plan_mode` 计划卡、轮次末尾 `ui-deliverables` 改动文件与交付卡片、`WorkflowRunPanel`、`Trajectory` 检查器，以及右侧停靠栏的文档预览、文件树与终端标签页。
   - 每移植一张卡片前，先核对本表「依赖的后端能力」列；若 `bridge/` 尚未暴露对应投影或 RPC，先在 `bridge/` 补齐方法与集成测试，再写 GTK 控件。

### 9.2 最终验收（Final Acceptance）逐行核对规程

1. **逐行对表**：在最终验收阶段，验收人基于本清单第 1–7 章的全部功能行，在真实启动的 `dsh-libadwaita` 客户端与同版本上游 `dsh --profile web` 之间执行同场景对照。
2. **状态标记**：验收报告需复制本清单表格并新增 `验收状态` 与 `验收证据/差异说明` 两列，每行仅允许标记为以下三种状态之一：
   - `已实现`：GTK 客户端具备同等用户可见功能与交互语义，且已有集成测试或手动验证证据；
   - `原生替代`：浏览器专有机制已由符合设计文档的 Linux 原生方案等价替代（如 Secret Service 替代明文 `.credentials.yaml`、Portal 替代 Web 目录选择器、GTK 原生渲染替代 React/WebView）；
   - `缺失`：尚未实现或行为与上游存在功能性差距。
3. **通过标准**：除明确列为 Non-goals 的项（如 WebWorker 预览入口、沙箱 WebView 浏览器标签页等嵌入浏览器引擎的特性）外，其余业务功能行全部达到 `已实现` 或 `原生替代`，且所有 `缺失` 项清零（或取得用户显式范围豁免批准），方可视为达到「功能与 DSH Web 无异」的交付目标。
