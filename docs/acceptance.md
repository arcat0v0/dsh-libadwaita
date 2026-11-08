# dsh-libadwaita 对 DSH Web 功能验收对照

日期：2026-11-06  
基线：`docs/dsh-web-feature-inventory.md` 与 `docs/web-ui-map.md` 的 N/M/C/A/S/P/O 编号，共 132 行。  
实现基线：bridge `33b486a`、app `ccdfc1f`；bridge 39 项测试、app 82 项测试均通过。
判定只依据当前仓库代码接线与可执行证据，不依据计划、映射表预期或上游 Web 的理论能力。

## 0. 验收口径

- `已实现`：当前代码具备对应功能的主要用户语义，并有仓库内可定位接线；不表示与上游逐像素一致。
- `部分实现`：已有真实接线，但存在明确且可复现的缺口；证据与缺口同时列出。
- `缺失`：未找到有效用户接线。
- `有意不移植`：设计文档明确排除的浏览器、React、PWA、遥测等机制。
- `有意简化`：设计文档明确接受的降级，例如 TeX 保留源码而不引入 KaTeX/WebView。

测试证据：

- `cargo test --manifest-path app/Cargo.toml`：82 passed。
- `npm test --prefix bridge`：39 passed。
- `app/tests/supervisor_dsh_tests.rs::test_real_dsh_lifecycle_and_session_create`：真实 dsh 启动、握手、会话创建、停止与重启。
- `bridge/tests/integration/gap-batch-d.spec.ts`：真实 dsh terminal 与 schedule 生命周期。

## 1. 状态汇总

| 区域 | 已实现 | 部分实现 | 缺失 | 有意不移植 | 有意简化 | 合计 |
|---|---:|---:|---:|---:|---:|---:|
| N 会话与导航 | 10 | 14 | 3 | 1 | 0 | 28 |
| M 消息与流式 | 10 | 16 | 1 | 0 | 1 | 28 |
| C 工具与展示卡片 | 1 | 14 | 3 | 0 | 0 | 18 |
| A 审批与提问 | 3 | 6 | 1 | 0 | 0 | 10 |
| S 设置与模型 | 7 | 11 | 2 | 0 | 0 | 20 |
| P 插件与凭据 | 2 | 7 | 3 | 0 | 0 | 12 |
| O 其他外围 | 4 | 4 | 1 | 7 | 0 | 16 |
| **总计** | **37** | **72** | **14** | **8** | **1** | **132** |

## 2. N：会话与导航

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| N01 | 三栏 AppFrame 与 shell 座 | 部分实现 | `app/src/ui/window.rs::MainWindow::new` 使用 `AdwOverlaySplitView`、`GtkPaned`、`RightDock`、`AdwToolbarView` 与 `AdwToastOverlay`；缺 56px 折叠轨、宽度优先级与窄屏抽屉规则。 |
| N02 | 左侧栏品牌/New Session/面板/设置/折叠轨道 | 部分实现 | `app/src/ui/sidebar.rs::Sidebar::new` 提供品牌、New Session、Workspace/Subagent/Session 区、搜索、Plugins/Schedules/Account/Settings；`win.toggle-sidebar` 可折叠，但无独立轨道视图。 |
| N03 | Workspace/Session 列表与排序/归档过滤 | 部分实现 | `Sidebar::load_workspaces` 按 workspace 分组 session；`Sidebar::load_sessions` 支持本地 Pin 置顶、Active/Archived/All 过滤与 Unarchive，`SessionController::{toggle_pin_session,set_archive_filter,unarchive_session}` 持久化 UiPrefs；缺手动拖拽排序。 |
| N04 | 新建会话/复用 blank/命名创建 | 部分实现 | `SessionController::prompt_new_session_portal` 使用 `GtkFileDialog::select_folder`，`create_session` 调 `session/create` 后 follow；缺同 cwd/preset blank 复用、命名创建与 agentPreset 选择。 |
| N05 | 打开并恢复既有会话 | 已实现 | `MainWindow::set_controller` 的 `win.select-session` → `SessionController::follow_session` → `UiMessage::SessionFollowed`；`ConversationView::update_transcript` 渲染 follow 快照。 |
| N06 | 启动时恢复当前 Session/Workspace | 已实现 | `app/src/session_state.rs::SessionState` 持久化 last session/cwd；`SessionController::handle_session_list_loaded` 在连接后恢复，`persist_session` 在 follow 后保存。 |
| N07 | 官方品牌与首页鲸鱼层 | 缺失 | 仅有 `Sidebar::new` 的 `dsh` 文本与 `ConversationView::new` 的 welcome card；无官方图标、Canvas 或简化鲸鱼动画。 |
| N08 | “关闭”当前会话视图 | 已实现 | `win.close-session` → `SessionController::close_session` → `libadwaita/session/release`；`UiMessage::SessionReleased` → `clear_active_session_ui`，不停止 Agent。 |
| N09 | 归档/取消归档，活动会话二次确认 | 部分实现 | `Sidebar::load_sessions` 的 Archive/Unarchive 菜单分别调 `session/archive(archived:true/false)`，`unarchive_session` 成功后刷新列表；活动会话归档确认仍缺失。 |
| N10 | 手动重命名 | 已实现 | `win.rename-session` → `open_rename_for_id` → `rename_session` 调 `libadwaita/session/rename`；`UiMessage::SessionRenamed` 更新活动会话、标题与缓存。 |
| N11 | 自动标题 | 部分实现 | `setup_ui_receiver` 处理 `IncomingNotification::SessionTitle`，更新 `AdwWindowTitle` 与活动会话；侧栏行只在列表重载时更新，不即时刷新。 |
| N12 | Fork 会话 | 已实现 | 消息动作 `win.fork-session` → `open_fork_dialog` → `fork_session` 调 `libadwaita/session/fork({sessionId,atSeq,increaseTitle:true})`；`UiMessage::SessionForked` follow 新会话。 |
| N13 | 会话元数据/内容搜索 | 部分实现 | `Sidebar::new` 的搜索框经 250ms debounce 调 `SessionController::search_sessions` → `session/search`，空查询恢复完整列表；base 搜索仅标题/cwd 元数据，`fullText:false`，无正文检索。 |
| N14 | 行菜单、Pin、Archive、Toast | 已实现 | `Sidebar::load_sessions` 的菜单提供 Rename/Close View/Archive/Unarchive，行尾 Pin/Unpin 调 `toggle_pin_session` 并持久化，归档/取消归档与 Toast 已接线；无菜单快捷键。 |
| N15 | Workspace 创建/层级/重命名/删除与目录选择 | 部分实现 | `prompt_new_workspace` 选目录，`create_workspace`/`switch_workspace`/`rename_workspace`/`remove_workspace` 使用 `libadwaita/workspace/*`；切换会 release 旧视图并创建新 cwd 会话，但不迁移 composer 草稿。 |
| N16 | 右停靠栏框架 | 部分实现 | `RightDock::new` 使用 `AdwTabView`/`AdwTabBar` 固定 Terminal/Files/Preview；`UiPrefs` 按会话保存选中 tab 与 dock 可见性，`win.toggle-dock` 与 follow 恢复；缺分屏、浮动、全屏。 |
| N17 | 终端 tab | 部分实现 | `app/src/ui/terminal_panel.rs::TerminalPanel` 使用只读 `GtkTextView` + `GtkEntry`，`restore` 调 `terminal/list`，无终端时 `create_terminal`，输入调 `terminal/input`，可见时 500ms `terminal/output` 轮询并有 5000 行缓冲，Close 调 `terminal/close`；无 VTE、shell 选择、重命名与 resize。 |
| N18 | 文件 tab | 已实现 | `FilesPanel` 基于 `ui/file_tree.rs::FileTreeModel` 渲染可展开层级树（目录优先排序、按需加载、折叠保留缓存、消失目录剪枝、面包屑导航、刷新、预览定位），消费 `files/list` 的工作区相对路径与 `files/stat`；目录行切换展开并懒加载，文件行 `preview_file` 并展开祖先面包屑。 |
| N19 | 文档预览 tab | 部分实现 | `PreviewPanel::show_text` 使用 `sourceview5::View` 与语言映射，`show_image` 使用 `GtkPicture`，`show_unavailable` 显示失败；缺 PDF、Office、HTML、行定位与渲染/源码切换。 |
| N20 | 浏览器 tab | 有意不移植 | 项目禁止内嵌 WebView；`app/src/ui/desktop.rs::open_path_in_default_app` 使用系统默认应用作为原生替代。 |
| N21 | 子代理目录/血缘/transcript/@引用 | 部分实现 | `Sidebar::load_subagents` 从 `subagent/list` 渲染子代理，`win.open-child` → `open_child_session` 调 follow 并设置面包屑，`win.interrupt-subagent` → `interrupt_subagent`；无子代理续接 prompt、深层血缘树与专用 @引用候选。 |
| N22 | Agent Team 开关与共享任务板 | 缺失 | bridge 有 `agent-team/get|set-enabled`，但 base-only 探针返回 `-32040`；GUI 无 Team 开关或任务板。 |
| N23 | 插件面板入口 | 已实现 | `Sidebar::new` 的 Plugins 行触发 `win.show-plugins`，`MainWindow::new` 切换 `ConversationView::select_panel("plugins")`；完整管理位于 Settings→Plugins。 |
| N24 | 自动化任务页入口 | 已实现 | `win.show-schedules` 切换 `ConversationView` 的 schedules page；`app/src/ui/schedules.rs::SchedulesPage` 已接入真实 catalog。 |
| N25 | Workflow 运行节点与子会话导航 | 缺失 | transcript 对未知 workflow 事件仅进入 `RenderBlock::GenericEvent`；无阶段节点、成员导航或专用折叠面板。 |
| N26 | 会话状态点/完成未读 | 部分实现 | 侧栏按 `running_sessions` 显示运行图标，非活动会话 running→idle 时写入 `unread_sessions` 并加粗；`handle_status_notification` 同时驱动通知。缺待审批/待提问的优先级仲裁。 |
| N27 | 连接状态与手动重连 | 已实现 | `show_startup_error`/`show_connection_lost` 使用 `AdwBanner` 与 Retry，`bootstrap_supervisor`/`restart_supervisor` 冷启动并重新 initialize；无独立状态 pill。 |
| N28 | 重连状态修复 | 部分实现 | `app/src/supervisor.rs::GenerationTracker::record_generation` 跟踪 generationId，`UiMessage::SupervisorRestarted` 后 follow 活动会话；无自动退避重连、同代际 page 修复与全部打开会话 rebaseline。 |

## 3. M：消息与流式

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| M01 | 事件装配/历史前插/settlement | 部分实现 | `app/src/transcript.rs::ConversationTranscript::{apply_baseline,apply_stream_frame,apply_event,prepend_events}` 支持 baseline、live 替换、committed/abandoned、seq 去重与历史前插；检测到 gap 后只返回 `GapDetected`，无自动修复。 |
| M02 | Chat 用户消息/steering/回显 | 已实现 | `SessionController::submit_text` 根据运行状态选择 prompt/steer，`ConversationTranscript::push_pending_user` 显示 pending，`user/message` 到达后原子替换，失败由 `UiMessage::EchoFailed` 恢复。 |
| M03 | Assistant 文本/思考/工具 delta、定稿/中断 | 已实现 | `ConversationTranscript::apply_stream_frame` 聚合 text/reasoning/tool delta，`assistant/message` 与 committed 替换 live，abandoned 移除；`app/tests/transcript_tests.rs` 覆盖正常、废弃与 reasoning。 |
| M04 | Markdown/GFM/表格/脚注/安全链接 | 部分实现 | `render_markdown` 支持表格（GtkGrid）、脚注、任务列表、代码与链接；`attach_link_handler` 允许 http/https/mailto，拦截 javascript/data/vbscript，文件链接切 Files/Preview 并定位行；流式尾部增量已实现（稳定前缀/尾部拆分，未闭合代码围栏不提前定稿）；缺超大表格虚拟化与超长行降级。 |
| M05 | 代码高亮/复制/换行/行号/流式 fence | 部分实现 | `build_code_block` 使用 `sourceview5::View`、语言识别、行号与复制按钮，live handle 每次更新正文；无换行开关、未闭合 fence 专用策略与超长行降级。 |
| M06 | TeX 数学与复制 | 有意简化 | 按 `docs/web-ui-map.md` §10.3 不引入 KaTeX/WebView；TeX 作为普通文本显示，不声称完成数学排版。 |
| M07 | 图片/文件附件、画廊、灯箱 | 已实现 | Attach 按钮、文件拖放与 Ctrl+V 图片经 `attachment/upload` 后作为 prompt image 发送；`build_user_message` 用 GtkFlowBox 展示图片并点击打开 AdwDialog 灯箱，非图片插入 `@path` 文件 chip（名称/大小）；助手消息的 image/file content block 经 `parse_message_attachments` 渲染为缩略图与文件 chip。 |
| M08 | 文件链接行号/HTTP 打开偏好 | 已实现 | `links::classify_link` 解析 `#L24-L30`/`:24-30`，`SessionController::open_link_target` 对 HTTP(S)/mailto 调系统 `UriLauncher`，对文件路径切 Preview 并 `PreviewPanel::set_pending_line` 滚动。 |
| M09 | 消息复制/时间/fork/扩展动作 | 部分实现 | 用户与定稿 assistant 消息均有 Copy 按钮和本地 HH:MM 时间戳；assistant 的 Fork 仅在最后一条已定稿消息显示（上游 branchUnavailable 边界），其余消息隐藏；无通用扩展动作菜单或时间戳悬浮详情。 |
| M10 | 工作过程分组/思考预览/整轮折叠 | 部分实现 | 会话头 DropDown 提供 Concise/Standard/Detailed/Expanded 并持久化，四档按上游 presentation policy 区分：Compact 分组折叠且无过程明细/思考预览；Standard 折叠并显示运行过程明细与已结思考首行；Detailed 仅历史轮次分组、当前轮次内联；Expanded 不分组。缺 Host `collapseTiming`/`foldedReplies` 与整轮外折叠时机。 |
| M11 | 历史分页/锚点/轮次导航/回到底部 | 部分实现 | `ConversationView::new` 监听 transcript adjustment 上沿，`SessionController::load_older_history` 调 `session/page`，`UiMessage::HistoryPageLoaded` 前插并按距底部偏移恢复锚点，`set_history_loading` 显示加载行；已实现尾部跟随（32px 回底阈值、上滚暂停、回底恢复）、浮动回底按钮与右侧轮次轨道（仅已加载轮次、活动轮次高亮、点击跳转、提示预览）；完整 turn outline 依赖 G12，未加载轮次的“先分页再跳转”未实现。 |
| M12 | Token/速度/缓存统计与每轮面板 | 部分实现 | `agent.stream` usage chunk 更新会话头 token 胶囊，`session/projections` 的 tokenUsage 也会显示；无生成速度、缓存命中与每轮明细面板。 |
| M13 | 命令/压缩/重试/错误/配额卡 | 部分实现 | `execute_command` 显示命令回执，`apply_event` 对未识别事件生成 `GenericEvent`，`SessionModel::handle_event` 记录 error code 供 continuation 使用；无压缩、重试、错误与配额专用卡。 |
| M14 | 运行状态/计时/断线暂停 | 部分实现 | `Composer::set_is_running`、`ConversationView::set_status` 与 `set_disconnected` 显示 Running/Idle/Disconnected，`turn_start_times` 用于通知时长；无逐秒计时器和断线期间计时暂停显示。 |
| M15 | 文本选区/右键扩展 | 部分实现 | 消息文本与代码视图可选择；右键菜单支持 Copy/Quote into composer，代码块支持 Copy Code；无按目标类型扩展的通用菜单项。 |
| M16 | 草稿/引用 chip/命令 chip/quote/恢复 | 部分实现 | `trigger_popup` 与 `insert_composer_text` 支持 `/`、`@`、`#` 候选插入，`app/src/drafts.rs::DraftStore` 提供持久化原语；controller 未接入 DraftStore，切换会话不保存/恢复草稿，无结构化 chip/quote。 |
| M17 | 发送与 Host 回显确认 | 已实现 | `submit_text` 先 `push_pending_user`，RPC 成功后 `EchoAck` 清空 composer，持久 `user/message` 原子替换 pending，失败 `EchoFailed` 清 pending、恢复草稿并 Toast。 |
| M18 | Queue/Steer 预览/编辑/删除/插话 | 已实现 | `app/src/ui/queue_dock.rs::QueueDock` 展示 inbox，`SessionController::{refresh_queue,queue_edit,queue_remove,queue_steer_item,queue_steer_all}` 调 queue RPC；`IncomingNotification::SessionInbox` 实时更新。 |
| M19 | Stop 按钮与双 Esc | 已实现 | `setup_composer` 在 running 时把 send button 转为 cancel，`setup_double_esc` 在 500ms 内两次 Escape 调 `cancel_current_turn`；无终端/IME/模态焦点仲裁。 |
| M20 | 失败后“继续生成” | 已实现 | `SessionModel::handle_event` 记录六类 error code，`update_continue_state` 使用 `drafts::continuation_eligibility`，Continue 调 `continue_generation` 发送空 contentBlocks、`mode:"queue"`。 |
| M21 | 附件选择/粘贴/拖放/上传进度/重试 | 部分实现 | Attach 打开 `GtkFileDialog.open_multiple`，窗口 DropTarget 接受 FileList，Ctrl+V 读取 image/png；附件进入 composer 暂存 rail（图片预览、文件卡、移除，按会话保留并由成功回显释放），提交时图片内联、文件经 bridge `admitEncodedFile` 转为 durable ref；无独立上传进度（提交前不预上传）。 |
| M22 | 上下文占用率与组成 | 部分实现 | `session/projections` 与 `session.projection` 的 contextPressure 驱动 Context 按钮百分比，点击显示真实 contextBreakdown JSON；不是上游圆环视觉，也不显示估算值。 |
| M23 | Todo dock | 已实现 | `app/src/ui/todo_dock.rs::TodoDock` 位于 composer 上方，`ConversationView::update_todo` 由 follow projections 与 `todo/write` 事件更新，显示完成计数与只读状态。 |
| M24 | 已发送凭据遮挡与眼睛 | 已实现 | `build_user_message` 调 `secret_mask::mask_secret_patterns`，覆盖 PEM 私钥块、URL 密码、`sk-`/GitHub 系列/AIza/AKIA/ASIA/LTAI/AKID/xox/Slack/Stripe/`whsec_`/JWT 等格式，保留前 8 字符并按码点计数；仅文本被改时显示 reveal，复制按钮复制原文，composer 原文不遮挡。 |
| M25 | Trajectory 表/检查器/时间线/搜索/虚拟行 | 部分实现 | `SessionModel` 保留 2000 条 raw SessionEvent，follow 加载 200 条 page 快照，live `session.event` 追加；`ui/trajectory.rs` 提供 GtkColumnView（seq/time/type/summary）、搜索过滤、GtkPaned 只读 SourceView JSON 检查器与实时追加；无时间线条与 `tool-result/watch` 原始中间值。 |
| M26 | 消息赞踩/反馈弹窗 | 已实现 | `message_renderer` 使用 `assistant/message` 的持久 `message.id` 作为反馈目标；点赞/点踩在已有同项时撤回、否则打开含七类原因与备注的对话框；`put`/`delete` 成功后刷新按钮状态，`version-conflict` 用错误 `data.item` 权威项对账；fork 仍使用 seq 定位。 |
| M27 | Chat/Trajectory 视图选择与固定 | 部分实现 | `AdwViewStack`/`AdwViewSwitcher` 可切换 Chat/Trajectory；选择未按会话持久化，Trajectory 内容缺失。 |
| M28 | Session 图片共享缓存 | 已实现 | `ImageCache` 按 `(sessionId, attachmentId)` 合并并发读取并缓存解码字节；bridge `attachment/read` 接受完整 `ImageAttachmentRef` 走 Host `readImage` 授权读取；会话 release/archive 时释放缓存；用户消息、助手图片与队列缩略图共用。 |

## 4. C：工具与展示卡片

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| C01 | 工具调用树/PTC/通用 fallback | 部分实现 | `ConversationTranscript` 聚合 `tool/ptc-dispatch-start`/`tool/ptc-dispatch` 并按 `parentCallId` 组装 `ToolCallInfo::sub_calls` 树，`render_tool_result_card` 递归渲染子调用（`dsh-subcalls` 缩进竖线）并回退 `build_generic_fallback_card`；无 dispatch 节点跳转 Trajectory 与深度上限保护。 |
| C02 | `read` 文件读取卡 | 部分实现 | `transcript.rs::parse_file_read` 与 `cards.rs::build_read_card` 展示路径、范围、行号代码；点击路径不打开 Files/Preview。 |
| C03 | `read_image` 图片卡 | 部分实现 | `cards.rs` 对 `read_image` 解析 attachmentId/data 并复用 `attachment/read` 缩略图组件，点击开 AdwDialog；缺少尺寸/字节元数据展示，非标准结果回退通用卡。 |
| C04 | `write/edit/apply_patch` diff 卡 | 部分实现 | `parse_file_diff` 与 `build_diff_card` 使用 diff 高亮展示路径/增删；无参数流式预期 diff、多文件列表与 stopped/meta 权威状态。 |
| C05 | Bash/Pwsh/terminal_send 输出卡 | 部分实现 | `parse_bash_tool` 与 `build_bash_card` 展示命令、cwd、退出码和输出；无 running spinner、后台状态与信号胶囊。 |
| C06 | `todo_write` 摘要与 diff | 部分实现 | `build_todo_diff_card` 列出 todos，`TodoDock` 显示完成计数；无 done/total 头部、新增/状态/移动/删除 diff 与早期基线比较。 |
| C07 | `grep/glob` 搜索卡 | 部分实现 | `parse_search_meta` 解析 `meta.shape`（matches/paths）、`total`/`truncated` 与 recovery 文本，`build_search_card` 结构化展示匹配行/路径列表与截断说明；无 meta 时回退原始输出；点击结果不打开预览。 |
| C08 | `web_search/web_fetch` 卡 | 部分实现 | `parse_web_meta` 解析 `meta.sources`/`answer`/`url`/`statusCode`/`truncated`，`build_web_card` 结构化展示来源列表（链接/摘要）、HTTP 状态、截断说明与外部打开按钮；无 meta 时回退 Markdown。 |
| C09 | `ask_user_question` 工具行 | 部分实现 | `cards.rs::render_tool_result_card` 对 `ask_user_question` 增加 Reopen，`SessionController::reopen_question` 调 `question/reopen`；快照仍通过可回答 dialog 展示，不是只读回看。 |
| C10 | 控制回执/结构化 details 卡 | 部分实现 | `build_control_details_card` 对 subagent/job/schedule/goal/workflow/lsp 展示顶层 JSON key/value 与原始结果；无嵌套/类型化 details 模型，未知工具仍回退通用卡。 |
| C11 | Auto-review 拒绝优先呈现 | 已实现 | `parse_auto_review_denial` 识别 `AutoReviewDeniedError`/`AUTO_REVIEW_DENIED` 并归一化原因；`build_auto_review_denial_card` 优先渲染拒绝卡（「Auto review 已拒绝」+「工具未执行。原因：…」），隐藏参数与原始输出；嵌套 PTC 子调用同样生效。 |
| C12 | 改动/交付卡与 review diff | 部分实现 | `workspace/changes` 事件生成 `DeliverablesBlock`，`build_deliverables_card` 显示 summary/diff；未调用 `workspace/changes` RPC 指定 turn/fileIndex，无 review tabs、stat 或 Present 文件按钮。 |
| C13 | Cordis 动态插件定义卡 run/stop | 缺失 | base 不挂载 Cordis define runner，GUI 无动态实例卡或 run/stop。 |
| C14 | 通用 Markdown/终端/diff/读取/搜索渲染器 | 已实现 | `render_markdown`、`build_code_block`、`build_card_shell` 为 M/C 卡提供统一 Markdown、SourceView、复制与展开能力。 |
| C15 | 下载/另存/图片复制 PNG/菜单 | 部分实现 | Files 行提供 Save As（`files/readBytes` + `GtkFileDialog.save`），Preview 提供 Copy PNG 与 Open Externally；无统一文件右键菜单或多选下载。 |
| C16 | 拖放字节传递 | 缺失 | 无 `GtkDropTarget`、拖放遮罩或文件转 prompt attachment。 |
| C17 | 在应用中打开 Workspace/文件 | 部分实现 | `ui::open_with` 提供 File Manager/Cursor/VS Code/Zed 等候选并按 PATH 探测，Open Folder 主按钮与 chevron 选择器可选择并持久化 `openWithApp`；Open Terminal 调 `desktop.rs::open_terminal_in_dir`；仍无文件 stat 与按文件类型的右键“打开方式”。 |
| C18 | 右键菜单框架 | 部分实现 | 消息文本共享 Popover 支持 Copy/Quote，代码视图支持 Copy Code；不是按目标类型异步解析的通用菜单框架。 |

## 5. A：审批与提问

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| A01 | 工具审批卡接管 composer | 部分实现 | `show_approval_dialog` 显示 toolName/callId/reason 与序列化 Details 展开，可选 displayReason/warning/detail 行，Allow Once/Reject/关闭均回传，fail-closed 保留；仍是模态 dialog，不接管 composer，无命令预览。 |
| A02 | 审批 Enter/Esc | 已实现 | `show_approval_dialog` 的 GtkEventControllerKey 将 Enter/KP_Enter 映射为 AllowedOnce、Escape 映射为 Rejected 并关闭；无复杂焦点优先级。 |
| A03 | 审批风险说明浮层 | 部分实现 | ApprovalRequestParams 新增可选 displayReason/warning/detail，dialog 在字段存在时显示对应行；bridge 当前 payload 通常不含这些字段，且不是 hover/focus popover。 |
| A04 | 通用提问卡/选项/自定义/翻页/跳过 | 部分实现 | 每题按页面展示 options（单选 group / 多选 CheckButton）、自由文本、Back/Next/Skip 与 Submit 汇总；仍无推荐项预选、inline 接管 composer 与单题独立提交。 |
| A05 | 提问倒计时/聚焦暂停/超时后作答 | 缺失 | 无 remainingMs、倒计时、聚焦暂停或超时后可答状态；base 不提供 attachWait 剩余时间。 |
| A06 | 迟到回答/草稿恢复/关闭重开 | 部分实现 | `question/reopen` 与 ask_user_question Reopen 已接线，关闭回 cancelled，重开快照展示；问题草稿未持久化，重开仍可交互而非只读。 |
| A07 | 已回答气泡与只读回看 | 已实现 | `user/message` 的 `source.kind=user-question-reply` 生成 `RenderBlock::QuestionReply`，右侧只读气泡显示问题摘要、真实 selected 标签与 View details JSON 展开；无答案时不伪造。 |
| A08 | Plan review 专用卡 | 部分实现 | `show_user_question_dialog` 检测 `UserQuestionIntent::PlanReview`，显示 detail Markdown，“同意执行”回 approve，“要求修改”回 cancelled；仍是模态 dialog，不接管 composer。 |
| A09 | Plan 模式徽章与退出 | 已实现 | composer 的 Plan mode 徽章由 follow projection 与 `plan/mode` 事件驱动；点击执行 `command/execute` 的 `plan off`，仅在 projection/事件确认后撤下徽章。 |
| A10 | 已提交计划卡/完整计划预览 | 部分实现 | `exit_plan_mode` 生成 `RenderBlock::PlanCard`，`build_plan_card` 渲染 Markdown；不自动打开右栏完整预览，历史分页不定位计划。 |

## 6. S：设置与模型

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| S01 | 设置外壳/配置表单/冲突保留草稿 | 部分实现 | `build_preferences_dialog` 使用 `AdwPreferencesDialog`；Host Settings 按 schema 生成 typed 字段表单并 `settings/mutate(expectedRevision)` 保存；`SettingsRevisionCache`（`ui/settings_cas.rs`）由 describe 播种、`settings.updated` 通知与 `-32020` 冲突的 `data.actual` 回写，连续保存不再冲突。 |
| S02 | 语言选择 | 部分实现 | General 页 Locale ComboRow 提供 System + 8 语言；`settings/describe` 有 locale namespace/字段时通过 `settings/mutate(expectedRevision)` 写入并回显当前值，否则禁用并标不可用；无 Rust 词典，界面文案不会即时翻译。 |
| S03 | 代码工作工具开关 | 已实现 | General 页 Developer Tools 写入 UiPrefs，`ConversationView::set_trajectory_visible` 控制 Trajectory tab 显隐，启动时恢复本地设置。 |
| S04 | 连接恢复/服务版本变化/版本行 | 部分实现 | `show_connection_lost`/`restart_supervisor` 提供重连，General 页显示 dsh/protocol 版本；版本值硬编码，无 release change 检测。 |
| S05 | 打开配置文件 | 缺失 | 无打开 profile/cordis 配置文件的动作。 |
| S06 | Models provider 行/编辑/排序/增删 | 部分实现 | Models 页动态调用 `model/catalog`，按 provider 展示 configured/not-configured 与模型列表，带 Refresh；无 provider 编辑、排序、增删或在线 discover。 |
| S07 | API Key 只写/校验/保存/删除范围 | 部分实现 | `AdwPasswordEntryRow` 写入 Secret Service 并调 `credentials/rotate`；保存前按上游 `apiKeyFailure` 规则校验（可打印 ASCII、拒绝空串/`NAME=value` 粘贴/引号包裹，空输入=保留现有 key），保存后经 `account/status` 探测并显示验证结果；无保存 spinner 和删除范围流程。 |
| S08 | 首启 DeepSeek 引导与版本说明 | 部分实现 | `app/src/ui/onboarding.rs::show_onboarding_dialog` 在 `credentials/status` 未配置时显示 key 表单与 Configure Later，保存后 inject；无 WelcomeNotice 版本确认与 settings 持久化。 |
| S09 | DeepSeek 账号登录/资料/余额/充值 | 部分实现 | Settings→Account 调 `account/status|profile|balance|sign-out`，错误显示 unavailable 而非零余额；浏览器 sign-in 明确标注不可用，无 watch、充值和配额弹窗。 |
| S10 | Agent loop 并行上限 | 已实现 | Host Settings 的 `settings/describe` 驱动表单为 agent-loop primitive schema 字段生成 Switch/Spin/Entry/Combo 并 `settings/mutate(expectedRevision)` 保存；namespace 缺失时不显示。 |
| S11 | 子代理深度/并行/模型设置 | 已实现 | Host Settings 为 subagent 与 subagent-model-selection 的 primitive schema 字段生成 typed 表单并按 namespace revision 保存；缺复杂嵌套字段。 |
| S12 | Web 搜索/获取 provider 与结果数 | 已实现 | Host Settings 对 web/web-search-deepseek/web-deepseek-api 的 schema 字段生成 typed 表单并保存；provider 列表本身仍受 G17 base 限制。 |
| S13 | 终端超时/输出上限 | 已实现 | Shell 页保留静态提示；Host Settings 对 bash-sandbox/pwsh-sandbox/shell namespace 的 primitive 字段生成可保存表单，namespace 缺失时不显示。 |
| S14 | 会话日志上传开关 | 已实现 | Host Settings 在 session-log-deepseek namespace 存在时按 schema 生成开关/字段并 `settings/mutate` 保存；不存在时显示不可用而非伪造。 |
| S15 | 默认权限与 `/permission` 会话选择 | 部分实现 | `Composer::permission_button` 与 `open_permission_picker` 调 `permission/get|set`，`permission.updated` 实时同步；默认权限设置和 `/permission` 命令弹窗未实现。 |
| S16 | Agent Preset 卡/默认值/YAML | 缺失 | base 不挂载 `AgentPresetRegistry`，协议未定义 preset methods，GUI 无 preset 卡。 |
| S17 | Goal 条带编辑/暂停/恢复/清除 | 已实现 | `ui/goal_bar.rs` 在 composer 上方显示 objective/phase/revision，follow 调 `goal/get`、`goal.updated` 实时更新；Edit/Pause/Resume/Clear 用 GoalRef CAS 调对应方法，无 goal 时隐藏。 |
| S18 | `/model` 与会话模型切换 | 部分实现 | `open_model_picker` 动态读取 `model/catalog`，显示 provider configured 状态与模型；选择调 `session/model/set`，follow 用 `session/model/get` 显示当前，失败回滚旧 model 并 Toast；无 reasoningEffort。 |
| S19 | 主题/字号/字体 | 部分实现 | Theme 立即应用到 StyleManager 并持久化；ui-theme namespace/字段存在时同步 `settings/mutate`；Font Size SpinRow 通过 GtkSettings 预览并本地持久化；无字体族选择与完整 Host theme token。 |
| S20 | 快捷键速查/录制/恢复 | 部分实现 | `win.show-help-overlay` 使用 `GtkShortcutsWindow` 显示固定键位；无搜索、录制、冲突检测、恢复默认与持久化。 |

## 7. P：插件与凭据

| 编号 | 功能点 | 状态 | 证据与剩余差异 |
|---|---|---|---|
| P01 | 插件列表/状态诊断/刷新 | 部分实现 | Settings→Plugins 调 `plugin/inventory` 渲染 bundles/plugins，Refresh 重新读取；组件行显示 fiberPhase（等待依赖/加载中/运行中/异常/卸载中）与 Host diagnostic；preset 分组仍缺。 |
| P02 | Bundle/组件启停与 restart-required | 部分实现 | `toggle_plugin` 调 `plugin/enable|disable`，restartRequired 显示 Banner，Restart Now 冷启动并 follow 活动会话；inventory 行本身无启停开关，也未恢复全部打开会话。 |
| P03 | 插件配置表单 | 缺失 | 无 bundle/row/namespace 动态配置表单。 |
| P04 | Bundle 详情文档与错误重试 | 已实现 | 详情按 `name@version` 缓存，调 `plugin/detail`（missing bundle 使用 npmFileSources）；成功渲染 Markdown，失败显示 Host 原因（过大/网络/凭据/Git/无效）与重试；README 图片渲染为占位 chip（协议未提供图片代理）；无 document 时显示“暂无详细介绍”。 |
| P05 | 添加插件与 inspect 预检 | 已实现 | Settings→Plugins 的 spec 行支持 npm name/local path/Git URL，Inspect 调 `plugin/inspect` 并显示 JSON，Install 调 `plugin/install`。 |
| P06 | 安装进度/日志/取消/恢复/构建审批 | 部分实现 | `plugin.install-state`/`plugin.install-log` 经 `SettingsDialogHandle` 更新阶段和日志，Cancel 调 `plugin/cancel-install`，Wait 方法已接；安装 RPC 丢失时进入 Unconfirmed 并自动 `wait-install` 对账、无结果时重读 inventory；missing bundle 可从包页/开关走 catalog 安装。进度不是独立模态，无 approvedBuilds 审批。 |
| P07 | 卸载/更新/版本检查与恢复 | 部分实现 | inventory 行提供 Remove 确认、Versions 与 Update 对话框，分别调 `plugin/remove|versions|update`；更新 RPC 丢失时刷新 inventory 并重查版本，Lost 提示以实际状态为准；版本检查与恢复结果卡已接，无独立更新输出流。 |
| P08 | 安装源选择/记忆/探测 | 部分实现 | Registry ComboRow 调用 `plugin/registries` 填充，选择持久化到 UiPrefs 并传给 inspect/install；无 registry 测速与推荐。 |
| P09 | 插件市场索引/搜索/分页/安装确认 | 缺失 | bridge 未定义 `plugin-market/*`，Settings 明确显示“插件市场依赖 Web Host，本客户端暂不支持”，不伪造空列表。 |
| P10 | 设置→内置插件只读清单/页面重试 | 部分实现 | inventory 只读清单支持名称搜索、Refresh 与 Retry（`plugin/retry`）；无 Loader 条目诊断、分组或页面模块重试。 |
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
| O06 | Worker/流式上传/receipt | 部分实现 | 文件/图片选择、拖放与剪贴板图片已接 `attachment/upload`，图片可进 prompt；上传是一次 base64 请求，无流式分块、进度或 receipt 恢复。 |
| O07 | 快捷键引擎/持久化 | 部分实现 | `main.rs` 设置 app/window accelerators，`setup_composer`/`setup_double_esc` 使用 `GtkEventControllerKey`；无自定义键位持久化或冲突管理。 |
| O08 | Locale 词典/fallback | 缺失 | 无 Rust i18n 词典、locale fallback 或动态文案处理。 |
| O09 | observable store/useResource | 部分实现 | `SessionModel`/`ConversationTranscript` 持有领域真相，controller 用 Tokio channel + `spawn_local` 更新 UI，terminal/files panel 持有会话资源状态；无统一 resource handle 与 release 生命周期。 |
| O10 | React renderer/Slots | 有意不移植 | 采用固定 GTK4/libadwaita 控件与 Rust 领域对象，不引入 React/Slot/JS runtime。 |
| O11 | Desktop 产品埋点 | 有意不移植 | 不新增产品遥测面。 |
| O12 | 会话后台 jobs 列表/输出/双击停止 | 已实现 | `ConversationView::set_jobs` 渲染 job roster，`follow_job` 调 `job/follow`，`IncomingNotification::JobOutput` 追加输出，`win.kill-job` 使用 `AdwAlertDialog` 二次确认后调 `job/kill`。 |
| O13 | 自动化计划任务 CRUD/运行/历史/时钟 | 已实现 | `SchedulesPage` 调 `schedule/catalog|create|edit|run|history|delete`，提供 New/Run/History/Edit/Delete 对话框，`schedule.changed` 刷新；无 delivery limits、暂停/恢复与目标会话时钟标记。 |
| O14 | PWA/manifest/子路径挂载 | 有意不移植 | 使用 `AdwApplication`、应用 id、`.desktop` 与 GApplication 激活，不实现 PWA/URL scope。 |
| O15 | 系统通知与提示音 | 部分实现 | GNotification 完成/后台通知与撤回保留；General 的 Desktop Notifications 开关持久化并同时约束 turn-complete、后台继续与关闭窗口通知；无提示音。 |
| O16 | 测试支撑包 | 已实现 | `app/tests/` 14 个 Rust 测试文件共 82 项，`bridge/tests/` 7 个 suite 共 39 项；产品窗口无测试入口。 |

## 9. G01–G29 当前缺口账本

| ID | 影响行 | 状态 | 证据与边界 |
|---|---|---|---|
| G01 | N03/N15 | implemented | `bridge/src/bridge.ts` 实现 `workspace/list|create|rename|remove`；GUI `SessionController::load_workspaces` 等已接线。 |
| G02 | N10 | implemented | `libadwaita/session/rename` 与 `SessionController::rename_session` 已接线。 |
| G03 | N12/M09 | implemented | `libadwaita/session/fork` 与 `SessionController::fork_session` 已接线。 |
| G04 | N03/N13 | implemented-partial | GUI 已把侧栏搜索接到 `session/search` 并做 250ms debounce；bridge 返回 `fullText:false`，base 的 `session-query-sqlite` 以 `openAt: never` 挂载（`packages/bundle/base/cordis.patch.yml` 第 157–161 行），仅标题/cwd 元数据。 |
| G05 | N17 | implemented-partial | bridge `terminal/create|list|input|output|close` 有真实 dsh 测试；无 resize RPC，GUI 使用 fallback TextView/Entry 而非 VTE。 |
| G06 | N18/N19/C02/C07/C12/C15/C17 | implemented | `files/list|stat|read|readBytes` 已实现，`FilesPanel`/`PreviewPanel` 已消费。 |
| G07 | N19 | base-blocked | `ctx.officeToPdf` 属于 web-app 文档预览组合；base 不提供，GUI 回退系统默认应用。 |
| G08 | N21 | implemented-partial | bridge `subagent/list|prompt|interrupt` 已实现；GUI 接 list/open/interrupt，未接子代理 prompt 续接。 |
| G09 | N22 | base-blocked | `packages/bundle/base/cordis.patch.yml` 第 366–367 行插入 agent-team，但本机探针 `ctx.get('agentTeams')` 为 undefined，bridge 返回 `-32040`。 |
| G10 | M02/M18 | implemented | `session/queue/list|edit|remove|steer` 与 `session.inbox` 已实现并接入 QueueDock。 |
| G11 | M07/M21/M28/C03/O06 | implemented-partial | bridge `attachment/upload|read`（含完整图片 ref 的 Host 授权读取）与 prompt image/file 已实现；GUI 已接选择、拖放、剪贴板、用户图片画廊灯箱、read_image 缩略图、文件保存、按会话图片缓存与 composer 附件暂存；仍无预上传进度与流式分块。 |
| G12 | M11/M12/M22/S17 | implemented-partial | `session/projections` 的 tokenUsage/contextPressure/contextBreakdown/goal 已消费；`packages/bundle/web-app/cordis.patch.yml` 第 98–104 行的 `turnOutline`/`sessionStats` 不属于 base，协议明确省略，不伪造。 |
| G13 | M25 | implemented-partial | GUI Trajectory 已消费 2000 条 raw SessionEvent 与 follow/page 快照，并实时追加 `session.event`；未订阅 `tool-result/watch`，不承诺原始未落盘中间值。 |
| G14 | M26 | implemented | `message-feedback/list|put|delete` 与 `session-feedback/record` 已实现；GUI 以持久 message id 接线 thumbs、七类原因/备注对话框、撤回刷新与冲突权威项对账。 |
| G15 | M16/A09 | implemented | `command/list|execute`、`skill/list`、`reference/list` 已实现并接入 trigger popups；`/plan off` 参数语义仍受上游命令目录限制。 |
| G16 | S01/S03/S06/S10–S13/P03 | implemented | `settings/describe`、`settings/mutate(expectedRevision)` 与 `settings.updated` 已实现；GUI Settings Editor 与 Host Settings typed 表单已消费，复杂嵌套/secret 字段不自动生成。 |
| G17 | S06/S12 | implemented-partial | GUI 已消费 `model/catalog` 动态模型目录；`model/discover` 与 `web-search/providers` 仍未接，base 仅 `llm-pi-ai` 可 discovery 且可能返回 `-32044`，web provider 列表可为空。 |
| G18 | S09/M13 | implemented-partial | bridge 提供 status/profile/balance/sign-out；`packages/credentials/deepseek-account-platform` 的浏览器 OAuth 依赖 Host webServer，base-only 不提供 start-sign-in/watch/quota 动作。 |
| G19 | S15 | implemented | `permission/catalog|get|set` 与 `permission.updated` 已实现，composer permission picker 已接线。 |
| G20 | S16 | base-blocked | `AgentPresetRegistry` 仅 `packages/bundle/web-app/cordis.patch.yml` 第 598 行挂载，preset 声明会重复挂载 base 工具行；协议未定义 `agent-preset/*`。 |
| G21 | S17 | implemented | `goal/get|edit|pause|resume|clear` 与 `goal.updated` 已实现；GUI GoalBar 已接 get/updated/edit/pause/resume/clear（objective only）。 |
| G22 | P04–P08 | implemented | `plugin/detail|inspect|install|cancel-install|wait-install|remove|versions|update|registries` 与 install-state/log notifications 已实现；GUI 已接 detail README、registries selector、versions/update/remove 与 cancel，仍无 approvedBuilds 构建审批。 |
| G23 | P09 | base-blocked | `@deepseek-ai/dsh-plugin-market`、`dsh-plugin-market-index` 与 `dsh-experimental-plugin-market-bundle` 不在 base 组合，bridge 无 `plugin-market/*`；GUI 显示不可用而非空市场。 |
| G24 | P01/P03/P10 | implemented-partial | GUI 使用 `plugin/inventory`，已接名称搜索、Refresh、Retry、组件 fiberPhase/diagnostic 展示与详情；仍无 preset 分组与页面模块重试。 |
| G25 | O12 | implemented | `job/list|follow|kill` 与 `job.output` 已实现并接入 header Jobs popover。 |
| G26 | N24/O13 | implemented-partial | `schedule/catalog|create|edit|run|history|delete` 与 `schedule.changed` 已实现并接入 SchedulesPage；未定义 delivery-limits/pause/resume。 |
| G27 | C13 | base-blocked | `cordis_define` 动态实例控制属于 `packages/extensions/ui-cordis`/web-app runner，base 不挂载；协议不定义 `cordis-control/*`。 |
| G28 | A03/A05/A06/C09 | implemented-partial | bridge 保存 `user-question/ask` 快照并支持 `question/reopen`；GUI 已渲染可选 approval displayReason/warning/detail，但不伪造 remainingMs/attachWait，倒计时与迟到回答仍缺。 |
| G29 | P11/S09 | implemented | `credentials/records/list|read|modify|delete` 已实现，GUI 只展示 metadata，不回读 secret。 |

## 10. 与 DSH Web 的功能差异

| # | 剩余差异 | 用户可见影响 | 保留原因 | 应用内 workaround |
|---:|---|---|---|---|
| 1 | 附件上传无分块进度/失败重试，无按会话图片缓存 | 大文件上传缺少进度反馈，图片重复读取 | UI scope；bridge `attachment/upload|read` 已是单次 base64 请求 | 小文件直接重试；图片在消息/工具卡内按需读取 |
| 2 | Trajectory 缺时间线条与 turn outline，未订阅 `tool-result/watch` | 无法按时间缩放或查看未落盘中间工具值 | UI scope + G13 派生边界 | 使用事件表、搜索、JSON 检查器与 page 快照 |
| 3 | 无生成速度与每轮 token 面板 | 不能分析每轮成本；上下文占用已按 `projectedTokens/pressureTokens` 与 `contextWindow` 显示百分比，并展示 system/tools/messages 分解 | G12 base 省略 sessionStats/turnOutline | 查看会话 token 胶囊与 Context 弹层分解 |
| 4 | 审批仍是模态 dialog，不接管 composer，无命令预览 | 审批时布局与焦点流程和 Web 不同 | UI scope；displayReason/warning 依赖 bridge payload | 鼠标或 Enter/Esc 完成 Allow Once/Reject |
| 5 | 提问无倒计时/聚焦暂停/迟到补答，问题草稿不持久化，推荐项不预选 | 长时间等待与跨刷新恢复体验弱于 Web | G28 remainingMs/attachWait base 限制 + UI scope | 问题选项、分页、Skip、自由文本与可重开快照可用 |
| 6 | Locale 选择不翻译现有 GTK 文案；Host 表单仅 primitive schema 字段，复杂/嵌套/secret 字段跳过；无字体族选择 | 语言切换不会即时改界面文案，部分高级设置不可编辑 | UI scope + 无 Rust i18n 词典；G16 schema 已可用 | 主题/字号/Developer Tools/通知可持久化；其余用 JSON Settings Editor |
| 7 | Models 无 provider 编辑/排序/增删与在线 discover | 只能使用 catalog 返回的 provider/model，不能管理自定义 provider | UI scope + G17 discovery base 限制 | 使用 API key 与动态 model picker |
| 8 | DeepSeek 浏览器 OAuth、账号 watch、充值跳转与配额恢复弹窗缺失 | 不能从原生窗口完成登录或处理欠费恢复 | G18 base-blocked；OAuth 依赖 Host webServer | 使用 API key onboarding；账号页读取状态/资料/余额并退出 |
| 9 | approvedBuilds 构建审批、Loader 诊断与页面模块 retry 仍缺 | 市场浏览、index 切换、registry 测速与安装源持久化已接入；不能批准构建脚本，诊断粒度较浅 | G23/G24 base 与 UI scope | 使用 npm/Git/local spec、市场 Discover、Detail/Retry |
| 10 | Agent Team、Agent Preset、Cordis define run/stop 缺失 | 不能管理 Team、preset 或动态 Cordis 实例 | G09/G20/G27 base-blocked | 使用默认 profile 与现有 session/permission 控制 |
| 11 | Workflow 专用节点与成员子会话导航缺失 | 多阶段 workflow 仅显示通用事件 | UI scope；session events 已可 follow | 从 Subagents 区打开已知 child session |
| 12 | PDF/Office 预览与完整文档渲染缺失 | 交付文件不能在内嵌预览中查看 PDF/Office | G07 base-blocked + UI scope | Files 行 Save As、Preview 外部打开；系统默认应用处理 |
| 13 | 会话全文搜索仍为标题/cwd 元数据；手动排序仅支持同组内拖拽 | 大量历史消息时检索效率低于 Web；跨工作区不能靠拖拽改变归属 | G04 base `fullText:false` + UI scope | Pin 置顶、Active/Archived/All 过滤、标题/cwd 搜索、手动排序 |
| 14 | 右栏无分屏、浮动、全屏 | 多资源并排布局能力弱于 Web | UI scope | 每会话 tab/可见性已持久化，可手动切换固定标签 |
| 15 | 终端是轮询 fallback，不是 VTE PTY | 无 shell 选择、重命名、resize、主题终端 | UI scope + G05 无 resize RPC；构建无 VTE 依赖 | Entry 发送命令，TextView 查看有界回滚并 Close |
| 16 | Markdown 流式尾部增量解析与超大表格虚拟化缺失 | 超长流式 Markdown 会重复重建，超大表格无虚拟行 | UI scope | 定稿后完整渲染；表格可横向滚动 |

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

当前实现已覆盖核心会话、可靠 transcript、Queue/Steer、审批与提问、Workspace、Files/Preview/Save As、Terminal fallback、Schedules、Jobs、插件生命周期与市场、Settings typed forms/CAS、GoalBar、Trajectory 检查器、附件、上下文/usage、credential records 与账号只读视图。2026-11-08 补齐了持久会话冷列表、工作区成员关系、持久归档、按会话草稿/视图、修复消息流 revision/attempt 防护与历史缺口修复、打开旧会话时加载最近 200 条历史、上下文 meter 字段修正、审批协议可选 callId / 本地化 displayReason、GFM 表格/任务列表/脚注与 `path:line:column` 链接。相对 DSH Web，主要剩余差距集中在附件进度/缓存、Trajectory 时间线、每轮统计、OAuth、构建审批、重型文档预览、VTE 终端与完整国际化。

按 132 行统计（2026-11-08 审计前的基线）：**37 已实现、72 部分实现、14 缺失、8 有意不移植、1 有意简化**。因此不能宣称“功能与 DSH Web 无异”；上述 16 项差异与 G07/G09/G20/G23/G27 的 base-blocked 边界是后续验收和范围决策的明确输入。

## 13. 2026-11-08 对等性审计与后续排序

六个子系统（N/M/C/A+S/P/O）共记录 124 条原始差距，去重后按“用户可见影响 × 可验证性”排序出 12 个行动项：

| 排名 | 主题 | 主要缺口 |
|---:|---|---|
| 1 | 消息流与历史 | 已完成最近窗口加载、缺口修复、attempt/revision 防护、UI 块替换/删除对账与流式尾部增量解析 |
| 2 | 审批/提问/计划 | 窗口级模态改为会话内 composer 接管；键盘仲裁、命令预览、推荐项预选、倒计时 |
| 3 | 异步响应会话隔离 | SessionFollowed/History/Projections/Feedback/Terminal/Files 已加会话/代数守卫；dock 布局与反馈映射仍需按会话隔离 |
| 4 | 重连与启动诊断 | 自动退避重连、恢复活动会话、保留 InitializeResult、插件级启动诊断 |
| 5 | 统计与生命周期卡片 | 每轮速度/缓存面板、Continue 可达性（已修事件/状态触发）、compaction/retry/error 专用卡片、运行计时 |
| 6 | 工具卡 | 结构化 read/edit/grep/glob/web 卡、auto-review 拒绝原因、消息图片与文件块、PTC 调用树 |
| 7 | 文件与 Markdown | 文件树/面包屑/刷新、预览格式（PDF/Office 受 G07 限制）、行内代码与链接细节（GFM 已启用） |
| 8 | 设置与凭证 | 设置 CAS revision 回写、Ctrl+, 路由（已修）、凭证写后校验、内置插件 inventory、子智能体模型授权 |
| 9 | 滚动与流式渲染 | 尾部跟随/回底按钮/轮次导航、流式 Markdown 增量、fork 边界、四档 transcript 模式 |
| 10 | 插件管理 | missing bundle 安装流程、安装/更新 RPC 丢失后的对账、组件诊断、README 图片与来源错误 |
| 11 | 密钥与附件 | 完整 secret 格式掩码、feedback message id、会话图片缓存、附件暂存与队列附件 |
| 12 | 布局与周边 | 侧边栏/右栏拖拽宽度持久化、分组/平铺排序、计划时钟、通知路由、打开方式与应用选择器 |

仍需 Host 侧能力（bridge RPC / profile mount / 原生依赖）的延后项：完整存档确认与分页、全文搜索（`session-query-sqlite` 目前 `openAt: never`）、VTE 终端与多标签 dock、PDF/Office 预览、Agent Team 与子智能体层级、请求关联提交与分块附件、完整 Trajectory ledger、变更文件多文件 diff、定时提问迟到回答、Provider CRUD/OAuth、第三方插件声明式配置、后台任务 unfollow 与完整 i18n/快捷键录制。

### 13.1 实施进度（2026-11-09）

- 项 1 消息流与历史：最近窗口加载、缺口修复、attempt/revision 防护（`c5d0854`）与流式尾部增量解析（项 9）已完成；本轮补齐 UI 块替换/删除：`ConversationView` 以 block fingerprint 比较已渲染前缀，pending user 持久化替换（seq 由 -2 变为正数）与 abandoned live 移除（数量减少）都会清除旧控件并按当前 blocks 重建，追加仍走增量路径；live 定稿时先同步最终文本，messageId 迟到时向已建动作行补挂点赞/点踩/Feedback。验证：新增 5 个 conversation lib 单测（pending 文本/提交指纹、live 文本/attempt 指纹、前缀追加/替换/删除判定、pending 替换与 abandoned 删除的 transcript 端到端指纹变化），完整 39 个测试二进制通过。未验证：真实模型流式下 abandoned/替换的视觉效果（测试 key 401、无 live agent）。
- 项 2 审批/提问/计划：composer 接管卡片、键盘仲裁、命令预览、推荐项预选、计划审查、会话路由与 fail-closed 已完成（`81f1d2c`）；倒计时/慢慢回答依赖 Host `remainingMs`/`attachWait`（G28），延后。
- 项 3 异步响应会话隔离：主要 UiMessage 已加会话/代数守卫（`c5d0854`）；dock 布局与反馈映射的按会话隔离仍待补。
- 项 4 重连与启动诊断：指数退避自动重连、恢复活动会话、保留 InitializeResult、console 诊断环形缓冲与设置页展示已完成（`644371b`）。
- 项 5 统计与生命周期卡片：每轮 token/速率/耗时 Stats 面板、运行计时器、compaction/turn-error/max-tokens/llm-retry 卡片已完成（`8cdc5f5`）。
- 项 6 结构化工具卡：auto-review 拒绝优先卡（参数隐藏、原因归一化）、`grep/glob` 结构化 meta 卡（matches/paths/total/truncated/recovery）、`web_search/web_fetch` 结构化卡（来源列表、HTTP 状态、截断说明、外部打开）、PTC 子调用树（嵌套渲染，含嵌套 sub-call 的独立专用卡）、助手消息图片/文件块画廊已完成（`5318ac9`）；`parse_search_meta`/`parse_web_meta`/`parse_auto_review_denial`/PTC 组装等由 14 个单测覆盖。视觉截图验证受环境阻塞（headless sway 重启后新 GTK 窗口 VK surface lost、grim 无 screencopy；且本机 `DEEPSEEK_API_KEY` 已 401 失效无法发起真实模型轮次）。
- 项 7 文件与 Markdown：文件面板改为层级树（目录优先排序、按需展开、折叠保留、消失目录剪枝、`FileTreeModel` 8 单测）、面包屑与刷新、预览定位、会话切换重置已完成（`7648161`）；同时修复右栏 dock 被 paned 挤到零宽不可见的问题（dock 最小宽度 340、`shrink_end_child(false)`）。GFM 已启用；PDF/Office 预览受 G07（Host 无 `officeToPdf`）限制延后。视觉验证：headless sway 中 Files tab 截图确认树渲染（docs/src/README.md 排序正确、面包屑、刷新后列出），目录行展开因 AT-SPI 不给 ActionRow 暴露 action 且无指针注入而未能截图验证。
- 项 8 设置与凭证：`SettingsRevisionCache` CAS revision 回写（describe 播种、`settings.updated` 通知回写、冲突 `data.actual` 回写，设置页与 locale/theme 保存共用）、API key 字符集校验（settings 与 onboarding）与保存后 `account/status` 验证探测已完成（`30dd7ec`）；插件 inventory 的 fiberPhase/diagnostic 行级诊断此前已实现（P01 剩 preset 分组）；子智能体模型授权由 S11 typed 表单覆盖，运行期 `subagent/model-selection-policy` 为 Host log-only 事件且 GUI 无子代理模型选择器。验证：`settings_cas_tests` 5 单测 + 设置对话框打开截图（describe 后显示 "7 namespaces available"），Models 页交互输入因 headless a11y 无法翻页未验证。
- 项 9 滚动与流式渲染：`ScrollFollow` 尾部跟随（32px 回底阈值、上滚暂停、回底恢复、会话切换重置）、浮动回底按钮、右侧轮次轨道（已加载轮次、活动轮次按视口上沿高亮、点击跳转、提示预览、≥2 轮显示）、流式 Markdown 稳定前缀/尾部分离（按空行切分且拒绝未闭合代码围栏，live assistant 仅重绘尾部）、`session/end-seed` fork 边界分隔、fork 仅出现在最后一条已定稿 assistant 消息、四档 presentation policy（步骤分组 collapsed/history/none、运行过程明细、已结思考首行预览）与定稿时补齐动作行已完成（`0fd9937`）；同时修复 trajectory `ColumnView` 未放入 `ScrolledWindow` 导致窗口最小高度随事件数增长的问题。验证：新增 21 个单测（scroll 5、stream 7、turn nav 5、presentation 4）+ 1 个 fork 边界 transcript 单测，35 个测试套件全部通过；headless sway 截图确认 fork 分隔条、8 轮轨道与活动轮次高亮、点击 Turn 1 跳到首轮并出现回底按钮、回底后按钮隐藏且活动标记回到第 8 轮。未验证：真实模型流式（本机 `DEEPSEEK_API_KEY` 仍 401）、完整 turn outline（G12）、`collapseTiming`/`foldedReplies` 与整轮外折叠时机。
- 项 10 插件管理：missing bundle 安装（`catalog_install_plan` 仅接受 missing + installTarget，排除 unsupported-platform/只读；包页“安装”按钮与启用开关自动改走 catalog 安装，registry target 带 saveExact）、安装 RPC 丢失自动 `wait-install` 对账且无结果时重读 inventory、更新 RPC 丢失后刷新 inventory 并重查版本、组件行显示 Host diagnostic、详情按 `name@version` 缓存并在失败时显示原因与重试、README 图片占位 chip 已完成（`9f36a76`）。验证：新增 5 个 model 单测（catalog 规则、detail key/version、diagnostic、图片占位）+ 1 个安装丢失对账 reducer 单测，35 个测试套件通过；headless 截图确认插件页与“自动授权审查”详情页（组件行、来源信息、README 图片占位）。未验证：真实 missing bundle 的安装按钮点击（测试 profile 中无 missing bundle）、真实安装/更新断线对账（无可用 API key，且不触发网络安装）。
- 项 11 密钥与附件：`secret_mask` 完整移植上游格式表与掩码规则（PEM 块保留 BEGIN/END 行、URL 仅遮密码并按码点计数、前缀凭据保留 8 字符、重叠候选合并）；feedback 使用 `assistant/message` 持久 messageId，点赞/点踩撤回与七类原因/备注对话框、`version-conflict` 权威项对账；`ImageCache` 按会话+附件合并读取并在 release/archive 清理，bridge `attachment/read` 支持完整 `ImageAttachmentRef` 走 Host `readImage`；composer 附件暂存 rail（图片预览、文件卡、移除，按会话保留到成功回显后释放），文件提交经 `admitEncodedFile` 成为 durable ref，QueueDock 显示图片缩略图与文件 chip 已完成（`ae00389`）。验证：新增 17 个 secret mask 单测、5 个 image cache 单测、3 个 queue attachment 单测与 2 个 transcript message-id 单测，35 个集成测试二进制全部通过；bridge `tsc --noEmit`、Biome 与 66 个 node 单测通过；headless 截图确认附件暂存 rail 的图片预览、文件名与移除按钮。未验证：附件发送后的真实回显与队列附件视觉（测试会话无 live agent，prompt RPC 被 Host 拒绝）、模型请求中的文件块消费（无可用 API key）、真实 `version-conflict` 对账（需并发写反馈）。
- 项 12 布局与周边：侧边栏右侧 6px 拖拽手柄（264–420px）与右栏 paned 拖拽宽度均写入 `ui-prefs.json` 的 `sidebarWidth`/`dockWidth`；右栏宽度仅在指针按下的拖拽中保存，程序化恢复/窗口尺寸变化不再用被布局钳制后的实际宽度覆盖偏好，恢复时经 `layout::dock_width` 按总宽与 70% 上限钳制。侧栏分组支持 Grouped/Flat，Manual 排序使用持久化 `sessionOrder` 并支持组内 DragSource/DropTarget 重排；`schedule/catalog` 的 Active 任务按其 target 与 `involvedSessionIds` 映射到侧栏时钟图标（running/unread 时不显示）；turn-completed/background 通知改为 `app.open-session` 带 sessionId，应用级 action 在已有窗口与新建窗口两条路径都 follow 目标会话；Open Folder 支持按 PATH 探测的候选应用列表、chevron 选择器与 `openWithApp` 持久化。验证：新增 `dock_width` 钳制、手动排序、计划时钟条件、Active 计划会话映射共 7 个单测，39 个测试二进制全部通过；headless 截图确认侧栏按保存值 360px 恢复、平铺列表无分组头、宽窗口下右栏按保存值 450px 应用（切隐藏/显示后仍为 450，偏好未被钳制值覆盖）、`app.open-session` 切换窗口标题与选中行到 hello-from-a、候选应用菜单显示 File Manager/Visual Studio Code/Zed 且选择 Zed 后重启主按钮变为 “Open in Zed”。未验证：鼠标拖拽侧栏/右栏与会话行的真实手感（headless seat 无输入设备）、计划时钟视觉（测试 profile 无 Active 计划，AT-SPI 打开新建表单不稳定）。另发现启动时初始会话 transcript 尚未加载完成就切换会话会让 GTK 主线程忙循环（`f162c46` 的 HEAD 二进制同样复现，非本项引入），需单独立项修复。

- 模型链路根因修复：bridge catalog 默认 provider 改为 `agentDefaultModel` 校验值（`deepseek-official`），App 采纳目录默认并纠正失效会话；BridgeCredentials 从 `*_API_KEY` 环境变量取种（`81f1d2c`）。
