# dsh-libadwaita 对 DSH Web 功能验收对照

日期：2026-11-06  
基线：`docs/dsh-web-feature-inventory.md` 与 `docs/web-ui-map.md` 的 N/M/C/A/S/P/O 编号，共 132 行。  
判定只依据本仓库当前代码接线，不依据映射表中的计划或上游 Web 的理论能力。

## 0. 验收口径与全局启动缺口

- `已实现`：代码中可找到对应用户功能及关键语义，证据列给出文件与方法。
- `部分实现`：已有真实接线，但存在明确功能缺口；证据列同时给出已实现部分与缺口。
- `缺失`：未找到有效接线。缺口分为 `Gxx`（bridge 缺口）、`GUI-only`（无需新增 bridge）、`上游能力不存在`（上游没有能力）。本轮没有仅因“上游能力不存在”而判缺失的行。
- `有意不移植` / `有意简化`：仅用于 `docs/web-ui-map.md` 明确允许的项目。

**全局启动缺口（不计入每行重复扣分）**：`app/src/main.rs::main` 当前只创建 `MainWindow` 与 `SessionController`，没有创建 `HarnessSupervisor`、执行 `HarnessSupervisor::start()`、调用 `SessionController::set_supervisor()` 或 `SessionController::set_connection()`。因此下表评价的是各功能的代码接线完成度；真实端到端运行前，必须先补齐这条 GUI 启动链路。`app/tests/supervisor_dsh_tests.rs::test_real_dsh_lifecycle_and_session_create` 已证明 supervisor、握手、session/create、停止与重启在真实 dsh 上可用。

## 1. 状态汇总

| 区域 | 已实现 | 部分实现 | 缺失 | 有意不移植 | 有意简化 | 合计 |
|---|---:|---:|---:|---:|---:|---:|
| N 会话与导航 | 0 | 16 | 11 | 1 | 0 | 28 |
| M 消息与流式 | 1 | 13 | 13 | 0 | 1 | 28 |
| C 工具与展示卡片 | 1 | 10 | 7 | 0 | 0 | 18 |
| A 审批与提问 | 0 | 5 | 5 | 0 | 0 | 10 |
| S 设置与模型 | 0 | 10 | 10 | 0 | 0 | 20 |
| P 插件与凭据 | 0 | 3 | 9 | 0 | 0 | 12 |
| O 其他外围 | 1 | 3 | 5 | 7 | 0 | 16 |
| **总计** | **3** | **60** | **60** | **8** | **1** | **132** |

已实现行：M03、C14、O16。按当前代码，离“功能与 DSH web 无异”仍有明显距离，不能宣称验收通过。

## 2. N：会话与导航

| 编号 | 功能点 | 状态 | 证据与缺口分类 |
|---|---|---|---|
| N01 | 三栏 AppFrame 与 shell 座 | 部分实现 | `app/src/ui/window.rs::MainWindow::new` 使用 `AdwOverlaySplitView`、`GtkPaned` 与 `RightDock`；`app/src/ui/right_dock.rs::RightDock::new` 提供右栏。缺 56px 折叠轨、宽度优先级与窄屏规则，属 GUI-only。 |
| N02 | 左侧栏品牌/New Session/面板/设置/折叠轨道 | 部分实现 | `app/src/ui/sidebar.rs::Sidebar::new` 有品牌、New Session、搜索、列表及 Plugins/Schedules/Settings/Account 行；折叠轨道未实现，属 GUI-only。 |
| N03 | Workspace/Session 列表与排序/归档过滤 | 部分实现 | `app/src/ui/sidebar.rs::load_sessions` 动态生成会话行，`app/src/ui/controller.rs::archive_session` 归档。Workspace 空目录/层级/重命名/删除缺 G01；Pin、排序与归档筛选缺 GUI-only。 |
| N04 | 新建会话/复用 blank/命名创建 | 部分实现 | `app/src/ui/controller.rs::prompt_new_session_portal` 与 `create_session` 选择 cwd 后 create+follow；空会话复用、命名与 preset 选择未实现，属 GUI-only。 |
| N05 | 打开并恢复既有会话 | 部分实现 | `app/src/ui/window.rs::set_controller` 的 `select-session` action → `SessionController::follow_session` → `UiMessage::SessionFollowed`；尚未调用 `session/page` 加载更早历史，属 GUI-only。 |
| N06 | 启动时恢复当前 Session/Workspace | 缺失 | GUI-only：`app/src/main.rs::main` 没有保存/恢复当前 session id 或启动 follow；`session/list` 与 `session/follow` 已可直接使用。 |
| N07 | 官方品牌与首页鲸鱼层 | 缺失 | GUI-only：当前只有 `Sidebar::new` 的 `dsh` 文本和 `ConversationView::new` 的 welcome 卡片，没有官方图标/鲸鱼动画。 |
| N08 | “关闭”当前会话视图 | 缺失 | GUI-only：协议已有 `libadwaita/session/release`，但 controller 无 release 调用，也没有关闭会话视图动作；窗口关闭不能替代该功能。 |
| N09 | 归档/取消归档，活动会话二次确认 | 部分实现 | `app/src/ui/controller.rs::archive_session` 调 `session/archive(archived:true)`，`Sidebar::load_sessions` 提供 Archive 菜单；缺活动会话确认、取消归档与归档筛选，属 GUI-only。 |
| N10 | 手动重命名 | 缺失 | G02：没有 `libadwaita/session/rename`，也没有重命名对话框或行菜单。 |
| N11 | 自动标题 | 部分实现 | `app/src/ui/controller.rs::setup_ui_receiver` 处理 `IncomingNotification::SessionTitle`，更新 `WindowTitle` 与会话标题；侧栏行没有同步绑定，属 GUI-only。 |
| N12 | Fork 会话 | 缺失 | G03：无 fork RPC 与消息轮次分支菜单。 |
| N13 | 会话元数据/内容搜索 | 部分实现 | `app/src/ui/sidebar.rs::Sidebar::new` 有 `GtkSearchEntry`，但未接过滤逻辑；本地标题过滤缺 GUI-only，全文检索缺 G04。 |
| N14 | 行菜单、Pin、Archive、Toast | 部分实现 | `Sidebar::load_sessions` 提供 Archive popover，`archive_session` 发送 Toast；Pin 与本地排序未实现，属 GUI-only。 |
| N15 | Workspace 创建/层级/重命名/删除与目录选择 | 部分实现 | `prompt_new_session_portal` 使用 `GtkFileDialog.select_folder`，`create_session` 在新 cwd 创建会话；Workspace 持久操作缺 G01，草稿迁移与旧视图 release 缺 GUI-only。 |
| N16 | 右停靠栏框架 | 部分实现 | `app/src/ui/right_dock.rs::RightDock::new` 有 `AdwTabBar`/`AdwTabView` 与 Terminal/Files/Preview 三页；没有 split/fullscreen、每会话 tab 状态或真实资源内容，属 GUI-only。 |
| N17 | 终端 tab | 缺失 | G05：右栏 Terminal 目前只是 `adw::StatusPage` 占位，无交互终端、shell 选择、恢复或关闭进程。 |
| N18 | 文件 tab | 缺失 | G06：Files 页只是占位，无 workspace 文件树、stat 或刷新。 |
| N19 | 文档预览 tab | 缺失 | G06/G07：Preview 页只是占位，没有真实文件读取、行定位、PDF/Office 预览。 |
| N20 | 浏览器 tab | 有意不移植 | 项目禁止内嵌 WebView；`app/src/ui/desktop.rs::open_path_in_default_app` 使用系统默认应用作为原生替代，不实现 iframe/browser tab。 |
| N21 | 子代理目录/血缘/transcript/@引用 | 缺失 | G08：无子代理目录、血缘、prompt/interrupt 接线。 |
| N22 | Agent Team 开关与共享任务板 | 缺失 | G09：无 team 开关或共享任务投影。 |
| N23 | 插件面板入口 | 部分实现 | `app/src/ui/window.rs::MainWindow::new` 的 `show-plugins` action 切换 `ConversationView::select_panel("plugins")`；当前页面为占位，完整列表/诊断缺 G24，基础 `plugin/list` 尚未接入。 |
| N24 | 自动化任务页入口 | 部分实现 | `show-schedules` action 切换占位 schedules 页；真实 CRUD/历史/时钟缺 G26。 |
| N25 | Workflow 运行节点与子会话导航 | 缺失 | GUI-only：无 workflow 专用节点或成员导航；`session.event` 与 `session/follow` 已可支撑后续实现。 |
| N26 | 会话状态点/完成未读 | 部分实现 | `SessionController::running_sessions`、`handle_status_notification` 与 `desktop.rs` 通知已记录运行状态；侧栏状态点、未读粗体与待处理交互仲裁未实现，属 GUI-only。 |
| N27 | 连接状态与手动重连 | 部分实现 | `app/src/connection.rs::BridgeConnection::state` 可查询状态，`SessionController::restart_supervisor` 可冷重启；没有连接状态 pill/banner 或手动重连按钮，属 GUI-only。 |
| N28 | 重连状态修复 | 部分实现 | `app/src/supervisor.rs::GenerationTracker::record_generation` 记录代际，`restart_supervisor` 后重新 follow 活动会话；没有自动掉线重连、同代际 page 修复与全部打开会话 rebaseline，属 GUI-only。 |

## 3. M：消息与流式

| 编号 | 功能点 | 状态 | 证据与缺口分类 |
|---|---|---|---|
| M01 | 事件装配/历史前插/settlement | 部分实现 | `app/src/transcript.rs::ConversationTranscript::{apply_baseline,apply_stream_frame,apply_event}` 实现 baseline、live 块、序号、重复事件、committed/abandoned；`app/src/ui/conversation.rs::update_transcript` 渲染。历史前插、page 缺口修复未实现，属 GUI-only；`app/tests/transcript_tests.rs` 覆盖主流程。 |
| M02 | Chat 用户消息/steering/回显 | 部分实现 | `transcript.rs::apply_event` 生成 `RenderBlock::UserMessage`，`message_renderer.rs::build_user_message` 右对齐显示；`SessionController::send_prompt` 在 Host 确认前清空输入，没有 pending/失败恢复，steering 未接；完整队列缺 G10。 |
| M03 | Assistant 文本/思考/工具 delta、定稿/中断 | 已实现 | `transcript.rs::{apply_baseline,apply_stream_frame,apply_event}` 聚合 text/reasoning/tool delta 并处理 committed/abandoned；`message_renderer.rs::{build_assistant_message,AssistantLiveHandle}` 更新 live 文本与 reasoning；`app/tests/transcript_tests.rs::{test_normal_streaming_and_commit_flow,test_abandoned_stream_flow,test_reasoning_deltas_accumulation}` 验证无残影。 |
| M04 | Markdown/GFM/表格/脚注/安全链接 | 部分实现 | `app/src/ui/message_renderer.rs::render_markdown` 支持段落、标题、列表、引用、行内样式、代码与基础链接；表格、脚注、尾部增量解析与链接 scheme 白名单激活未实现，属 GUI-only。 |
| M05 | 代码高亮/复制/换行/行号/流式 fence | 部分实现 | `message_renderer.rs::build_code_block` 使用 `sourceview5::View`、行号、语言识别与复制按钮；换行开关、未闭合 fence 优化、超长行降级规则未实现，属 GUI-only。 |
| M06 | TeX 数学与复制 | 有意简化 | 按 `docs/web-ui-map.md` §10.3 不引入 KaTeX/WebView；`render_markdown` 将 TeX 源码作为普通文本显示，不宣称数学排版。 |
| M07 | 图片/文件附件、画廊、灯箱 | 缺失 | G11：无附件 receipt/read，消息渲染器没有图片画廊或灯箱。 |
| M08 | 文件链接行号/HTTP 打开偏好 | 缺失 | G06：文件预览与行号读取缺失；`desktop.rs::open_path_in_default_app` 可外部打开 URI，但 Markdown 链接未接激活逻辑，后者属 GUI-only。 |
| M09 | 消息复制/时间/fork/扩展动作 | 缺失 | 复制与时间动作属 GUI-only 未实现；fork 属 G03，当前没有消息操作栏。 |
| M10 | 工作过程分组/思考预览/整轮折叠 | 部分实现 | `message_renderer.rs::build_assistant_message` 有 reasoning `GtkExpander`，`cards.rs::build_card_shell` 有 revealer；四档过程分组与整轮折叠策略未实现，属 GUI-only。 |
| M11 | 历史分页/锚点/轮次导航/回到底部 | 缺失 | 历史 page 与滚动锚点属 GUI-only 未实现；全量 turn outline 属 G12。当前 controller 没有 `session/page` 调用。 |
| M12 | Token/速度/缓存统计与每轮面板 | 缺失 | G12：没有 tokenUsage/sessionStats 投影与统计面板；`StreamChunk::Usage` 也未累计到 transcript。 |
| M13 | 命令/压缩/重试/错误/配额卡 | 部分实现 | `transcript.rs::apply_event` 对未知事件生成 `GenericEvent`，`message_renderer.rs::build_generic_event` 显示摘要；专用命令/压缩/错误卡未实现，账号配额动作缺 G18。 |
| M14 | 运行状态/计时/断线暂停 | 部分实现 | `ConversationView::set_status` 与 `Composer::set_is_running` 显示状态/spinner，`SessionController::turn_start_times` 记录时长用于通知；实时计时器与断线暂停显示未实现，属 GUI-only。 |
| M15 | 文本选区/右键扩展 | 部分实现 | `message_renderer.rs` 多处 label 设置 `selectable(true)`；右键 quote/扩展菜单未实现，属 GUI-only。 |
| M16 | 草稿/引用 chip/命令 chip/quote/恢复 | 缺失 | 每会话草稿与 quote/chip 属 GUI-only 未实现；命令、技能、引用目录缺 G15。 |
| M17 | 发送与 Host 回显确认 | 部分实现 | `SessionController::send_prompt` 调 `libadwaita/session/prompt`，但提前清空 buffer，没有 pending 气泡、messageId 对账或失败恢复，属 GUI-only。 |
| M18 | Queue/Steer 预览/编辑/删除/插话 | 缺失 | G10：完整队列能力缺失；虽然协议 `SessionPromptParams.mode` 支持 `steer`，controller 固定使用 `prompt`，直接插话也未接线。 |
| M19 | Stop 按钮与双 Esc | 部分实现 | `Composer::set_is_running` 切换停止图标，`setup_composer` 在 running 时调用 `cancel_current_turn`，`setup_double_esc` 实现 500ms 双 Esc；IME、模态与终端焦点仲裁未显式实现，属 GUI-only。 |
| M20 | 失败后“继续生成” | 缺失 | GUI-only：没有错误资格判断与空 contentBlocks 的续写按钮。 |
| M21 | 附件选择/粘贴/拖放/上传进度/重试 | 缺失 | G11：`Composer::new` 的 attach 按钮没有 handler，文件/图片上传与重试未实现。 |
| M22 | 上下文占用率与组成 | 缺失 | G12：没有 contextPressure/contextBreakdown 投影与占用率控件。 |
| M23 | Todo dock | 部分实现 | `SessionModel::handle_event` 跟踪 `todo/write`，`cards.rs::build_todo_diff_card` 显示 todo 列表；composer 上方常驻只读 dock 未实现，属 GUI-only。 |
| M24 | 已发送凭据遮挡与眼睛 | 缺失 | GUI-only：`message_renderer.rs::build_user_message` 直接显示原文，没有 token 遮挡或显隐按钮。 |
| M25 | Trajectory 表/检查器/时间线/搜索/虚拟行 | 部分实现 | `ConversationView::new` 有 Trajectory 占位页与 `AdwViewSwitcher`；事件表、时间线、搜索与检查器未实现，实时原始结果 watch 缺 G13。 |
| M26 | 消息赞踩/反馈弹窗 | 缺失 | G14：无反馈按钮、CAS 对账或七类反馈弹窗。 |
| M27 | Chat/Trajectory 视图选择与固定 | 部分实现 | `ConversationView::new` 有 `AdwViewStack`/`AdwViewSwitcher`；每会话视图选择持久化未实现，属 GUI-only。 |
| M28 | Session 图片共享缓存 | 缺失 | G11：无 attachment 授权读取与按 Session 的图片缓存。 |

## 4. C：工具与展示卡片

| 编号 | 功能点 | 状态 | 证据与缺口分类 |
|---|---|---|---|
| C01 | 工具调用树/PTC/通用 fallback | 部分实现 | `app/src/ui/cards.rs::{build_generic_fallback_card,render_tool_result_card}` 提供通用与专用卡，`message_renderer.rs::build_assistant_message` 显示 tool-call-delta；PTC 子树、dispatch 专用节点与 Trajectory 检查跳转未实现，属 GUI-only。 |
| C02 | `read` 文件读取卡 | 部分实现 | `cards.rs::build_read_card` 与 `transcript.rs::parse_file_read` 展示路径、范围、行号代码；点击文件定位预览缺 G06。 |
| C03 | `read_image` 图片卡 | 缺失 | G11：无图片附件读取、缩略图、尺寸或灯箱。 |
| C04 | `write/edit/apply_patch` diff 卡 | 部分实现 | `cards.rs::build_diff_card` 使用 SourceView diff 语言，`transcript.rs::parse_file_diff` 提取路径/diff；流式预期 diff、多文件与 stopped/meta 权威状态未实现，属 GUI-only。 |
| C05 | Bash/Pwsh/terminal_send 输出卡 | 部分实现 | `cards.rs::build_bash_card` 与 `transcript.rs::parse_bash_tool` 展示命令、cwd、退出码与输出；运行中 spinner、后台状态与信号胶囊未实现，属 GUI-only。 |
| C06 | `todo_write` 摘要与 diff | 部分实现 | `cards.rs::build_todo_diff_card` 列出 todo 与完成 checkbox；done/total、增删/状态/移动 diff 与基线比较未实现，属 GUI-only。 |
| C07 | `grep/glob` 搜索卡 | 部分实现 | `cards.rs::build_search_card` 与 `transcript.rs::parse_search_tool` 展示 pattern/path/results；截断元数据未处理，点击预览缺 G06。 |
| C08 | `web_search/web_fetch` 卡 | 部分实现 | `cards.rs::build_web_card` 与 `transcript.rs::parse_web_tool` 展示 query/URL、状态码与 Markdown；结构化来源列表、截断与外部打开动作未实现，属 GUI-only。 |
| C09 | `ask_user_question` 工具行 | 缺失 | G28：没有可重开/只读回答工具行或跨刷新待处理恢复。 |
| C10 | 控制回执/结构化 details 卡 | 部分实现 | `cards.rs::build_generic_fallback_card` 展示 JSON 参数与输出；Subagent/Job/Schedule/Goal 等结构化 key/value 详情未实现，属 GUI-only。 |
| C11 | Auto-review 拒绝优先呈现 | 缺失 | GUI-only：没有 auto-review meta 解析、优先警告卡或参数折叠。 |
| C12 | 改动/交付卡与 review diff | 部分实现 | `cards.rs::build_deliverables_card` 展示文件增删统计与 diff；review tabs、Present 文件按钮与 stat 缺 G06，controller 也未调用 `workspace/changes`。 |
| C13 | Cordis 动态插件定义卡 run/stop | 缺失 | G27：没有动态实例状态卡或 run/stop 控制。 |
| C14 | 通用 Markdown/终端/diff/读取/搜索渲染器 | 已实现 | `message_renderer.rs::{render_markdown,build_code_block}` 与 `cards.rs::build_card_shell` 提供统一 Markdown、SourceView、复制与展开能力，C02/C04/C05/C07/C08 均复用。 |
| C15 | 下载/另存/图片复制 PNG/菜单 | 缺失 | G06：无文件 bytes/stat 读取；保存对话框、PNG 复制与路径菜单未实现。 |
| C16 | 拖放字节传递 | 缺失 | G11：没有 `GtkDropTarget`、拖放遮罩或附件 receipt；图片直接 base64 提交也未接线。 |
| C17 | 在应用中打开 Workspace/文件 | 部分实现 | `ConversationView::new` 有 Open Folder/Open Terminal，`desktop.rs::{open_path_in_default_app,open_terminal_in_dir}` 使用系统应用/终端；应用列表与文件 stat 缺 G06。 |
| C18 | 右键菜单框架 | 缺失 | GUI-only：目前只有会话行 Archive popover，没有按目标类型异步解析的通用右键菜单框架。 |

## 5. A：审批与提问

| 编号 | 功能点 | 状态 | 证据与缺口分类 |
|---|---|---|---|
| A01 | 工具审批卡接管 composer | 部分实现 | `app/src/ui/approval_dialog.rs::show_approval_dialog` 展示 toolName/callId/reason，Allow Once/Rejected 均正确；`connection.rs::handle_server_request` 无 responder/超时默认 rejected。当前是模态 `AdwDialog` 而非 inline composer，且没有命令预览。 |
| A02 | 审批 Enter/Esc | 缺失 | GUI-only：approval dialog 只有按钮回调，没有 Enter 允许、Esc 拒绝及焦点规则。 |
| A03 | 审批风险说明浮层 | 缺失 | G28：缺 displayReason/warning/detail 扩展字段与风险 popover。 |
| A04 | 通用提问卡/选项/自定义/翻页/跳过 | 部分实现 | `approval_dialog.rs::show_user_question_dialog` 支持每题自定义输入、Submit 与 Cancel；选项按钮、multiSelect、推荐项、翻页与单题跳过未实现，属 GUI-only。 |
| A05 | 提问倒计时/聚焦暂停/超时后作答 | 缺失 | G28：没有 remainingMs、倒计时、聚焦暂停、无限等待或 attachWait 迟到回答。 |
| A06 | 迟到回答/草稿恢复/关闭重开 | 缺失 | G28：没有问题草稿持久化、可重开快照或工具行恢复。 |
| A07 | 已回答气泡与只读回看 | 缺失 | GUI-only：没有问题答案专用右对齐气泡或只读回看；普通 user message 不能替代。 |
| A08 | Plan review 专用卡 | 部分实现 | `show_user_question_dialog` 检测 `UserQuestionIntent::PlanReview`，用 `render_markdown` 显示 detail，同意回 `selected:[approve]`，要求修改回 `cancelled:true`；当前为模态 dialog 而非 inline 卡。 |
| A09 | Plan 模式徽章与退出 | 部分实现 | `SessionModel::handle_event` 跟踪 `plan/mode`，`message_renderer.rs::build_plan_mode_badge` 显示状态；composer 徽章与 `/plan off` 动作缺 G15。 |
| A10 | 已提交计划卡/完整计划预览 | 部分实现 | `transcript.rs::apply_event` 在 `exit_plan_mode` 生成 `PlanCard`，`cards.rs::build_plan_card` 渲染 Markdown；右栏完整预览自动打开与历史分页未实现，属 GUI-only。 |

## 6. S：设置与模型

| 编号 | 功能点 | 状态 | 证据与缺口分类 |
|---|---|---|---|
| S01 | 设置外壳/配置表单/冲突保留草稿 | 部分实现 | `app/src/ui/settings.rs::build_preferences_dialog` 有 PreferencesDialog/Page/Group/SwitchRow；动态 schema、revision 冲突与草稿保护缺 G16。 |
| S02 | 语言选择 | 缺失 | GUI-only：无 8 语言 ComboRow、词典或设置写入；`settings/get`/`mutate` 已在协议中。 |
| S03 | 代码工作工具开关 | 部分实现 | `settings.rs::build_preferences_dialog` 有 Developer Tools `AdwSwitchRow`，但未连接设置或 Trajectory 显隐；动态 schema 缺 G16。 |
| S04 | 连接恢复/服务版本变化/版本行 | 部分实现 | `settings.rs` 有 dsh/protocol 版本行，`restart_supervisor` 支持重启后 follow；版本行目前硬编码，连接恢复 banner 与 release-change 检测未实现，属 GUI-only。 |
| S05 | 打开配置文件 | 缺失 | GUI-only：没有打开 profile 配置文件的动作；`desktop.rs::open_path_in_default_app` 可复用。 |
| S06 | Models provider 行/编辑/排序/增删 | 部分实现 | `settings.rs` 有 DeepSeek provider/key 状态与静态模型目录；未调用 `model/catalog`，provider 编辑/排序/增删未实现，动态 schema 与模型发现缺 G16/G17。 |
| S07 | API Key 只写/校验/保存/删除范围 | 部分实现 | `settings.rs` 使用 `AdwPasswordEntryRow`、masked 状态与 Save/Rotate；`CredentialStore::{get_key,set_key,delete_key,mask_key}` 与 `SessionController::rotate_credential` 已接线。ASCII 校验、保存 spinner 与删除范围未实现。 |
| S08 | 首启 DeepSeek 引导与版本说明 | 部分实现 | `app/src/ui/onboarding.rs::show_onboarding_dialog` 提供 Welcome、PasswordEntryRow、Save/Configure Later；`SessionController::set_connection` 在 `credentials/status` 未配置时自动注入已存 key 或显示向导。版本化 WelcomeNotice/settings 持久化未实现。 |
| S09 | DeepSeek 账号登录/资料/余额/充值 | 缺失 | G18/G29：账号页只是 Local user profile 占位，无 OAuth、资料、余额或配额恢复。 |
| S10 | Agent loop 并行上限 | 缺失 | G16：没有动态 schema 或并行上限 SpinRow。 |
| S11 | 子代理深度/并行/模型设置 | 缺失 | G16：没有子代理设置表单与 revision 语义。 |
| S12 | Web 搜索/获取 provider 与结果数 | 缺失 | G16/G17：没有 provider 目录或搜索设置表单。 |
| S13 | 终端超时/输出上限 | 部分实现 | `settings.rs` Shell 页显示静态 `Command Timeout: 300 seconds`；实际编辑、输出上限与 schema 缺 G16。 |
| S14 | 会话日志上传开关 | 缺失 | GUI-only：没有开关或 Host namespace 可用性检测；协议 settings/get/mutate 可支撑后续实现。 |
| S15 | 默认权限与 `/permission` 会话选择 | 缺失 | G19：没有权限目录、默认权限设置或会话权限切换。 |
| S16 | Agent Preset 卡/默认值/YAML | 缺失 | G20：没有 preset 列表、默认值或 YAML 只读预览。 |
| S17 | Goal 条带编辑/暂停/恢复/清除 | 缺失 | G21：没有 Goal 投影或操作条。 |
| S18 | `/model` 与会话模型切换 | 部分实现 | `SessionController::{open_model_picker,switch_model}` 显示模型 popover 并调用 `session/model/set`，composer badge 更新；模型列表硬编码，未调用 `model/catalog` 或 `session/model/get`，reasoningEffort、失败回滚未实现。 |
| S19 | 主题/字号/字体 | 部分实现 | `settings.rs` 有 System/Light/Dark 选择，`desktop.rs::configure_system_style` 使用 StyleManager 与 CSS 变量；字号、字体族及设置持久化未实现，属 GUI-only。 |
| S20 | 快捷键速查/录制/恢复 | 部分实现 | `desktop.rs::build_shortcuts_window` 与 `window.rs::show-help-overlay` 显示固定快捷键；搜索、录制、恢复默认与冲突检测未实现，属 GUI-only。 |

## 7. P：插件与凭据

| 编号 | 功能点 | 状态 | 证据与缺口分类 |
|---|---|---|---|
| P01 | 插件列表/状态诊断/刷新 | 部分实现 | `settings.rs::build_preferences_dialog` 有 Core Bundles 与 Plugin Management 行，conversation 另有插件占位页；没有调用 `plugin/list`、状态诊断或刷新，完整 Loader 清单缺 G24。 |
| P02 | Bundle/组件启停与 restart-required | 部分实现 | `settings.rs` 的 SwitchRow 调 `SessionController::toggle_plugin` → `plugin/enable|disable`，返回 restartRequired 时显示 `AdwBanner`；Restart Now 调 `restart_supervisor` 并重新 follow 活动会话。没有真实列表状态、组件行粒度与全部会话恢复。 |
| P03 | 插件配置表单 | 缺失 | G16/G24：没有插件动态配置表单、Bundle/row 命名空间或 Loader 清单。 |
| P04 | Bundle 详情文档与错误重试 | 缺失 | G22：无 detail/readme/错误重试。 |
| P05 | 添加插件与 inspect 预检 | 缺失 | G22：无来源输入、inspect 或安全预检。 |
| P06 | 安装进度/日志/取消/恢复/构建审批 | 缺失 | G22：无安装任务、日志、取消、恢复或构建审批。 |
| P07 | 卸载/更新/版本检查与恢复 | 缺失 | G22：无版本检查、更新、卸载或恢复。 |
| P08 | 安装源选择/记忆/探测 | 缺失 | G22：无 registry 选择、测速或记忆。 |
| P09 | 插件市场索引/搜索/分页/安装确认 | 缺失 | G23：无市场索引、搜索、分页或安装确认。 |
| P10 | 设置→内置插件只读清单/页面重试 | 缺失 | G24：当前只有硬编码插件行，不是 Loader 只读清单，也没有重试。 |
| P11 | 凭据 describe/set/unset 与完整 provider seam | 部分实现 | `CredentialStore` 与 `SessionController::{inject_credential,rotate_credential}` 覆盖 API key 保存、注入与轮换；`credentials/remove` 未接 UI，OAuth/record seam 缺 G29。 |
| P12 | 入口定制/Pin/插件草稿 | 缺失 | GUI-only：没有“更多”菜单、Pin 本地排序或只追加草稿的动作。 |

## 8. O：其他外围

| 编号 | 功能点 | 状态 | 证据与缺口分类 |
|---|---|---|---|
| O01 | 启动页/失败报告 | 缺失 | GUI-only：没有启动 StatusPage 或逐项失败诊断；`HarnessSupervisor::start` 已提供 `ProcessSpawnFailed`/`HandshakeFailed` 等错误可接入。 |
| O02 | 浏览器 token/Cookie/信任检查 | 有意不移植 | 使用 Unix socket 0600 与本地进程边界；`HarnessSupervisor::start` 与 `set_socket_permissions_0600` 实现原生信任边界，无 Web token/Cookie。 |
| O03 | 客户端模块系统/bundle 路由/页面同步重试 | 有意不移植 | 不引入 React/CJS 模块系统；`app/src/ui/` 为静态 Rust widget 模块，插件状态仍走 `libadwaita/plugin/*`。 |
| O04 | HMR/开发 bundle 重建 | 有意不移植 | profile 禁用 HMR；`ProfileTemplateConfig.disabled_rows` 包含 `hmr`，插件变更走 restart-required 冷重启而非页面热替换。 |
| O05 | RPC/WebSocket/Fetch 路由 | 有意不移植 | `app/src/connection.rs::BridgeConnection` 统一 Unix socket NDJSON JSON-RPC，没有第二条 Web Host 私有 RPC 通路。 |
| O06 | Worker/流式上传/receipt | 缺失 | G11：无通用文件流式上传/receipt；当前只有文本 `PromptContentBlock::Text` 接线。 |
| O07 | 快捷键引擎/持久化 | 部分实现 | `main.rs` 设置全局 accels，`controller.rs` 使用 `GtkEventControllerKey`，`desktop.rs` 有快捷键窗口；自定义键位持久化与冲突管理未实现，属 GUI-only。 |
| O08 | Locale 词典/fallback | 缺失 | GUI-only：没有 Rust i18n 词典、系统 locale fallback 或动态文案处理。 |
| O09 | observable store/useResource | 部分实现 | `SessionModel`、`ConversationTranscript` 持有领域真相，`controller.rs` 用 `Rc<RefCell<_>>` 与 Tokio mpsc + `spawn_local` 更新 UI；统一 URI resource handle 与释放生命周期未实现，属 GUI-only 原生缺口。 |
| O10 | React renderer/Slots | 有意不移植 | 采用固定 GTK4/libadwaita 控件与 Rust 领域对象，不引入 React/Slot/JS 运行时。 |
| O11 | Desktop 产品埋点 | 有意不移植 | 按设计不新增产品遥测面。 |
| O12 | 会话后台 jobs 列表/输出/双击停止 | 缺失 | G25：没有 jobs roster、输出流或双击停止。 |
| O13 | 自动化计划任务 CRUD/运行/历史/时钟 | 缺失 | G26：schedules 目前只有占位页，没有任务 CRUD、历史或时钟。 |
| O14 | PWA/manifest/子路径挂载 | 有意不移植 | 使用原生 `adw::Application`、应用 id 与 GIO 单实例激活，不实现 PWA 或浏览器 URL scope。 |
| O15 | 系统通知与提示音 | 部分实现 | `desktop.rs::{send_turn_completed_notification,send_background_notification}` 与 `SessionController::handle_status_notification` 已实现系统通知；提示音与关闭通知设置未实现，属 GUI-only。 |
| O16 | 测试支撑包 | 已实现 | `app/tests/` 下有 protocol、connection、supervisor、supervisor_dsh、transcript、credentials、cards、desktop 测试；真实 dsh 集成测试见 `app/tests/supervisor_dsh_tests.rs` 与 `bridge/tests/integration/dsh-real.spec.ts`。产品窗口没有测试入口。 |

## 9. 优先补齐清单：先做无需新 bridge 的 GUI-only 工作

### P0：必须先补的启动链路

1. 在 `app/src/main.rs` 创建 `HarnessSupervisor`，执行 `start().await`，再调用 `SessionController::set_supervisor()` 与 `set_connection()`。这是所有真实 RPC 行验收的前置条件，不需要新 bridge。
2. 将启动失败映射到 O01 的 `AdwStatusPage` 错误报告，避免只有空窗口。

### P1：Top 10 GUI-only 缺口

| 优先级 | 行号 | 可直接复用的现有能力 | 补齐内容 |
|---:|---|---|---|
| 1 | N06 启动恢复当前 Session/Workspace | `SessionController::{load_session_list,follow_session}` | 本地保存当前 session id/cwd，启动后恢复并 rebaseline。 |
| 2 | O01 启动页/失败报告 | `HarnessSupervisor::start` 的错误类型与状态机 | 增加启动 StatusPage、错误详情与重试按钮。 |
| 3 | N08 关闭当前会话视图 | 协议已有 `libadwaita/session/release` | 增加 close-view action、release 调用与空白首页，不停止 Agent。 |
| 4 | M11 历史分页/锚点/回到底部 | 协议已有 `libadwaita/session/page` | 上沿加载历史、保持锚点、回到底部；turn outline 后续仍依赖 G12。 |
| 5 | M16 草稿/引用恢复中的每会话草稿 | `Composer::text_view` 与 active session 切换 | 每会话保存/恢复未发送文本，切换不丢草稿；命令/引用目录另等 G15。 |
| 6 | M17 发送与 Host 回显确认 | `SessionPromptResult.messageId` 与 session.event | 收到 messageId/事件后再清草稿，增加 pending 气泡与失败恢复。 |
| 7 | M20 失败后“继续生成” | 空 contentBlocks 的 `session/prompt` | 增加失败资格判断和继续按钮，不注入隐藏文本。 |
| 8 | M23 Todo dock | `SessionModel.todos` 与 `cards.rs::build_todo_diff_card` | 在 composer 上方增加只读 TodoPanel 与完成计数。 |
| 9 | M24 已发送凭据遮挡与眼睛 | `build_user_message` 与 `CredentialStore::mask_key` 的思路 | 对已发送文本做 token mask、单值显隐，复制仍取原文。 |
| 10 | N27 连接状态与手动重连 | `BridgeConnection::state`、`restart_supervisor` | 增加连接状态 banner/pill、手动重连与错误提示。 |

### P2：其余无需新 bridge 的可见补齐

- 导航与桌面：N01/N02 折叠轨与响应式，N03/N14 Pin/排序/归档筛选，N04 空会话复用，N07 品牌首页，N09 取消归档与确认，N11 侧栏标题绑定，N13 本地标题过滤，N16 右栏状态持久化，N25 workflow 通用节点，N26 状态点/未读，N28 自动重连与 rebaseline。
- 消息基础体验：M02 pending/steering，M04 表格/脚注/安全链接，M05 换行开关与流式 fence，M08 HTTP 外链，M09 复制/时间，M10 过程分组，M13 专用状态卡，M14 实时计时，M15 右键扩展，M18 直接 steer，M19 焦点仲裁，M25 本地 trajectory，M27 视图选择持久化。
- 卡片与审批：C01 PTC 树，C04 多文件 diff，C05 运行/信号状态，C06 todo diff 摘要，C07 截断，C08 来源，C10 结构化 details，C11 auto-review 拒绝卡，C12 review tabs，C17 应用列表，C18 通用右键菜单；A01 inline 审批与命令预览，A02 Enter/Esc，A04 选项/翻页，A07 回答气泡，A08 inline plan review，A09 composer 徽章，A10 右栏计划预览。
- 设置与插件：S02 语言，S03 Developer Tools 生效，S04 恢复/版本提示，S05 打开配置，S07 校验/删除，S08 版本说明，S13 Shell 编辑，S14 日志开关，S18 catalog/get/reasoning，S19 字号字体，S20 快捷键录制；P01 接入 plugin/list 与刷新，P02 完整重启恢复，P12 Pin/草稿。
- 外围：O07 键位持久化，O08 locale 词典，O09 resource handle 生命周期，O15 提示音与通知开关。

## 10. 按 G-gap 的后续 bridge 工作顺序

先解除核心会话/消息能力，再补文件与附件，最后补完整生态。每项仍需同步 `docs/bridge-protocol.md`、bridge 实现、Rust 类型与真实 dsh 集成测试。

| 顺序 | G-gap | 影响行 | 需要补齐的 bridge 能力 |
|---:|---|---|---|
| 1 | G02 | N10 | `session/rename` 与 `session.title` 推送。 |
| 2 | G10 | M02、M18 | queue/list/edit/remove/steer 与 inbox 通知。 |
| 3 | G06 | N18、N19、C02、C07、C12、C15、C17 | workspace files list/stat/read/readBytes，含根目录边界。 |
| 4 | G11 | M07、M21、M28、C03、C16、O06 | attachment upload/read、进度与授权 receipt。 |
| 5 | G15 | M16、A09 | command/skill/reference list 与 execute，含 `/plan off`。 |
| 6 | G16 | S01、S03、S06、S10–S13、P03 | settings schema/describe、expectedRevision 与更新通知。 |
| 7 | G12 | M11、M12、M22、S17 | turnOutline、tokenUsage、contextPressure、goal 等投影。 |
| 8 | G28 | A03、A05、A06、C09 | 审批/提问扩展元数据、remainingMs、attachWait 与恢复快照。 |
| 9 | G04 | N03、N13 | session 全文搜索与 provider-unavailable 区分。 |
| 10 | G03 | N12、M09 | session/fork，传 atSeq/increaseTitle。 |
| 11 | G01 | N03、N15 | workspace list/create/rename/remove。 |
| 12 | G17 | S06、S12 | model discover 与 web-search provider 目录。 |
| 13 | G13 | M25 | tool-result/watch 实时原始结果。 |
| 14 | G14 | M26 | message-feedback 与 session-feedback CAS。 |
| 15 | G19 | S15 | permission catalog/get/set 与会话投影。 |
| 16 | G20 | S16 | agent-preset list/read/default。 |
| 17 | G21 | S17 | goal get/edit/pause/resume/clear 与通知。 |
| 18 | G24 | P01、P03、P10 | Loader inventory、诊断与 retry。 |
| 19 | G22 | P04–P08 | Bundle detail/inspect/install/cancel/wait/remove/versions/update/registries 与日志。 |
| 20 | G23 | P09 | plugin-market indexes/search/detail 与变更通知。 |
| 21 | G25 | O12 | job/list/follow/kill 与输出通知。 |
| 22 | G26 | N24、O13 | schedule catalog/create/edit/run/history/delete/delivery-limits。 |
| 23 | G05 | N17 | 交互终端 create/input/resize/close/list 与输出通知。 |
| 24 | G07 | N19 | Office→PDF 转换与有界结果。 |
| 25 | G08 | N21 | subagent list/prompt/interrupt 与目录通知。 |
| 26 | G09 | N22 | agent-team get/set-enabled 与任务投影。 |
| 27 | G27 | C13 | Cordis 动态实例 run/stop/status。 |
| 28 | G18 | S09、M13 | account watch/sign-in/profile/balance 与配额通知。 |
| 29 | G29 | P11、S09 | credentials records/OAuth grant，不回读 secret。 |

## 11. 结论

当前代码已形成可编译的 GTK4/libadwaita 骨架、可靠 transcript、基础卡片、审批/计划审查、凭据、设置与桌面通知接线；但主程序尚未接入 supervisor/connection 启动链路，且大量上游高级功能仍依赖 G01–G29。按本表 132 行统计，当前为 **3 已实现、60 部分实现、60 缺失、8 有意不移植、1 有意简化**。下一轮应优先完成 P0 启动链路与 P1 的 10 个 GUI-only 缺口，再按第 10 节顺序推进 bridge 缺口，避免把占位页或静态文本误记为“已实现”。
