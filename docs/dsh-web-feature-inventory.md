# DSH Web 功能清单（权威验收基准）

- 上游：https://github.com/deepseek-ai/deepseek-harness（`master`）
- 上游克隆提交：`fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159`（浅克隆于 /tmp/dsh-upstream-ref，只读参考，未执行/未修改）
- npm `@deepseek-ai/dsh` 最新版本：`0.2.2-rc.1`（`npm view @deepseek-ai/dsh version`，2026-11-06）
- 本文每一行均出自实际读过的上游 README / 源码；上游位置以仓库路径标注。读不到、未读到的能力一律标 **未证实**，不作推断。

## 结构结论（先读）

- `apps/web` 只有 **两个 HTML 入口**：`index.html` 与实验性 `preview.html`（WebWorker 预览）。**没有传统 UI 路由**：无 React Router / URL→页面映射；`/api`、`/plugins`、`/connect` 是传输与资源端点。界面由 Host 启动图（`window.__DSH_BOOT__`）加载客户端插件树后挂载（`apps/web/src/main.ts`、`packages/client/web/src/boot.ts`、`vite.config.ts`）。
- "页面" 即插件贡献的全局面板：左侧栏（工作区/会话树、插件页、自动化任务页）、中栏会话视图（Chat / Trajectory）、右侧停靠栏（终端 / 文件 / 文档预览 / 浏览器 tab）与设置面板。启动必需插件：`dsh-client-modules`、`dsh-api-session-controller`、`ui-renderer`、`ui-layout`、`ui-session`、`ui-chat`、`ui-sidebar`（`packages/client/web/src/boot-client.ts`）。
- Web 组合层为 `packages/bundle/web-app`（`cordis.patch.yml` + `presets/{standard,ptc,minimal,cordis}.patch.yml`）；base 层 `packages/bundle/base` 只含核心服务，session/workspace/job/settings/schedule controller 均在 web-app 层挂载——**base-only bridge 不能原样复用 Web 的 `ctx.remote.*` 调用链**。
- 多轮 turn：会话内可见交互（审批、提问、plan review、技能调用）都经 `ctx.remote.$on(...)` waterfall 或投影实现，host 侧对应 `ctx.approval`、`ctx.userQuestions`、`ctx.schedule` 等。

---

## 1. 会话与导航

| 功能点 | 上游位置 | 依赖后端能力 | 备注 |
|---|---|---|---|
| 三栏 AppFrame：侧栏/会话区/右停靠栏，列宽拖拽（侧栏 264–420、右栏默认 45% 视口、上限 70%），<768px 抽屉模式，`shell.bottom`/`shell.leading`/`shell.overlay` 座 | `packages/client/ui-layout/` | `ctx.layout.selectPanel/toggleSidebar/openRightbar/closeRightbar`；root slot `main`/`sidebar`/`rightbar` | 布局状态刷新重置；右栏先让步保中栏 400px；macOS/Windows 原生窗口 chrome 座（`README.zh.md`） |
| 左侧栏：品牌、New Session、面板列表与"更多"菜单、设置入口、折叠轨道 | `packages/client/ui-sidebar/`（`src/client/SidebarRoot.tsx`） | slots `sidebar.workspaces`/`sidebar.settings`/`sidebar.brand.*`/`sidebar.panellist*` | 窄屏抽屉；埋点仅 Desktop |
| 工作区/会话列表：分组/平铺、置顶/最近/手动排序、水位线、归档过滤、搜索 | `packages/client/ui-workspace/`（`rows/Rows.tsx`、`tree.ts`、`WorkspaceBrowser.tsx`） | `ctx.uiWorkspace.openSession/openWorkspace/forkSession`；workspace-controller 投影 | 水位线仅内存；无 Session 删除 |
| 新建会话（复用 blank）、命名创建 | `packages/client/ui-workspace/src/client/navigation.ts`；`packages/api/session-controller/src/agent.ts` | `session/create` → `ApiSessionAgentController.createSession` | 同 Workspace 同 preset 可复用 blank；可指定 cwd/workspaceId/agentPreset |
| 打开/恢复既有会话（冷列表 ≠ 恢复） | `packages/client/ui-workspace/src/client/navigation.ts`；`packages/api/session-controller/src/client/transport.ts` | `session/follow({activate:true,assistantStream:true})` → `resolveAgent/resume` | 打开历史才请求后台激活；恢复组合 preset + installModelSelection |
| 启动时恢复上次会话/工作区 | `packages/client/ui-workspace/src/client/navigation.ts` | 浏览器持久化 `dsh.sessions.current`；`workspaces.initializeDefault` | blank 会重新核对 preset |
| 品牌呈现：侧栏官方图形与首页鲸鱼动画层 | `packages/client/ui-brand-official/` | slots `sidebar.brand.*`、`conversation.hero.frame` | profile 门控 |
| "关闭"当前会话视图 | `packages/client/ui-workspace/src/client/navigation.ts` | `clearMain()` 释放 mainView retain | 不是 Host close：不停止 Agent、不删日志。上游无 `session/close` RPC（已搜索证实） |
| 归档/取消归档（活动会话先确认再停止） | `packages/client/ui-workspace/src/client/index.ts`；`packages/api/workspace-controller/src/commands.ts` | `workspace/archiveSession`、`workspace/unarchiveSession`；`workspace/session-stop` → `agent.cancel({kind:'user'})` | 停止并归档丢弃 pending inbox（`packages/core/agent/src/archive-admission.ts`），与普通 Stop 不同 |
| 会话重命名（菜单/双击标题/快捷键） | `packages/client/ui-workspace/src/client/session-actions/RenameSession.tsx` | `session/rename` → `ctx.sessionTitle.rename` | `source:{kind:'user'}` pin 标题 |
| 自动会话标题（首条 prompt → LLM，含确定性 fallback） | `packages/session/session-title/`、`packages/session/session-title-first-prompt-llm/` | `session/event(user/message)`、`ctx.sessionTitle.register`、`llm/stream` | 用户显式命名后不再自动生成 |
| Fork 会话（完成轮次页脚分支） | `packages/client/ui-chat/src/client/chat/MessageIconActions.tsx` | `session/fork`（`atSeq`、`increaseTitle`） | 分支成功后打开子会话 |
| 会话搜索：元数据本地过滤 + 内容搜索 | `packages/client/ui-workspace/`；`packages/api/session-controller/src/list.ts` | `session/list`、`session/search`；Host `ctx.sessionQuery` | base 中 `session-query-sqlite` `openAt:'never'`，web patch 改为 `:memory:`/`never`；全文搜索需显式启用 |
| 会话行菜单、Pin、Archive、Toast | `packages/client/ui-workspace/src/client/session-actions/`、`context-menus.tsx` | `ctx.contextMenus`；workspace RPC | |
| Workspace 层级/创建/重命名/删除、目录选择（原生或应用内浏览） | `packages/client/ui-workspace/`；`packages/client/ui-directory-picker-native/`、`ui-directory-picker-browse/` | `workspace/create`（幂等）；`directoryPicker/pick|list|createDirectory` | 切换 Workspace = 打开新 blank 会话并迁移草稿，**非修改现有会话 cwd**；未证实存在通用 cwd 写 API |
| 右停靠栏框架：每会话停靠面、tab 注册、split/float/fullscreen/关闭、tab 引导页 | `packages/client/ui-sidebar-right/`；`packages/client/ui-dockkit/` | `ctx.sidebarRight.*`、`ctx.sidebarRightTabs.register`；localStorage `dsh.sidebar-right.v1.<sessionId>` | 无会话无停靠面；布局不跨设备同步 |
| 终端 tab（shell 选择/重命名、恢复、主题跟随） | `packages/client/ui-sidebar-terminal/` | terminal-controller RPC；tab kind `terminal` | 关闭 tab 结束进程 |
| 文件 tab（目录树、导航栏、定位当前文件） | `packages/client/ui-sidebar-files/` | `remote.workspaceFiles.list`；tab kind `files` | 只列目录 |
| 文档预览 tab（Markdown/代码/图片/PDF/Office/HTML/文本） | `packages/client/ui-sidebar-documentpreview/` | `remote.workspaceFiles.read/readBytes`、`remote.officeToPdf`；tab kind `text` | 只读预览；Office 需 Host |
| 浏览器 tab（sandbox HTTP(S)、地址栏、前进后退） | `packages/client/ui-sidebar-browser/` | tab kind `browser`；Web iframe / Desktop webview | Web profile 默认禁用（`cordis.patch.yml` disabled 非 desktop） |
| 子代理目录、面包屑血缘导航、子级 transcript、@引用 | `packages/client/ui-subagent/` | `ctx.sessions.subagentCatalog`；`subagents/prompt`、`subagents/interruptByUser` | |
| Agent Team 开关与共享任务板 | `packages/client/ui-agent-team/` | `remote.agentTeams.setEnabled`；投影 `agentTeam` | 新会话默认关闭 |
| 插件面板（侧栏"插件"页） | `packages/client/ui-plugin-manager/` | `pluginInventory.list()` 判可用性 | 见 §6 |
| 自动化任务页（侧栏主面板 + 右栏 tab） | `packages/client/ui-schedule/` | `remote.schedule.*` | 见 §7 |
| Workflow 运行节点（阶段/成员折叠、打开子会话） | `packages/client/ui-workflow-run/` | 事件 `tool-workflow/run-start` 等；chat node slot | 仅顶层运行可导航 |
| 会话状态点与完成未读标记（侧栏行） | `packages/client/ui-session/`；`ui-workspace/tree.ts` | `api-session/status`；pending interaction precedence：plan-review > question > approval | 非主视图 running→idle 产生 `completionUnread`；侧栏状态点，非系统通知 |
| 连接状态指示（断线/重连/恢复药丸） | `packages/client/ui-settings-general/src/client/SettingsRoot.tsx`；`ui-primitives/src/ConnectionIndicator.tsx` | `ctx.connection.state/reconnect()` | 自动重连 500ms 起指数退避、10s 封顶；点击可手动重连 |
| 重连后的状态修复 | `packages/api/session-controller/src/client/transport.ts` | 新代际重拉 `session/list`、`session/control` baseline、`session/follow` 独立 resume | 增量不跨代际复用 |

## 2. 消息与流式

| 功能点 | 上游位置 | 依赖后端能力 | 备注 |
|---|---|---|---|
| 会话事件装配：历史前插、实时 append、assistant settlement | `packages/client/ui-conversation/src/client/conversation/assembly.ts`、`assembler.ts` | `session/follow` 首帧 snapshot+projection+assistant baseline；`session/event`、`agent/assistant-stream`；`session/page` 缺口修复 | event seq 与 assistant revision 均须连续，断档重开 follow/rebaseline |
| Chat 视图：用户消息、steering、本地回显对账 | `packages/client/ui-chat/src/client/conversation-nodes/message.ts`、`inbox.ts` | `user/message`、`agent/inbox/spliced`；投影 `inbox` | 回显在持久记录到达时原子替换 |
| Assistant 流式：文本/思考/工具参数 delta、定稿、中断冻结 | `packages/client/ui-chat/src/client/conversation-nodes/assistant.ts` | `assistant/live-chunk`（`text-delta/reasoning-delta/tool-call-delta/block-start/block-end/usage`）、`assistant/message`、`llm/retry` | retry 清空当前尝试；中断前缀无 messageId，无消息操作 |
| Markdown/GFM 渲染：表格、脚注、删除线、安全策略（禁 javascript:/data: 等） | `packages/client/ui-chat/src/client/chat/AssistantMarkdown.tsx`；`ui-primitives/src/markdown/*` | 无 RPC | 原始 HTML 不作为 markup；流式只重解析尾部 |
| 代码高亮（Shiki）、复制、换行、行号、流式 fence 增量 | `ui-primitives/src/markdown/CodeBlock.tsx`、`highlight.ts` | 无 RPC | 单行 1000 UTF-16 上限不高亮 |
| TeX 数学（KaTeX）、选区复制为 TeX | `ui-primitives/src/markdown/parse.ts`、`katex.tsx`、`copy-tex.ts` | 无 RPC | 流式期间公式保持字面 |
| 图片/文件附件呈现：用户附件、画廊、灯箱、悬停预览 | `ui-chat/src/client/chat/UserContent.tsx`；`ui-attachment/` | `session.readAttachment`；`ctx.uiConversation.imageUrl()` | Markdown 远程图仅 absolute HTTP(S) |
| 文件链接点击（行号 `#L24`/`:24-30`）、HTTP(S) 打开方式偏好 | `ui-chat/src/client/apply.ts`；`ui-primitives/src/markdown/file-link.ts` | `ctx.sidebarRight.openResource`；设置 `ui-chat.linkOpening` | 无 Browser 时外部打开 |
| 消息操作：复制、时间、fork、扩展动作 | `ui-chat/src/client/chat/MessageIconActions.tsx` | `ctx.sessions.fork`；`useAccess('session/fork')` | |
| 工作过程分组、思考预览、整轮折叠（简洁/标准/详细/完全展开） | `ui-chat/src/client/conversation-nodes/process-groups.ts`、`ReasoningRow.tsx` | 设置 `ui-chat.transcriptView/collapseTiming/foldedReplies` | |
| 历史分页、语义锚点、轮次轨道导航、回到底部 | `ui-chat/src/client/chat/ChatView.tsx`、`TurnNavigator.tsx` | `session/loadOlder/loadThrough`；投影 `turnOutline` | 普通页 ≥50 messages，上限 500 |
| Token/速度/缓存统计药丸、每轮用量面板 | `ui-chat/src/client/chat/StatsPills.tsx`、`TurnUsagePanel.tsx` | 投影 `sessionStats`、`tokenUsage` | |
| 命令回执、压缩（compaction）卡、重试行、错误行、配额提示 | `ui-chat/src/client/conversation-nodes/command.ts`、`compaction.ts`、`retry.ts`、`turn-error.ts`、`turn-max-tokens.ts` | `command/run/done`、`compaction/*`、`llm/retry*`、`turn/end` | QUOTA/ACCOUNT_QUOTA 实时提示 |
| 运行状态：鲸鱼动画、计时（断线暂停） | `ui-chat/src/client/chat/RunningStatus.tsx` | `ctx.connection.state`；Session `running` | |
| 文本选区/右键扩展承载 | `ui-chat/src/client/chat/text-selection.ts` | slot `conversation.chat.text-interaction` | |
| 输入编辑器：逐会话草稿、引用 chip、命令 chip、quote、恢复 | `packages/client/ui-conversation/src/client/input/*` | `inputTriggers`；scoped 事件 `slash/input-*`；localStorage `dsh.conversation` | 附件 File 对象不持久化 |
| 发送与回显：beginSubmission → prompt，Host receipt 确认后清空 | `ui-conversation/src/client/service.ts`、`input/machine.ts` | `session/prompt(content,mode,requestId)` | 失败保留草稿；同一时刻一条在途 |
| Queue/Steer：QueueDock 预览/编辑/删除/插话 | `ui-conversation/src/client/queue/QueueDock.tsx`、`submission-policy.ts` | `session/updateQueue`（`edit/remove/steer`）；`session/prompt(...,'steer')` | 繁忙 Enter 默认 Queue；空草稿加速 Enter 全部转 Steer |
| 停止当前 turn：按钮 + 双 Esc 快捷键 | `ui-conversation/src/client/stop-shortcut.ts`、`stop-sequence.ts` | `session/cancel` → `agent.cancel({kind:'user'},{keepInbox:true})` | 默认双 Esc 间隔 500ms；保留 Queue |
| 失败后"继续生成"（六码资格） | `ui-conversation/src/client/skeleton/InputBar.tsx`；`packages/api/session-controller/src/continuation.ts` | `session/prompt([], 'queue')` | RATE_LIMIT/SERVER/TIMEOUT/TRANSPORT/EMPTY_RESPONSE/STREAM_CLOSED |
| 附件上传：选择/粘贴/拖放、进度、失败重试 | `ui-conversation/src/client/service.ts`；`packages/client/file-upload/` | `ctx.fileUpload.upload/uploadImage`；`POST /api/session/uploadFileBinary`；`useAccess('fileUploads/upload')` | PNG/JPEG/WebP/GIF；receiptId 供后续 prompt 引用 |
| 上下文占用率圆环与组成详情 | `ui-conversation/src/client/skeleton/ContextMeter.tsx` | 投影 `contextPressure`、`contextBreakdown` | |
| Todo dock（只读投影） | `ui-conversation/src/client/skeleton/TodoPanel.tsx` | 投影 `todos` | 不是编辑器 |
| 已发送文本凭据遮挡（含眼睛开关） | `packages/client/ui-secret-mask/` | `ctx.uiConversation.userText.register()`；无 RPC | 覆盖各类 key/JWT/私钥；仅交互文本有眼睛 |
| Trajectory 视图：事件表、检查器、时间线（TTFT/吞吐/缩放）、搜索、虚拟行 | `packages/client/ui-trajectory/` | `tool/call/result`、`request/header`、`ctx.remote.toolResults.watch` | 需 `developerTools` 开关；实时工具结果仅活跃会话内存 |
| 消息赞踩与反馈弹窗（`/feedback`、会话菜单入口） | `packages/client/ui-message-feedback/` | `messageFeedback.put/delete`（`ifVersion` CAS）、`sessionFeedback.record` | 七类反馈 + 可选描述 |
| 视图选择/固定（Chat 恒定，Trajectory 随开发者工具） | `ui-conversation/src/client/view-selection.ts` | localStorage `dsh.conversation.<sessionId>` | |
| 会话图片共享缓存 | `ui-conversation/src/client/conversation/historical-images.ts` | `session.readAttachment` | Chat/Trajectory/Queue/Tool 共用 |

## 3. 工具与展示卡片

| 功能点 | 上游位置 | 依赖后端能力 | 备注 |
|---|---|---|---|
| 工具调用树（根调用 + PTC 子调用）、通用 fallback 卡 | `packages/client/ui-tool/src/client/tool/ToolCallTree.tsx` | `tool/call/result`、`tool/ptc-dispatch-start/dispatch` | 按 wire tool name 分发 `tool.call.toolview` |
| `read` 文件读取卡（行号窗口、打开定位） | `ui-tool/src/client/tool/toolviews/read-row.tsx` | 持久 `meta.path/lines/totalLines/lang` | |
| `read_image` 图片结果卡（图库、尺寸） | `toolviews/read-image-row.tsx` | `session.readAttachment` | |
| `write/edit/apply_patch` diff 卡（运行中预期 diff → 完成 meta.diffs） | `toolviews/file-mutation-row.tsx` | 结果 `meta.diffs/path/stopped` | apply_patch 不虚构运行中 diff |
| Bash/Pwsh 终端卡（命令、cwd、输出、退出码/信号） | `toolviews/bash-sample.tsx` | 结果文本 `[exit code: …]` | 持久 shell/后台走通用卡 |
| `todo_write` 摘要与 Todo diff | `toolviews/todo-row.tsx` | `todo/write` | 基线由持久事件建立 |
| `grep/glob` 搜索卡 | `toolviews/search-row.tsx` | 结果 `meta.shape/files/total/truncated` | |
| `web_search/web_fetch` 卡（回答、来源、URL、status） | `toolviews/web-row.tsx` | 结果 `meta.answer/sources/statusCode` | fetch 链接恒新标签打开 |
| `ask_user_question` 工具行（重开面板、迟到回答只读） | `toolviews/ask-question-row.tsx` | 投影 `userQuestions` | |
| 控制回执与结构化详情卡（goal/schedule/subagent/job/LSP 等） | `toolviews/details-row.tsx` | 已记录 arguments/result | |
| Auto-review 拒绝优先呈现 | `ui-tool/src/client/tool/models/auto-review-denial.ts` | ToolCallBlock meta | 隐藏原始参数 |
| 交付卡：改动文件卡 + review diff tab、交付文件卡、收尾文件引用 | `packages/client/ui-deliverables/` | `ctx.workspaceChanges.summary/diff`；`workspace-files.stat`；事件由 `packages/host/workspace-changes`（git 工作树快照）记录 | Host 重启后无快照不显示 |
| Cordis 动态插件定义卡（run/stop 开关） | `packages/extensions/ui-cordis/`（注意：不在 `packages/client/`） | `cordis_define` 工具行 | |
| 通用 Markdown/TeX/终端/diff/读取/搜索输出渲染器 | `packages/client/ui-primitives/` | 无 RPC | 共享原子控件/图标库，供所有卡片复用 |
| 文件动作：下载/另存、图片复制为 PNG、右键菜单 | `packages/client/ui-file-actions/` | `remote.workspaceFiles.readBytes/stat`、`session.openWorkspacePath` | saveAs 仅 Desktop |
| 拖放字节传递（`ctx.fileDrag` prepare/read） | `packages/client/ui-file-drag/` | shell.overlay 拖放遮罩 | 浏览器令牌不跨标签 |
| 在应用中打开（会话标题栏拆分按钮、文档预览打开单文件） | `packages/client/ui-open-in-app/` | `session/openWorkspacePath`、`session.workspacePathApplications` | Host 解析默认应用；空目录时隐藏 |
| 右键菜单框架（目标保留、异步 resolve、超时） | `packages/client/ui-context-menu/` | `ctx.contextMenus` | 默认超时 1000ms |

## 4. 审批与提问

| 功能点 | 上游位置 | 依赖后端能力 | 备注 |
|---|---|---|---|
| 工具审批卡：接管 composer、允许/拒绝（`allowed-once`/`rejected`） | `packages/client/ui-approval/src/client/index.ts`、`ApprovalPanel.tsx`、`ApprovalCard.tsx` | waterfall `approval/request`；Host `ctx.approval.request` 写 `approval/asked/decided` | 通用工具审批**无 always-allow**；一次性回答锁；详情显示理由与关联命令 |
| 审批快捷键：详情聚焦后 Enter 允许 / Esc 拒绝 | `ui-approval/src/client/index.ts` | `ctx.shortcuts.registerFixed('approval.allow'/'approval.reject')` | |
| 审批风险说明浮层（非模态） | `ui-approval/src/client/ApprovalWarning.tsx` | 无 RPC | Esc/失焦只关浮层不拒绝 |
| 用户提问卡：单选/多选/自定义答案、翻页、跳过 | `packages/client/ui-user-questions/src/client/QuestionComposer.tsx` | waterfall `user-questions/request`；`remote.userQuestions.answer` | 一次一个请求占据 composer |
| 提问倒计时、聚焦暂停、超时后继续作答 | `ui-user-questions/src/client/index.ts` | `remote.userQuestions.attachWait(sessionId, callId)`；rejection `ASK_TIMED_OUT` | 到零放行 Agent 但卡片仍可答 |
| 迟到回答、草稿恢复、关闭与重开 | `ui-user-questions/src/client/index.ts`、`draft-store.ts` | `userQuestions`/`inbox` 投影；草稿存 `dsh.user-questions.drafts.v1` | 有 callId 关闭仅隐藏可重开；无 callId 关闭为 `ASK_CANCELLED` |
| 已回答气泡（右对齐）与只读回看 | `ui-user-questions/src/client/question-reply.ts`、`QuestionReplyView.tsx` | `tool/call/result`；late `user/message` `source.kind:'user-question-reply'` | |
| Plan review 专用卡（同意执行 / 要求修改） | `ui-user-questions/src/client/PlanReviewPanel.tsx`；`packages/plan/plan-mode/src/index.ts` | `exit_plan_mode` → `ctx.userQuestions.ask`，`intent:{kind:'plan-review'}` | **不是 approval/request**；"要求修改"以 `ASK_CANCELLED` 返回且 plan mode 保持 |
| Plan 模式徽章与退出（`/plan off`） | `packages/client/ui-plan/src/client/PlanModeControl.tsx` | 投影 `plan`；`ctx.remote.commands.execute(sessionId,'/plan off',[])` | 等待 Host 投影确认，不乐观撤下 |
| 已提交计划卡与右侧完整计划预览 | `ui-plan/src/client/plan-definition.ts`、`PlanPreview.tsx` | `exit_plan_mode` 的 `arguments.plan`；`ctx.remote.session.events/page` | 每浏览器对待审计划自动打开一次 |

## 5. 设置与模型

| 功能点 | 上游位置 | 依赖后端能力 | 备注 |
|---|---|---|---|
| 设置外壳（800×800 面板、移动端横向导航、分区滚动）、配置表单底座（原子 mutate、revision 冲突保留草稿） | `packages/client/ui-settings/`、`ui-settings-general/` | `ctx.configForms.get/set/unset/mutate`；Remote `settings/describe/mutate`；`settings/document-updated` | `Mod+,` 打开；写入串行、redactSecrets |
| 语言选择（8 语言、即时切换） | `packages/client/locale/src/client/LanguageRow.tsx` | `ctx.locale.setLocale()`；settings `locale` | 无偏好时按浏览器语言回退英文 |
| 代码工作工具开关（`ui-settings.enabled`） | `packages/client/ui-settings/` | `ctx.configForms.developerTools` | 控制 Trajectory 显隐、HTML 预览等 |
| 连接恢复指示、服务版本变化对话框、当前版本行 | `ui-settings-general/` | `ctx.connection.*`；`ctx.modules.entries.releaseChange/dismissRelease()` | |
| 打开配置文件（Host 原生编辑器，loopback only） | `ui-settings-general/`；`packages/api/settings-controller/` | `settings/openSettingsDocument` | 底层为 `ctx.configEditor`（原子落盘、回滚） |
| Models 页：提供商行、编辑卡、排序（拖拽/键盘）、新增/删除提供商 | `packages/client/ui-settings-models/`（`ModelsSection.tsx`、`ModelListEditor.tsx`） | `llm/listProviders`、`session/modelCatalog`、`settings/mutate`、`llm/discoverModels` | 一次一张编辑卡 |
| API Key：只写输入、格式校验、保存到凭据、不回读 | `ui-settings-models/src/client/ApiKeyInput.tsx`、`operations.ts` | `credentials.set/unset/describe` | `credentials.describe` 只返回元数据（configured/writable），**不能读回 secret**；无独立"删除 API Key"按钮，清除仅随提供商删除 |
| 首次运行 DeepSeek 凭据引导 + Web 版本化说明对话框 | `ui-settings-models/src/client/DeepSeekOnboardingDialog.tsx`、`WelcomeNotice.tsx` | `settings.mutate`/`credentials.set`；readiness 共享快照 | 仅无可用 provider 时显示 |
| 账号页：DeepSeek 登录/退出、资料、余额、充值、Platform 视图 | `packages/client/ui-settings-account/` | `account/watch`、`account.startSignIn/cancelSignIn/signOut`、`account.getProfile/getBalance`；`shell.quota-notice` | 凭据失效 toast（`deepseek-account/session-expired`）；登录后 `session.initializeDefaultModel()` |
| Agent 循环设置（并行工具调用上限） | `packages/client/ui-settings-agent-loop/` | `ctx.configForms.get('agent-loop')` | |
| 子代理设置（递归深度/并行容量/模型选择） | `packages/client/ui-settings-subagent/` | `configForms.get('subagent'/'subagent-model-selection')`、`session.modelCatalog()` | |
| Web 搜索与获取设置（provider 下拉、结果数） | `packages/client/ui-settings-web-search/` | `remote.web.listProviders`；`web-search-deepseek` 等 namespace | |
| 终端设置（命令超时、输出上限） | `packages/client/ui-settings-shell/` | `configForms.get('bash-sandbox'/'pwsh-sandbox')` | |
| 会话日志上传开关 | `packages/client/ui-settings-session-log/` | `session-log-deepseek` namespace | |
| 权限预设：默认权限行 + `/permission` 会话选择器 | `packages/client/ui-permission-presets/` | `permission` 描述符；Session `permissions` 投影 | 设置行只影响未来会话 |
| Agent Preset 卡（内置/自定义、默认值、YAML 查看） | `packages/client/ui-agent-preset/` | `agentPresets/list/read`；`agent-preset-registry` settings | Web 不编辑 preset |
| Goal 条带（显示/编辑/暂停/恢复/清除） | `packages/client/ui-goal/` | `useProjection('goal')`、`ctx.remote.goals.*`、`goal/activation-changed` | |
| 会话模型切换：`/model` 弹窗 + composer 模型位 | `packages/client/ui-model-selection/` | `session/modelCatalog`、`session/selectModel`；`llm/adapters-updated`、`llm/models-updated` | 完整选择下一次请求生效，当前 step 保留旧配置；同时保存默认 |
| 主题/字号/字体设置与 token 投影 | `packages/client/ui-theme/` | `ctx.settings`；`ctx.theme`；Desktop 字号快捷键 | 持久化到 Host settings `ui-theme` |
| 快捷键速查与录制编辑弹窗（`Mod+/`） | `packages/client/ui-shortcuts/` | `ctx.shortcuts.catalog/config/edit()` | 固定操作只读；键位持久化于 localStorage（Web）/ userData（Desktop） |

## 6. 插件与凭据

| 功能点 | 上游位置 | 依赖后端能力 | 备注 |
|---|---|---|---|
| 插件面板：列表（内置配置/实验性/已安装）、卡片状态诊断、手动刷新 | `packages/client/ui-plugin-manager/src/client/PluginManagerPage.tsx`、`manager-store.ts` | `pluginManager.listBundles/listPlugins`；`pluginInventory.list()`；`plugin-manager/changed`；`connection/reset` | 页面属于 profile；无受管 profile 时报告不可用 |
| Bundle/组件行启停（`applied`/`restart-required`/`overridden`） | `ui-plugin-manager/src/client/manager-store.ts` | `pluginManager.setBundleEnabled/setPluginEnabled` | 禁 HMR 时插件启停需要重启才能生效（`no-hmr`） |
| 插件配置表单（官方/Bundle/组件行） | `ui-plugin-manager/src/client/PluginManagerPage.tsx` | `ctx.configForms`；slots `plugins.bundle.config` 等 | |
| Bundle 详情文档（npm/Git README，错误分类与重试） | `ui-plugin-manager/src/client/detail-store.ts` | `pluginManager.readBundleDetail()`；`pluginRegistryProbe.fastest()` | 图片经 `api/plugin-manager/detail-file` |
| 添加插件：npm/Git/压缩包/本地路径、inspect 预检 | `ui-plugin-manager/src/client/manager-store.ts` | `pluginManager.inspect/installBundle` | 安装以用户权限运行；安全提示明示 |
| 安装进度对话框（pnpm 输出、取消、丢响应恢复、GitHub 镜像建议、构建脚本审批） | `ui-plugin-manager/src/client/manager-store.ts`、`PluginManagerPage.tsx` | `cancelInstall/waitForInstall`；`plugin-manager/install-state/install-log`；`{approvedBuilds}` | 一次一个安装任务 |
| 卸载确认、官方按需包更新、版本检查与更新对话框 | `ui-plugin-manager/src/client/versions.tsx`、`manager-store.ts` | `pluginManager.removeBundle/listVersions/updateBundle` | 更新可能先删后装并报告重启 |
| 安装源选择/记忆/自动探测（npm 当前源、镜像、自定义） | `ui-plugin-manager/src/client/manager-store.ts` | `pluginManager.registries()`；Host fetch ping | localStorage 记忆 |
| 插件市场"发现"标签：索引源管理、搜索/分类/标签/排序/分页、兼容性、安装确认 | `packages/client/ui-plugin-market/`（`market-store.ts`、`DiscoverSection.tsx`） | `pluginMarket.listIndexes/setIndexes/search/readEntryDetail` | 需用户配置第三方 HTTP(S) 索引 |
| 设置→内置插件：标签壳 + 只读 Host Loader 清单 | `packages/client/ui-settings-plugins/`、`ui-settings-plugin-inventory/` | `ctx.remote.pluginInventory.list()`；`ctx.modules.entries.retry()` | 清单懒加载只读 |
| 凭据 describe/set/unset（脱敏元数据，不回读 secret） | `packages/api/settings-controller/src/credentials.ts`；`packages/credentials/credentials/` | Host `ctx.credentials.resolve/describe/set/unset` + record 半部（OAuth grant 等） | 完整 CredentialProvider seam 含 update events；正确实现可热轮换 |
| 入口定制：分区"更多"菜单、Pin 插件入口、插件草稿 | `packages/client/ui-entry-customization/` | `ctx.uiWorkspace.startSession`；`pluginNavigation` | 创建仅追加草稿不发送 |

## 7. 其他外围

| 功能点 | 上游位置 | 依赖后端能力 | 备注 |
|---|---|---|---|
| 启动页（spinner、逐 entry 失败报告、必需插件审计） | `packages/client/web/src/boot-page.ts`、`boot-client.ts` | `window.__DSH_BOOT__`；`internal/status` | 可选插件失败仅控制台警告 |
| 浏览器启动认证（token 换签名 Cookie）、信任检查 | `packages/client/connection/src/browser-auth.ts`、`api-request-trust.ts` | `GET /?token=…`；`ctx.connection.admit()` | Cookie 30 天 HttpOnly；跨站拒绝 |
| 客户端插件模块系统、bundle 路由（combo/chunk/资源）、页面同步失败重试 | `packages/client/modules/` | `ctx.clientModules`/`ctx.modules`；`/plugins/...` | 插件启停免刷新；release 变化暂停图更新 |
| HMR（开发重建、SSE `/plugins/events`） | `packages/client/hmr/` | `ctx.clientModules.onRebuilt()`；`ctx.modules.entries.reload()` | Desktop 不走此 SSE |
| RPC/WebSocket 传输、精确 Fetch 路由注册 | `packages/client/connection/` | `ctx.connection.rpc`；`POST /api/*`；WS `/api/remote.mux`；`$events` waterfall | 一元 JSON + multipart 二进制 |
| 文件上传服务（Worker、流式、receiptId） | `packages/client/file-upload/` | `ctx.fileUpload`；`ctx.attachments.saveFileStream` | |
| 快捷键引擎（注册、DOM 分发、输入保护、冲突检测、持久化） | `packages/client/shortcuts/` | `ctx.shortcuts.register/registerFixed` | 按 desktop/web × 平台声明默认键 |
| 本地化（8 语言词典、fallback 链、运行时注册） | `packages/client/locale/` | `ctx.locale` | |
| 可观测/快照 store 原语、`useResource` 资源模型 | `packages/client/store/`、`packages/client/resources/` | `ctx.resources.register/pin` | 右栏 tab 钉住资源流 |
| React 绑定渲染器、Slot 注册表核心 | `packages/client/ui-renderer/`、`ui-slots/` | `ctx.uiRenderer.mount`；slot 生命周期 | 纯基础设施 |
| 产品埋点（Desktop only，普通 Web 不上报） | `packages/client/product-analytics/` | `ctx.remote.productAnalytics` | 提示词/回复/key 不进事件字段 |
| 会话后台 jobs（头部列表、输出面板、双击停止） | `packages/client/ui-jobs/` | `job/list/follow/kill`（`ctx.jobs`） | 清空只是本浏览器隐藏 |
| 自动化计划任务（创建/编辑/立即运行/暂停/删除、投递历史、时钟） | `packages/client/ui-schedule/` | `remote.schedule.*`；Host `ctx.schedule` | 独立于 jobs 的持久调度 |
| PWA 元数据、favicon、子路径挂载 | `apps/web/index.html`、`public/manifest.webmanifest` | 无 RPC | |
| 系统通知与提示音 | （搜索 `apps/web`、`packages/client`） | — | **未证实 Web 存在会话完成系统通知或声音**；Electron 仅更新提醒 `Notification({silent:true})`（`apps/desktop/src/update-attention.ts`） |
| 测试支撑包 | `packages/client/test-runtime/`（见 `packages/test-support/client-runtime/`） | — | 非 UI |

## 使用说明（如何消费本清单）

1. **驱动 `docs/web-ui-map.md` 与 GUI 实现顺序（阶段 3）**：以本清单为唯一输入，逐区把"功能点"映射到 libadwaita 控件；每张展示卡片先登记其"依赖后端能力"列，缺的 bridge 能力在阶段 2/6 补齐并加集成测试。
2. **实现顺序建议**：先做 §1 会话与导航骨架（会话列表/创建/恢复/提交/流式），再 §2 消息渲染与交互，再 §4 审批与提问，随后 §5 设置与模型，§6 插件与凭据，最后 §3 展示卡片与 §7 外围——与实施计划阶段 4→6 一致。
3. **最终验收（对照 DSH web 逐行走查）**：对每行标注 已实现 / 缺失 / 有意不移植（需在验收记录中写明理由，如依赖浏览器的 iframe、PWA、Electron 窗口 chrome 等在 GTK 下由原生等价物替代）。"功能与 DSH web 无异"指：每行用户可见行为等价，允许实现载体不同；标注 未证实 的行在验收前需先在上游复验。
4. **上游升级**：重新克隆上游、比对本清单行级差异（新增/移除/改名的能力），同步更新本文档后再更新映射表与 bridge 契约。

## 已知偏差与未证实项（供 bridge 设计复核）

- 上游无 `session/close` RPC；"关闭会话"仅是释放视图引用。销毁 live Agent、归档、删日志是三种不同动作。
- 未证实存在修改既有会话 cwd 的通用 API；切换 Workspace 实为打开新会话。
- Web 事件词汇是 `user-questions/request` / `approval/request` waterfall（Cordis），SDK wire 的 `user-question/ask` 是另一层；bridge 若沿用 SDK 词汇需自行映射，并处理 late answer 与 plan review。
- Plan review 走 `ctx.userQuestions.ask`，**不是** `ctx.approval.request`。
- 系统通知/提示音上游 Web 没有，属下游新增能力。
- `credentials/describe` 不返回 secret；GUI 侧 Secret Service 注入是下游新协议。
- `agent/assistant-stream` 源码未见单 consumer 限制（多 listener 已存在），但"第二远端 consumer 端到端可用"仍需 spike 验证；且可靠流需要 opening snapshot + revision 连续 + gap repair，不能只广播两条事件流。
- 禁用 HMR 后，插件启停保存结果为 `restart-required`，GUI 必须提供重启流程。
- settings / plugin-manager / config-editor 均依赖受管 profile（`profileContext`），base-only 组合会禁用。
