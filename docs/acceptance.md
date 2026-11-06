# dsh-libadwaita 对 DSH Web 功能验收对照

日期：2026-11-06  
基线：`docs/dsh-web-feature-inventory.md` 与 `docs/web-ui-map.md` 的 N/M/C/A/S/P/O 编号，共 132 行。  
实现基线：bridge `33b486a`、app `d25d2ea`；bridge 39 项测试、app 65 项测试均通过。  
判定只依据当前仓库代码接线与可执行证据，不依据计划、映射表预期或上游 Web 的理论能力。

## 0. 验收口径

- `已实现`：当前代码具备对应功能的主要用户语义，并有仓库内可定位接线；不表示与上游逐像素一致。
- `部分实现`：已有真实接线，但存在明确且可复现的缺口；证据与缺口同时列出。
- `缺失`：未找到有效用户接线。
- `有意不移植`：设计文档明确排除的浏览器、React、PWA、遥测等机制。
- `有意简化`：设计文档明确接受的降级，例如 TeX 保留源码而不引入 KaTeX/WebView。

测试证据：

- `cargo test --manifest-path app/Cargo.toml`：65 passed。
- `npm test --prefix bridge`：39 passed。
- `app/tests/supervisor_dsh_tests.rs::test_real_dsh_lifecycle_and_session_create`：真实 dsh 启动、握手、会话创建、停止与重启。
- `bridge/tests/integration/gap-batch-d.spec.ts`：真实 dsh terminal 与 schedule 生命周期。

## 1. 状态汇总

| 区域 | 已实现 | 部分实现 | 缺失 | 有意不移植 | 有意简化 | 合计 |
|---|---:|---:|---:|---:|---:|---:|
| N 会话与导航 | 9 | 14 | 4 | 1 | 0 | 28 |
| M 消息与流式 | 8 | 12 | 7 | 0 | 1 | 28 |
| C 工具与展示卡片 | 1 | 11 | 6 | 0 | 0 | 18 |
| A 审批与提问 | 0 | 6 | 4 | 0 | 0 | 10 |
| S 设置与模型 | 0 | 13 | 7 | 0 | 0 | 20 |
| P 插件与凭据 | 1 | 6 | 5 | 0 | 0 | 12 |
| O 其他外围 | 4 | 3 | 2 | 7 | 0 | 16 |
| **总计** | **23** | **65** | **35** | **8** | **1** | **132** |

## 2. N：会话与导航

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| N01 | 三栏 AppFrame 与 shell 座 | 部分实现 | `app/src/ui/window.rs::MainWindow::new` 使用 `AdwOverlaySplitView`、`GtkPaned`、`RightDock`、`AdwToolbarView` 与 `AdwToastOverlay`；缺 56px 折叠轨、宽度优先级与窄屏抽屉规则。 |
| N02 | 左侧栏品牌/New Session/面板/设置/折叠轨道 | 部分实现 | `app/src/ui/sidebar.rs::Sidebar::new` 提供品牌、New Session、Workspace/Subagent/Session 区、搜索、Plugins/Schedules/Account/Settings；`win.toggle-sidebar` 可折叠，但无独立轨道视图。 |
| N03 | Workspace/Session 列表与排序/归档过滤 | 部分实现 | `Sidebar::load_workspaces` 按 workspace 分组 session，`Sidebar::load_sessions` 渲染会话行，`SessionController::archive_session` 调 `session/archive`；缺 Pin、手动排序、归档筛选与取消归档。 |
| N04 | 新建会话/复用 blank/命名创建 | 部分实现 | `SessionController::prompt_new_session_portal` 使用 `GtkFileDialog::select_folder`，`create_session` 调 `session/create` 后 follow；缺同 cwd/preset blank 复用、命名创建与 agentPreset 选择。 |
| N05 | 打开并恢复既有会话 | 已实现 | `MainWindow::set_controller` 的 `win.select-session` → `SessionController::follow_session` → `UiMessage::SessionFollowed`；`ConversationView::update_transcript` 渲染 follow 快照。 |
| N06 | 启动时恢复当前 Session/Workspace | 已实现 | `app/src/session_state.rs::SessionState` 持久化 last session/cwd；`SessionController::handle_session_list_loaded` 在连接后恢复，`persist_session` 在 follow 后保存。 |
| N07 | 官方品牌与首页鲸鱼层 | 缺失 | 仅有 `Sidebar::new` 的 `dsh` 文本与 `ConversationView::new` 的 welcome card；无官方图标、Canvas 或简化鲸鱼动画。 |
| N08 | “关闭”当前会话视图 | 已实现 | `win.close-session` → `SessionController::close_session` → `libadwaita/session/release`；`UiMessage::SessionReleased` → `clear_active_session_ui`，不停止 Agent。 |
| N09 | 归档/取消归档，活动会话二次确认 | 部分实现 | `Sidebar::load_sessions` 提供 Archive 菜单，`archive_session` 调 `session/archive(archived:true)`；缺活动会话确认、取消归档与归档筛选。 |
| N10 | 手动重命名 | 已实现 | `win.rename-session` → `open_rename_for_id` → `rename_session` 调 `libadwaita/session/rename`；`UiMessage::SessionRenamed` 更新活动会话、标题与缓存。 |
| N11 | 自动标题 | 部分实现 | `setup_ui_receiver` 处理 `IncomingNotification::SessionTitle`，更新 `AdwWindowTitle` 与活动会话；侧栏行只在列表重载时更新，不即时刷新。 |
| N12 | Fork 会话 | 已实现 | 消息动作 `win.fork-session` → `open_fork_dialog` → `fork_session` 调 `libadwaita/session/fork({sessionId,atSeq,increaseTitle:true})`；`UiMessage::SessionForked` follow 新会话。 |
| N13 | 会话元数据/内容搜索 | 缺失 | `Sidebar::new` 有 `GtkSearchEntry`，但未连接 search-changed；`libadwaita/session/search` 虽已由 bridge 提供，GUI 无调用。 |
| N14 | 行菜单、Pin、Archive、Toast | 部分实现 | `Sidebar::load_sessions` 的 `GtkPopover` 提供 Rename/Close View/Archive，`win.*` action 与 `AdwToast` 已接线；缺 Pin、本地排序与菜单快捷键。 |
| N15 | Workspace 创建/层级/重命名/删除与目录选择 | 部分实现 | `prompt_new_workspace` 选目录，`create_workspace`/`switch_workspace`/`rename_workspace`/`remove_workspace` 使用 `libadwaita/workspace/*`；切换会 release 旧视图并创建新 cwd 会话，但不迁移 composer 草稿。 |
| N16 | 右停靠栏框架 | 部分实现 | `RightDock::new` 使用 `AdwTabView`/`AdwTabBar` 固定 Terminal/Files/Preview；缺分屏、浮动、全屏与每会话 tab 状态持久化。 |
| N17 | 终端 tab | 部分实现 | `app/src/ui/terminal_panel.rs::TerminalPanel` 使用只读 `GtkTextView` + `GtkEntry`，`restore` 调 `terminal/list`，无终端时 `create_terminal`，输入调 `terminal/input`，可见时 500ms `terminal/output` 轮询并有 5000 行缓冲，Close 调 `terminal/close`；无 VTE、shell 选择、重命名与 resize。 |
| N18 | 文件 tab | 已实现 | `app/src/ui/files_panel.rs::FilesPanel` 使用 `libadwaita/files/list` 列目录、`files/stat` 显示状态，目录行调用 `navigate_to`，文件行调用 `preview_file`；根路径由 `session/workdir/get` 提供。 |
| N19 | 文档预览 tab | 部分实现 | `PreviewPanel::show_text` 使用 `sourceview5::View` 与语言映射，`show_image` 使用 `GtkPicture`，`show_unavailable` 显示失败；缺 PDF、Office、HTML、行定位与渲染/源码切换。 |
| N20 | 浏览器 tab | 有意不移植 | 项目禁止内嵌 WebView；`app/src/ui/desktop.rs::open_path_in_default_app` 使用系统默认应用作为原生替代。 |
| N21 | 子代理目录/血缘/transcript/@引用 | 部分实现 | `Sidebar::load_subagents` 从 `subagent/list` 渲染子代理，`win.open-child` → `open_child_session` 调 follow 并设置面包屑，`win.interrupt-subagent` → `interrupt_subagent`；无子代理续接 prompt、深层血缘树与专用 @引用候选。 |
| N22 | Agent Team 开关与共享任务板 | 缺失 | bridge 有 `agent-team/get|set-enabled`，但 base-only 探针返回 `-32040`；GUI 无 Team 开关或任务板。 |
| N23 | 插件面板入口 | 已实现 | `Sidebar::new` 的 Plugins 行触发 `win.show-plugins`，`MainWindow::new` 切换 `ConversationView::select_panel("plugins")`；完整管理位于 Settings→Plugins。 |
| N24 | 自动化任务页入口 | 已实现 | `win.show-schedules` 切换 `ConversationView` 的 schedules page；`app/src/ui/schedules.rs::SchedulesPage` 已接入真实 catalog。 |
| N25 | Workflow 运行节点与子会话导航 | 缺失 | transcript 对未知 workflow 事件仅进入 `RenderBlock::GenericEvent`；无阶段节点、成员导航或专用折叠面板。 |
| N26 | 会话状态点/完成未读 | 部分实现 | `SessionController::running_sessions`、`handle_status_notification` 与桌面通知记录运行/完成；侧栏无状态点、未读样式或待处理交互仲裁。 |
| N27 | 连接状态与手动重连 | 已实现 | `show_startup_error`/`show_connection_lost` 使用 `AdwBanner` 与 Retry，`bootstrap_supervisor`/`restart_supervisor` 冷启动并重新 initialize；无独立状态 pill。 |
| N28 | 重连状态修复 | 部分实现 | `app/src/supervisor.rs::GenerationTracker::record_generation` 跟踪 generationId，`UiMessage::SupervisorRestarted` 后 follow 活动会话；无自动退避重连、同代际 page 修复与全部打开会话 rebaseline。 |

## 3. M：消息与流式

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| M01 | 事件装配/历史前插/settlement | 部分实现 | `app/src/transcript.rs::ConversationTranscript::{apply_baseline,apply_stream_frame,apply_event,prepend_events}` 支持 baseline、live 替换、committed/abandoned、seq 去重与历史前插；检测到 gap 后只返回 `GapDetected`，无自动修复。 |
| M02 | Chat 用户消息/steering/回显 | 已实现 | `SessionController::submit_text` 根据运行状态选择 prompt/steer，`ConversationTranscript::push_pending_user` 显示 pending，`user/message` 到达后原子替换，失败由 `UiMessage::EchoFailed` 恢复。 |
| M03 | Assistant 文本/思考/工具 delta、定稿/中断 | 已实现 | `ConversationTranscript::apply_stream_frame` 聚合 text/reasoning/tool delta，`assistant/message` 与 committed 替换 live，abandoned 移除；`app/tests/transcript_tests.rs` 覆盖正常、废弃与 reasoning。 |
| M04 | Markdown/GFM/表格/脚注/安全链接 | 部分实现 | `app/src/ui/message_renderer.rs::render_markdown` 支持段落、标题、列表、引用、粗体/斜体/删除线、代码与链接；无表格、脚注、链接 scheme 白名单与尾部增量解析。 |
| M05 | 代码高亮/复制/换行/行号/流式 fence | 部分实现 | `build_code_block` 使用 `sourceview5::View`、语言识别、行号与复制按钮，live handle 每次更新正文；无换行开关、未闭合 fence 专用策略与超长行降级。 |
| M06 | TeX 数学与复制 | 有意简化 | 按 `docs/web-ui-map.md` §10.3 不引入 KaTeX/WebView；TeX 作为普通文本显示，不声称完成数学排版。 |
| M07 | 图片/文件附件、画廊、灯箱 | 缺失 | bridge 已提供 `attachment/upload|read` 与 prompt image，但 `Composer::new` 的 attach 按钮无 handler，消息渲染器无图片画廊或灯箱。 |
| M08 | 文件链接行号/HTTP 打开偏好 | 缺失 | Markdown 链接只生成 `<a>`，无统一激活处理；未解析 `#L`/`:line` 并联动 Files/Preview。 |
| M09 | 消息复制/时间/fork/扩展动作 | 部分实现 | 已定稿 assistant 消息有 Fork 按钮，用户消息在检测到 secret 时显示复制原文按钮；无统一复制全文、时间戳显示与扩展动作菜单。 |
| M10 | 工作过程分组/思考预览/整轮折叠 | 部分实现 | `build_assistant_message` 提供 reasoning `GtkExpander`，`cards.rs::build_card_shell` 提供展开/折叠；无四档过程分组、整轮折叠策略与失败轮例外。 |
| M11 | 历史分页/锚点/轮次导航/回到底部 | 部分实现 | `ConversationView::new` 监听 transcript adjustment 上沿，`SessionController::load_older_history` 调 `session/page`，`UiMessage::HistoryPageLoaded` 前插并按距底部偏移恢复锚点，`set_history_loading` 显示加载行；无 turn outline、轮次导航与回到底部按钮。 |
| M12 | Token/速度/缓存统计与每轮面板 | 缺失 | `StreamChunk::Usage` 在 transcript 中被忽略，无 tokenUsage/sessionStats 投影、统计胶囊或每轮面板。 |
| M13 | 命令/压缩/重试/错误/配额卡 | 部分实现 | `execute_command` 显示命令回执，`apply_event` 对未识别事件生成 `GenericEvent`，`SessionModel::handle_event` 记录 error code 供 continuation 使用；无压缩、重试、错误与配额专用卡。 |
| M14 | 运行状态/计时/断线暂停 | 部分实现 | `Composer::set_is_running`、`ConversationView::set_status` 与 `set_disconnected` 显示 Running/Idle/Disconnected，`turn_start_times` 用于通知时长；无逐秒计时器和断线期间计时暂停显示。 |
| M15 | 文本选区/右键扩展 | 部分实现 | Markdown label 与代码视图允许选择；无消息右键、Quote 插入与扩展动作。 |
| M16 | 草稿/引用 chip/命令 chip/quote/恢复 | 部分实现 | `trigger_popup` 与 `insert_composer_text` 支持 `/`、`@`、`#` 候选插入，`app/src/drafts.rs::DraftStore` 提供持久化原语；controller 未接入 DraftStore，切换会话不保存/恢复草稿，无结构化 chip/quote。 |
| M17 | 发送与 Host 回显确认 | 已实现 | `submit_text` 先 `push_pending_user`，RPC 成功后 `EchoAck` 清空 composer，持久 `user/message` 原子替换 pending，失败 `EchoFailed` 清 pending、恢复草稿并 Toast。 |
| M18 | Queue/Steer 预览/编辑/删除/插话 | 已实现 | `app/src/ui/queue_dock.rs::QueueDock` 展示 inbox，`SessionController::{refresh_queue,queue_edit,queue_remove,queue_steer_item,queue_steer_all}` 调 queue RPC；`IncomingNotification::SessionInbox` 实时更新。 |
| M19 | Stop 按钮与双 Esc | 已实现 | `setup_composer` 在 running 时把 send button 转为 cancel，`setup_double_esc` 在 500ms 内两次 Escape 调 `cancel_current_turn`；无终端/IME/模态焦点仲裁。 |
| M20 | 失败后“继续生成” | 已实现 | `SessionModel::handle_event` 记录六类 error code，`update_continue_state` 使用 `drafts::continuation_eligibility`，Continue 调 `continue_generation` 发送空 contentBlocks、`mode:"queue"`。 |
| M21 | 附件选择/粘贴/拖放/上传进度/重试 | 缺失 | attach 按钮无 handler，无 `GtkDropTarget`、剪贴板图片、上传进度或失败重试。 |
| M22 | 上下文占用率与组成 | 缺失 | 无 contextPressure/contextBreakdown 投影消费，无圆环或明细面板。 |
| M23 | Todo dock | 已实现 | `app/src/ui/todo_dock.rs::TodoDock` 位于 composer 上方，`ConversationView::update_todo` 由 follow projections 与 `todo/write` 事件更新，显示完成计数与只读状态。 |
| M24 | 已发送凭据遮挡与眼睛 | 已实现 | `build_user_message` 调 `drafts::mask_secret_patterns`，仅当文本被改变时显示 reveal 按钮；复制按钮复制原文，composer 原文不遮挡。 |
| M25 | Trajectory 表/检查器/时间线/搜索/虚拟行 | 缺失 | `ConversationView::new` 的 Trajectory 页仍是 `AdwStatusPage` 占位；无事件表、检查器、时间线、搜索或 `tool-result/watch` 订阅。 |
| M26 | 消息赞踩/反馈弹窗 | 部分实现 | `message_renderer` 为定稿 assistant 消息提供 thumbs up/down 与 Feedback 对话框，调用 `message-feedback/list|put|delete` 和 `session-feedback/record`；以 seq 派生 message id，无七类原因、冲突权威项对账与撤回状态刷新。 |
| M27 | Chat/Trajectory 视图选择与固定 | 部分实现 | `AdwViewStack`/`AdwViewSwitcher` 可切换 Chat/Trajectory；选择未按会话持久化，Trajectory 内容缺失。 |
| M28 | Session 图片共享缓存 | 缺失 | 无 attachmentId 授权读取、按会话缓存或 release 清理。 |

## 4. C：工具与展示卡片

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| C01 | 工具调用树/PTC/通用 fallback | 部分实现 | `ConversationTranscript::apply_stream_frame` 聚合 tool-call delta，`render_tool_result_card` 路由专用卡并回退 `build_generic_fallback_card`；无 PTC 子树、dispatch 节点与 Trajectory 跳转。 |
| C02 | `read` 文件读取卡 | 部分实现 | `transcript.rs::parse_file_read` 与 `cards.rs::build_read_card` 展示路径、范围、行号代码；点击路径不打开 Files/Preview。 |
| C03 | `read_image` 图片卡 | 缺失 | 无 image attachment 解析、缩略图、尺寸、灯箱或 `attachment/read` 接线。 |
| C04 | `write/edit/apply_patch` diff 卡 | 部分实现 | `parse_file_diff` 与 `build_diff_card` 使用 diff 高亮展示路径/增删；无参数流式预期 diff、多文件列表与 stopped/meta 权威状态。 |
| C05 | Bash/Pwsh/terminal_send 输出卡 | 部分实现 | `parse_bash_tool` 与 `build_bash_card` 展示命令、cwd、退出码和输出；无 running spinner、后台状态与信号胶囊。 |
| C06 | `todo_write` 摘要与 diff | 部分实现 | `build_todo_diff_card` 列出 todos，`TodoDock` 显示完成计数；无 done/total 头部、新增/状态/移动/删除 diff 与早期基线比较。 |
| C07 | `grep/glob` 搜索卡 | 部分实现 | `parse_search_tool` 与 `build_search_card` 展示 pattern/path/results；未处理 `meta.truncated`，点击不打开预览。 |
| C08 | `web_search/web_fetch` 卡 | 部分实现 | `parse_web_tool` 与 `build_web_card` 展示 query/URL/status/Markdown；无结构化来源列表、截断说明与外部打开动作。 |
| C09 | `ask_user_question` 工具行 | 部分实现 | `cards.rs::render_tool_result_card` 对 `ask_user_question` 增加 Reopen，`SessionController::reopen_question` 调 `question/reopen`；快照仍通过可回答 dialog 展示，不是只读回看。 |
| C10 | 控制回执/结构化 details 卡 | 部分实现 | 已知工具走专用卡，其他走 `build_generic_fallback_card` 并显示原始 arguments/result；无 Subagent/Job/Schedule/Goal/LSP 等 key/value details 模型。 |
| C11 | Auto-review 拒绝优先呈现 | 缺失 | 无 auto-review meta 解析、拒绝优先卡或参数隐藏策略。 |
| C12 | 改动/交付卡与 review diff | 部分实现 | `workspace/changes` 事件生成 `DeliverablesBlock`，`build_deliverables_card` 显示 summary/diff；未调用 `workspace/changes` RPC 指定 turn/fileIndex，无 review tabs、stat 或 Present 文件按钮。 |
| C13 | Cordis 动态插件定义卡 run/stop | 缺失 | base 不挂载 Cordis define runner，GUI 无动态实例卡或 run/stop。 |
| C14 | 通用 Markdown/终端/diff/读取/搜索渲染器 | 已实现 | `render_markdown`、`build_code_block`、`build_card_shell` 为 M/C 卡提供统一 Markdown、SourceView、复制与展开能力。 |
| C15 | 下载/另存/图片复制 PNG/菜单 | 缺失 | 无 `GtkFileDialog.save`、PNG clipboard action 或文件动作菜单。 |
| C16 | 拖放字节传递 | 缺失 | 无 `GtkDropTarget`、拖放遮罩或文件转 prompt attachment。 |
| C17 | 在应用中打开 Workspace/文件 | 部分实现 | `ConversationView::new` 的 Open Folder/Open Terminal 调 `desktop.rs::{open_path_in_default_app,open_terminal_in_dir}`；无应用候选列表、文件 stat 与右键“打开方式”。 |
| C18 | 右键菜单框架 | 缺失 | 仅会话/工作区行使用固定 `GtkPopover`；无按目标类型异步解析的统一右键菜单。 |

## 5. A：审批与提问

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| A01 | 工具审批卡接管 composer | 部分实现 | `app/src/ui/approval_dialog.rs::show_approval_dialog` 显示 toolName/callId/reason，Allow Once/Reject/关闭均回传，`connection.rs` 无 responder/超时 fail-closed；当前是模态 dialog，不接管 composer，也无命令预览。 |
| A02 | 审批 Enter/Esc | 缺失 | dialog 只有按钮回调，无 Enter 允许、Esc 拒绝或焦点优先级。 |
| A03 | 审批风险说明浮层 | 缺失 | 只显示 `reason`；bridge 未扩展 displayReason/warning/detail，GUI 无风险 popover。 |
| A04 | 通用提问卡/选项/自定义/翻页/跳过 | 部分实现 | `show_user_question_dialog` 对每题显示 question/detail 与自由文本 Entry，支持 Submit/Cancel；未渲染 options、multiSelect、推荐项、翻页与单题跳过。 |
| A05 | 提问倒计时/聚焦暂停/超时后作答 | 缺失 | 无 remainingMs、倒计时、聚焦暂停或超时后可答状态；base 不提供 attachWait 剩余时间。 |
| A06 | 迟到回答/草稿恢复/关闭重开 | 部分实现 | `question/reopen` 与 ask_user_question Reopen 已接线，关闭回 cancelled；问题草稿不持久化，重开快照不是只读。 |
| A07 | 已回答气泡与只读回看 | 缺失 | 答案作为普通 user message 或 tool result 显示，无问答回显气泡和只读面板。 |
| A08 | Plan review 专用卡 | 部分实现 | `show_user_question_dialog` 检测 `UserQuestionIntent::PlanReview`，显示 detail Markdown，“同意执行”回 approve，“要求修改”回 cancelled；仍是模态 dialog，不接管 composer。 |
| A09 | Plan 模式徽章与退出 | 部分实现 | `SessionModel::handle_event` 跟踪 plan mode，`build_plan_mode_badge` 在 transcript 显示；composer 无徽章/退出按钮，未调用 `/plan off`。 |
| A10 | 已提交计划卡/完整计划预览 | 部分实现 | `exit_plan_mode` 生成 `RenderBlock::PlanCard`，`build_plan_card` 渲染 Markdown；不自动打开右栏完整预览，历史分页不定位计划。 |

## 6. S：设置与模型

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| S01 | 设置外壳/配置表单/冲突保留草稿 | 部分实现 | `app/src/ui/settings.rs::build_preferences_dialog` 使用 `AdwPreferencesDialog`；Bridge→Settings Editor 调 `settings/describe`，保存调 `settings/mutate(expectedRevision)`，`-32020` 保留草稿并显示 `AdwBanner`；仍是 namespace + JSON patch，不是 schema 生成的字段表单。 |
| S02 | 语言选择 | 缺失 | 无 locale ComboRow、词典或 `settings/mutate` 写入。 |
| S03 | 代码工作工具开关 | 部分实现 | General 页有 Developer Tools `AdwSwitchRow`，但未连接 settings/get/mutate，也不控制 Trajectory 显隐。 |
| S04 | 连接恢复/服务版本变化/版本行 | 部分实现 | `show_connection_lost`/`restart_supervisor` 提供重连，General 页显示 dsh/protocol 版本；版本值硬编码，无 release change 检测。 |
| S05 | 打开配置文件 | 缺失 | 无打开 profile/cordis 配置文件的动作。 |
| S06 | Models provider 行/编辑/排序/增删 | 部分实现 | Models 页有 API key、credential records 与静态 model catalog；未调用 `model/catalog` 动态渲染，无 provider 编辑/排序/增删。 |
| S07 | API Key 只写/校验/保存/删除范围 | 部分实现 | `AdwPasswordEntryRow` 写入 Secret Service 并调 `credentials/rotate`，masked status 不回读；无 ASCII 校验、保存 spinner 和删除范围流程。 |
| S08 | 首启 DeepSeek 引导与版本说明 | 部分实现 | `app/src/ui/onboarding.rs::show_onboarding_dialog` 在 `credentials/status` 未配置时显示 key 表单与 Configure Later，保存后 inject；无 WelcomeNotice 版本确认与 settings 持久化。 |
| S09 | DeepSeek 账号登录/资料/余额/充值 | 部分实现 | Settings→Account 调 `account/status|profile|balance|sign-out`，错误显示 unavailable 而非零余额；浏览器 sign-in 明确标注不可用，无 watch、充值和配额弹窗。 |
| S10 | Agent loop 并行上限 | 缺失 | 无 agent-loop namespace 表单。 |
| S11 | 子代理深度/并行/模型设置 | 缺失 | 无 subagent settings 表单。 |
| S12 | Web 搜索/获取 provider 与结果数 | 缺失 | 无 web provider 目录或结果数表单。 |
| S13 | 终端超时/输出上限 | 部分实现 | Shell 页显示静态 Command Timeout；无 schema 字段和 `settings/mutate`。 |
| S14 | 会话日志上传开关 | 缺失 | 无 session-log namespace 检测或开关。 |
| S15 | 默认权限与 `/permission` 会话选择 | 部分实现 | `Composer::permission_button` 与 `open_permission_picker` 调 `permission/get|set`，`permission.updated` 实时同步；默认权限设置和 `/permission` 命令弹窗未实现。 |
| S16 | Agent Preset 卡/默认值/YAML | 缺失 | base 不挂载 `AgentPresetRegistry`，协议未定义 preset methods，GUI 无 preset 卡。 |
| S17 | Goal 条带编辑/暂停/恢复/清除 | 部分实现 | Settings→Bridge 调 `goal/get` 并显示 objective/phase/revision/rounds，`goal.updated` 刷新；composer 上方无 GoalBar，未接 edit/pause/resume/clear。 |
| S18 | `/model` 与会话模型切换 | 部分实现 | `open_model_picker` 调 `session/model/set` 并更新按钮；列表硬编码，未调 `model/catalog`/`session/model/get`，无 reasoningEffort 与失败回滚。 |
| S19 | 主题/字号/字体 | 部分实现 | `AdwComboRow` 通过 `AdwStyleManager` 切换 System/Light/Dark，`configure_system_style` 跟随系统；无字号、字体族和 Host settings 持久化。 |
| S20 | 快捷键速查/录制/恢复 | 部分实现 | `win.show-help-overlay` 使用 `GtkShortcutsWindow` 显示固定键位；无搜索、录制、冲突检测、恢复默认与持久化。 |

## 7. P：插件与凭据

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| P01 | 插件列表/状态诊断/刷新 | 部分实现 | Settings→Plugins 调 `plugin/inventory` 渲染 bundles/plugins，Refresh 重新读取；无 pending/failed/unloading 诊断和 preset 分组。 |
| P02 | Bundle/组件启停与 restart-required | 部分实现 | `toggle_plugin` 调 `plugin/enable|disable`，restartRequired 显示 Banner，Restart Now 冷启动并 follow 活动会话；inventory 行本身无启停开关，也未恢复全部打开会话。 |
| P03 | 插件配置表单 | 缺失 | 无 bundle/row/namespace 动态配置表单。 |
| P04 | Bundle 详情文档与错误重试 | 缺失 | 无 `plugin/detail` 调用、README 对话框或失败重试。 |
| P05 | 添加插件与 inspect 预检 | 已实现 | Settings→Plugins 的 spec 行支持 npm name/local path/Git URL，Inspect 调 `plugin/inspect` 并显示 JSON，Install 调 `plugin/install`。 |
| P06 | 安装进度/日志/取消/恢复/构建审批 | 部分实现 | `plugin.install-state`/`plugin.install-log` 经 `SettingsDialogHandle` 更新阶段和日志，Cancel 调 `plugin/cancel-install`，Wait 方法已接；进度不是独立模态，无 approvedBuilds 审批与断线恢复。 |
| P07 | 卸载/更新/版本检查与恢复 | 部分实现 | inventory 行提供 Remove 确认、Versions 与 Update 对话框，分别调 `plugin/remove|versions|update`；无失败恢复旧版或更新状态流。 |
| P08 | 安装源选择/记忆/探测 | 缺失 | 无 `plugin/registries` UI、registry 选择、测速或本地记忆。 |
| P09 | 插件市场索引/搜索/分页/安装确认 | 缺失 | bridge 未定义 `plugin-market/*`，Settings 明确显示“插件市场依赖 Web Host，本客户端暂不支持”，不伪造空列表。 |
| P10 | 设置→内置插件只读清单/页面重试 | 部分实现 | `plugin/inventory` 已提供只读清单；未接 `plugin/retry`、搜索或 Loader 诊断。 |
| P11 | 凭据 describe/set/unset 与完整 provider seam | 部分实现 | `CredentialStore` 使用 keyring Secret Service，controller 调 `credentials/status|inject|rotate`，Settings records 调 `credentials/records/list|read|modify|delete` 且只显 metadata；API key 删除按钮和 OAuth grant seam 未接。 |
| P12 | 入口定制/Pin/插件草稿 | 缺失 | 无“更多”入口、Pin 排序或插件草稿动作。 |

## 8. O：其他外围

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| O01 | 启动页/失败报告 | 已实现 | `MainWindow::new` 的 startup `AdwStatusPage`/spinner 与 `AdwBanner` Retry；`bootstrap_supervisor` 把 `SupervisorError` 映射到 `show_startup_error`。 |
| O02 | 浏览器 token/Cookie/信任检查 | 有意不移植 | Unix socket 0600、instance lock 与 `libadwaita/initialize` 构成本机信任边界；无 Cookie/HTTP token。 |
| O03 | 客户端模块系统/bundle 路由/页面同步重试 | 有意不移植 | 使用静态 Rust widget 模块与 `libadwaita/plugin/*`，不实现浏览器 CJS loader。 |
| O04 | HMR/开发 bundle 重建 | 有意不移植 | `app/src/supervisor.rs::generate_cordis_patch_yml` 禁用 hmr，插件变更走 restart-required 冷重启。 |
| O05 | RPC/WebSocket/Fetch 路由 | 有意不移植 | `app/src/connection.rs::BridgeConnection` 统一 Unix socket NDJSON JSON-RPC，无第二条 Web Host 通路。 |
| O06 | Worker/流式上传/receipt | 缺失 | bridge 有 attachment upload/read，GUI 无文件选择、进度、receipt 或拖放接线。 |
| O07 | 快捷键引擎/持久化 | 部分实现 | `main.rs` 设置 app/window accelerators，`setup_composer`/`setup_double_esc` 使用 `GtkEventControllerKey`；无自定义键位持久化或冲突管理。 |
| O08 | Locale 词典/fallback | 缺失 | 无 Rust i18n 词典、locale fallback 或动态文案处理。 |
| O09 | observable store/useResource | 部分实现 | `SessionModel`/`ConversationTranscript` 持有领域真相，controller 用 Tokio channel + `spawn_local` 更新 UI，terminal/files panel 持有会话资源状态；无统一 resource handle 与 release 生命周期。 |
| O10 | React renderer/Slots | 有意不移植 | 采用固定 GTK4/libadwaita 控件与 Rust 领域对象，不引入 React/Slot/JS runtime。 |
| O11 | Desktop 产品埋点 | 有意不移植 | 不新增产品遥测面。 |
| O12 | 会话后台 jobs 列表/输出/双击停止 | 已实现 | `ConversationView::set_jobs` 渲染 job roster，`follow_job` 调 `job/follow`，`IncomingNotification::JobOutput` 追加输出，`win.kill-job` 使用 `AdwAlertDialog` 二次确认后调 `job/kill`。 |
| O13 | 自动化计划任务 CRUD/运行/历史/时钟 | 已实现 | `SchedulesPage` 调 `schedule/catalog|create|edit|run|history|delete`，提供 New/Run/History/Edit/Delete 对话框，`schedule.changed` 刷新；无 delivery limits、暂停/恢复与目标会话时钟标记。 |
| O14 | PWA/manifest/子路径挂载 | 有意不移植 | 使用 `AdwApplication`、应用 id、`.desktop` 与 GApplication 激活，不实现 PWA/URL scope。 |
| O15 | 系统通知与提示音 | 部分实现 | `desktop.rs::send_turn_completed_notification`、`send_background_notification`、`withdraw_notification` 使用 `GNotification`；无提示音和通知开关设置。 |
| O16 | 测试支撑包 | 已实现 | `app/tests/` 10 个 Rust 测试文件共 65 项，`bridge/tests/` 7 个 suite 共 39 项；产品窗口无测试入口。 |

## 9. G01–G29 当前缺口账本

| ID | 影响行 | 状态 | 证据与边界 |
|---|---|---|---|
| G01 | N03/N15 | implemented | `bridge/src/bridge.ts` 实现 `workspace/list|create|rename|remove`；GUI `SessionController::load_workspaces` 等已接线。 |
| G02 | N10 | implemented | `libadwaita/session/rename` 与 `SessionController::rename_session` 已接线。 |
| G03 | N12/M09 | implemented | `libadwaita/session/fork` 与 `SessionController::fork_session` 已接线。 |
| G04 | N03/N13 | implemented-partial | bridge `session/search` 返回 `fullText:false`；base 的 `session-query-sqlite` 以 `openAt: never` 挂载（`packages/bundle/base/cordis.patch.yml` 第 157–161 行），GUI 尚未调用搜索方法。 |
| G05 | N17 | implemented-partial | bridge `terminal/create|list|input|output|close` 有真实 dsh 测试；无 resize RPC，GUI 使用 fallback TextView/Entry 而非 VTE。 |
| G06 | N18/N19/C02/C07/C12/C15/C17 | implemented | `files/list|stat|read|readBytes` 已实现，`FilesPanel`/`PreviewPanel` 已消费。 |
| G07 | N19 | base-blocked | `ctx.officeToPdf` 属于 web-app 文档预览组合；base 不提供，GUI 回退系统默认应用。 |
| G08 | N21 | implemented-partial | bridge `subagent/list|prompt|interrupt` 已实现；GUI 接 list/open/interrupt，未接子代理 prompt 续接。 |
| G09 | N22 | base-blocked | `packages/bundle/base/cordis.patch.yml` 第 366–367 行插入 agent-team，但本机探针 `ctx.get('agentTeams')` 为 undefined，bridge 返回 `-32040`。 |
| G10 | M02/M18 | implemented | `session/queue/list|edit|remove|steer` 与 `session.inbox` 已实现并接入 QueueDock。 |
| G11 | M07/M21/M28/C03/O06 | implemented-partial | bridge `attachment/upload|read` 与 prompt image 已实现；GUI 未接附件选择、读取、画廊或缓存。 |
| G12 | M11/M12/M22/S17 | implemented-partial | `session/projections` 与 goal 投影可用；`packages/bundle/web-app/cordis.patch.yml` 第 98–104 行的 `turnOutline`/`sessionStats` 不属于 base，协议明确省略，不伪造。 |
| G13 | M25 | implemented-partial | bridge 从持久 `tool/result` 派生 `tool-result.updated`；GUI Trajectory 未订阅，不承诺原始未落盘中间值。 |
| G14 | M26 | implemented | `message-feedback/list|put|delete` 与 `session-feedback/record` 已实现，GUI thumbs/Feedback 对话框已接线。 |
| G15 | M16/A09 | implemented | `command/list|execute`、`skill/list`、`reference/list` 已实现并接入 trigger popups；`/plan off` 参数语义仍受上游命令目录限制。 |
| G16 | S01/S03/S06/S10–S13/P03 | implemented | `settings/describe`、`settings/mutate(expectedRevision)` 与 `settings.updated` 已实现；GUI Settings Editor 已消费。 |
| G17 | S06/S12 | implemented-partial | bridge 提供 `model/discover` 与 `web-search/providers`；base 仅 `llm-pi-ai` 可 discovery 且可能返回 `-32044`，本机 web provider 列表可为空，GUI 未接。 |
| G18 | S09/M13 | implemented-partial | bridge 提供 status/profile/balance/sign-out；`packages/credentials/deepseek-account-platform` 的浏览器 OAuth 依赖 Host webServer，base-only 不提供 start-sign-in/watch/quota 动作。 |
| G19 | S15 | implemented | `permission/catalog|get|set` 与 `permission.updated` 已实现，composer permission picker 已接线。 |
| G20 | S16 | base-blocked | `AgentPresetRegistry` 仅 `packages/bundle/web-app/cordis.patch.yml` 第 598 行挂载，preset 声明会重复挂载 base 工具行；协议未定义 `agent-preset/*`。 |
| G21 | S17 | implemented | `goal/get|edit|pause|resume|clear` 与 `goal.updated` 已实现；GUI 当前只消费 get/updated。 |
| G22 | P04–P08 | implemented | `plugin/detail|inspect|install|cancel-install|wait-install|remove|versions|update|registries` 与 install-state/log notifications 已实现；GUI 未接 detail/registries/build approval。 |
| G23 | P09 | base-blocked | `@deepseek-ai/dsh-plugin-market`、`dsh-plugin-market-index` 与 `dsh-experimental-plugin-market-bundle` 不在 base 组合，bridge 无 `plugin-market/*`；GUI 显示不可用而非空市场。 |
| G24 | P01/P03/P10 | implemented-partial | `plugin/inventory|retry` 已实现；GUI 使用 inventory，未接 retry 和完整 Loader 诊断。 |
| G25 | O12 | implemented | `job/list|follow|kill` 与 `job.output` 已实现并接入 header Jobs popover。 |
| G26 | N24/O13 | implemented-partial | `schedule/catalog|create|edit|run|history|delete` 与 `schedule.changed` 已实现并接入 SchedulesPage；未定义 delivery-limits/pause/resume。 |
| G27 | C13 | base-blocked | `cordis_define` 动态实例控制属于 `packages/extensions/ui-cordis`/web-app runner，base 不挂载；协议不定义 `cordis-control/*`。 |
| G28 | A03/A05/A06/C09 | implemented-partial | bridge 保存 `user-question/ask` 快照并支持 `question/reopen`；不伪造 remainingMs，approval warning/displayReason 仍未扩展。 |
| G29 | P11/S09 | implemented | `credentials/records/list|read|modify|delete` 已实现，GUI 只展示 metadata，不回读 secret。 |

## 10. 与 DSH Web 的功能差异

| # | 剩余差异 | 用户可见影响 | 保留原因 | 应用内 workaround |
|---:|---|---|---|---|
| 1 | 附件选择、拖放、上传进度、图片画廊与灯箱缺失 | 无法上传文件/图片，不能在消息中浏览原图 | UI scope；bridge `attachment/upload|read` 已可用 | 把文本或本地路径粘贴到 composer；文件结果通过 Files/Preview 查看 |
| 2 | Markdown 表格、脚注与链接 scheme 白名单缺失 | GFM 表格/脚注显示不完整，危险链接未被集中拦截 | UI scope；pulldown-cmark 解析器未接入这些节点 | 在代码块中查看原始表格/脚注文本 |
| 3 | 文件链接 `#L` 行号定位与 HTTP 打开偏好缺失 | 点击消息链接不会打开预览或外部浏览器 | UI scope；无统一 LinkController | 从 Files tab 手动定位路径 |
| 4 | Trajectory 表、检查器、时间线与实时原始结果缺失 | 开发者无法逐事件调试 prompt header、schema 与工具原始输出 | UI scope + G13 派生边界 | 使用 tool card 的原始 JSON 展开 |
| 5 | Token/speed/cache 统计与上下文占用率缺失 | 无法看到每轮和会话资源消耗 | G12 base 省略 `sessionStats`/`turnOutline`，GUI 未消费 usage chunk | 仅显示运行状态，不显示估算值 |
| 6 | 审批为模态 dialog，不接管 composer，且无 Enter/Esc | 审批时焦点和键盘流程与 Web 不同 | UI scope + G28 元数据限制 | 使用鼠标点击 Allow Once/Reject |
| 7 | 提问未渲染选项、multiSelect、翻页、跳过、倒计时与只读回答 | 多选/多题问题只能用自由文本回答 | UI scope + G28 remainingMs/attachWait base 限制 | 自由文本输入；ask_user_question 行可 Reopen |
| 8 | 设置仍为 namespace + JSON patch，而非 schema 驱动字段表单 | 用户需理解 JSON，字段校验较少 | UI scope；G16 schema 已可用但未生成控件 | Settings Editor 保留草稿，冲突时 Banner 提示 |
| 9 | 语言、字号、字体、Developer Tools 与多数 Host namespace 未持久化 | 界面偏好不能跨会话保存 | UI scope；部分 namespace 在 base 可缺失 | 主题可在当前进程用 StyleManager 切换 |
| 10 | Models provider 编辑、排序、增删与在线 discover 缺失 | 只能使用固定模型列表和 API key | UI scope + G17 base discovery 限制 | 通过 API key 和固定 model picker 切换 |
| 11 | DeepSeek 浏览器登录、watch、充值与配额恢复弹窗缺失 | 不能从原生窗口完成 OAuth 登录 | G18 base-blocked；OAuth 回调依赖 Host webServer | 使用 API key onboarding；账号页可读取状态/资料/余额并退出 |
| 12 | 插件市场、registry 测速、构建审批、详情 README 与 Loader retry 缺失 | 只能直接输入 bundle spec，不能浏览市场或批准构建脚本 | G23/G22 UI scope 与 base 组合限制 | 使用 npm/Git/local spec 直接 Inspect/Install |
| 13 | Agent Team、Agent Preset、Cordis define run/stop 缺失 | 不能管理 Team、preset 或动态 Cordis 实例 | G09/G20/G27 base-blocked | 使用默认 profile 与现有 session/permission 控制 |
| 14 | Workflow 专用节点和成员子会话导航缺失 | 多阶段 workflow 只显示通用事件 | UI scope；session events 已可 follow | 从 Subagents 区打开已知 child session |
| 15 | 文件下载/另存、复制 PNG、PDF/Office 预览与行定位缺失 | 交付文件不能直接保存或预览重型文档 | G07 base-blocked + UI scope | 用系统默认应用打开文件或目录 |
| 16 | 会话标题/全文搜索与侧栏 Pin/排序/状态点/未读缺失 | 会话多时导航效率低于 Web | UI scope + G04 全文 base 限制 | 使用 Workspace 分组列表和启动恢复 |
| 17 | 右栏无分屏、浮动、全屏与每会话 tab 持久化 | 多资源工作区布局不能保存复用 | UI scope | 手动切换 Terminal/Files/Preview 固定标签 |
| 18 | 终端是轮询 fallback，不是 VTE PTY | 无 shell 选择、重命名、resize、主题终端行为 | UI scope + G05 无 resize RPC；当前构建无 VTE 依赖 | Entry 发送命令，TextView 查看有界回滚并手动 Close |

## 11. 手动桌面验证清单

以下项目依赖真实桌面会话、凭据或窗口管理器，不能仅由 headless test 判定：

1. 在已解锁 GNOME Keyring/Secret Service 会话中完成首启 API Key 输入，确认 keyring 不回读明文、`credentials/inject` 成功且刷新 Models。
2. 使用 Portal 或原生文件选择器创建 Workspace/Session，确认目录授权、取消选择与权限错误提示。
3. 触发一轮非前台完成，确认系统通知出现、点击可激活窗口、完成后通知撤回且无重复通知。
4. 启动第二个进程，确认 D-Bus 单实例激活只聚焦已有窗口，不创建第二个 dsh child 或 socket。
5. 在 Wayland 与 X11 下切换深浅色、高对比、缩放与减少动效，确认 HeaderBar、卡片、代码块和 Toast 可读。
6. 打开真实会话的 Terminal tab，确认 `terminal/list` 恢复、输入 echo、500ms 轮询输出、缓冲上限和 Close 终止 PTY。
7. 创建 Schedule 并立即 Run，确认 catalog/history/delete 与 `schedule.changed` 刷新；若上游 timing/target 被拒绝，确认错误可见且不显示假成功。
8. 在真实 dsh 上连续运行 30 秒，确认 app 与 child 存活、socket 权限 0600、SIGTERM 退出码为 0、日志无 panic 和凭据泄露。

## 12. 验收结论

当前实现已覆盖核心会话、可靠 transcript、Queue/Steer、审批与提问基础交互、Workspace、Files/Preview、Terminal fallback、Schedules、Jobs、Plugins 生命周期、Settings CAS、credential records 与账号只读视图。相对 DSH Web，主要剩余差距集中在附件、Trajectory/统计、schema 驱动设置、OAuth 登录、插件市场、重型文档预览和 VTE 终端。

按 132 行统计：**23 已实现、65 部分实现、35 缺失、8 有意不移植、1 有意简化**。因此不能宣称“功能与 DSH Web 无异”；上述 18 项差异与 G07/G09/G20/G23/G27 的 base-blocked 边界是后续验收和范围决策的明确输入。
