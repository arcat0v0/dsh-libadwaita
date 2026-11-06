# DSH Web → GTK4/libadwaita 移植清单

- 唯一功能来源：`docs/dsh-web-feature-inventory.md`（上游 `fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159`）。
- bridge 契约：`docs/bridge-protocol.md` v1，方法以 §5 为准，通知以 §7 为准，双向请求以 §8 为准。
- 目标：阶段 4–6 的实现与最终逐行验收；**照抄上游布局、导航与交互，只更换控件技术，不重新设计产品**。
- 行号约定：`N/M/C/A/S/P/O + 两位序号` 分别对应功能清单 §1–§7，各表自上而下首行为 `01`。映射表与清单一一对应，不新增功能。
- bridge 列只写真实契约：`libadwaita/*`、`session.event`、`agent.stream`、`session.status`、`session.title`、`user-question.cancelled`、`approval/request`、`user-question/ask`。`G01–G29` 均为 **bridge 缺口**，不是现有 API；汇总见 §10。`无需 bridge` 表示纯 GUI/本机桌面能力。

## 1. 页面与导航结构

### 1.1 主窗口组成（保持上游三栏与全局面板语义）

```text
AdwApplicationWindow
└─ AdwToastOverlay
   └─ AdwToolbarView
      ├─ top: AdwHeaderBar（侧栏开关、会话标题/状态、模型、全局动作）
      └─ content: AdwOverlaySplitView（左侧栏；窄窗转抽屉）
         ├─ sidebar: GtkStack（完整侧栏 / 56px 折叠轨道）
         │  ├─ 品牌 + New Session + GtkSearchEntry
         │  ├─ GtkTreeListModel + GtkListView（Workspace / Session）
         │  └─ 面板入口（插件、自动化任务）+ 设置 + 账号
         └─ content: GtkPaned(horizontal)
            ├─ start: GtkStack（conversation / plugins / schedules 全局面板）
            │  └─ conversation
            │     ├─ 会话头部、血缘面包屑、连接/运行状态
            │     ├─ AdwViewStack + AdwViewSwitcher（Chat / Trajectory）
            │     ├─ GtkListView + GtkScrolledWindow（虚拟化 transcript）
            │     └─ composer（GtkTextView、附件、Queue、模型、发送/停止）
            └─ end: 右停靠栏
               ├─ AdwTabBar + AdwTabView（Terminal / Files / Preview）
               ├─ 需要 split 时嵌套 GtkPaned，每格一个 AdwTabView
               └─ fullscreen 时切换根 GtkStack，隐藏中栏与 tab 非当前内容
```

| 上游区域/导航 | GTK4/libadwaita 实现决定 | 必须保持的行为 |
|---|---|---|
| 左侧栏固定列、折叠轨道、窄屏抽屉 | `AdwOverlaySplitView`，侧栏内部 `GtkStack` 切换完整/轨道；宽度拖拽由其分隔柄承担，窄窗 `collapsed=true` | `Mod+B`；导航后关闭抽屉；打开右栏时按上游规则收起侧栏 |
| 中栏 Conversation 与 Plugins/Schedules 全局页面互换 | `GtkStack` 保存各页面实例；`layout.selectPanel` 对应 GUI `PanelController.select` | 切面板不隐式改当前 Session；返回 conversation 恢复原 Session |
| Chat / Trajectory 标签、更多菜单、固定 | `AdwViewStack` + `AdwViewSwitcher`；空间不足时用 `GtkPopoverMenu` 承载隐藏项 | Chat 始终固定；Trajectory 受 `ui-settings.enabled` 控制；选择按 Session 保存 |
| 左侧 Workspace/Session 树 | `GtkTreeListModel` + `GtkListView` + `GtkSignalListItemFactory`；行尾 `GtkMenuButton` | 分组/平铺、归档置灰、置顶、状态点、搜索、拖放排序、键盘操作 |
| 中栏与右栏宽度、右栏挤压 | `GtkPaned(horizontal)`，按上游尺寸约束调整 position | 右栏初始约 45%、上限 70%；优先压缩右栏以保护中栏 |
| 右栏 tabs / split / fullscreen / close | `AdwTabView` + `AdwTabBar`；split 用嵌套 `GtkPaned`；fullscreen 用根 `GtkStack` | tab 状态按 Session 保存；close page ≠ close Session；split 可拖宽 |
| 会话/全局 Header | `AdwHeaderBar` + `GtkLabel` + `GtkMenuButton`；状态使用小 `GtkLabel`/ spinner，不另造 Web chrome | 标题双击重命名；血缘、模型、jobs、反馈、打开工作区等动作位置对应上游 |
| 设置 | `AdwPreferencesDialog` → `AdwPreferencesPage` → `AdwPreferencesGroup` → 各类 `Adw*Row` | `Mod+,`；分区导航、逐项滚动、编辑冲突保留草稿 |
| 审批/提问/plan review | **上游是接管 composer 的 inline 卡**：`GtkFrame` + `AdwClamp` + `GtkBox`，替换普通 composer；仅风险详情/二次确认使用 `AdwDialog`/`AdwAlertDialog` | 不错误改成全局模态；仍路由到所属 Session；Enter/Esc 与 fail-closed 不变 |
| 右键与更多菜单 | `GtkPopoverMenu` + `GMenuModel`；复杂内容用 `GtkPopover` | 保留目标 Session/path/line；菜单动作与快捷键共用命令 |
| Toast、错误、重启提示 | `AdwToastOverlay`、`AdwToast`、`AdwBanner` | 安装失败保留诊断；`restartRequired` 必须给“立即重启”动作 |
| 首启与版本化说明 | `AdwDialog` + `AdwStatusPage`/表单；不可绕过时按上游条件呈现 | 无凭据不阻塞 dsh 握手；可稍后配置；版本说明只在版本变化后出现 |

### 1.2 控件选型边界

- **不使用 `AdwNavigationSplitView`**：左栏不是层层 push 的 NavigationPage 栈，`AdwOverlaySplitView` 更贴合上游可折叠三栏。
- **不使用 `GtkNotebook`**：右栏需要可拖 tab、关闭按钮与 libadwaita 外观，采用 `AdwTabView`；`GtkNotebook` 仅作为无 libadwaita 时的技术后备，不进入主方案。
- **不把审批做成普通模态框**：这会改变上游 composer 占用、迟到回答与重开行为。模态仅用于明确危险确认、设置和首启。
- **窗口 chrome 使用系统原生**：`AdwHeaderBar` 替代 Electron/macOS/Windows 特殊拖拽区；不复制浏览器 PWA chrome。

## 2. 会话与导航（清单 §1，N01–N28）

| 功能点（清单行） | 上游组件 | libadwaita/GTK 控件与交互 | 所需 bridge 能力 | 阶段 | 备注 |
|---|---|---|---|---|---|
| N01 三栏 AppFrame 与 shell 座 | `ui-layout` | §1.1 主窗口；overlay 由 `AdwToastOverlay`/顶层 `GtkOverlay` 实现 | `libadwaita/initialize`；布局无需 bridge | 4 | 宽度、抽屉、右栏优先收缩照抄；Electron chrome 由 `AdwHeaderBar` 等价替代 |
| N02 左侧栏品牌/New Session/面板/设置/折叠轨道 | `ui-sidebar` | `GtkStack` 完整/轨道；`GtkButton`、`GtkListBox`、`GtkMenuButton` | 面板数据见对应行；导航无需 bridge | 4 | 埋点不复制；品牌由资源图标替代 |
| N03 Workspace/Session 列表与排序/归档过滤 | `ui-workspace` | `GtkTreeListModel` + `GtkListView`；拖放用 `GtkDragSource`/`GtkDropTarget` | `libadwaita/session/list`、`libadwaita/session/archive`；**G01** 空 Workspace/层级与重命名删除；**G04** 内容搜索 | 4 | Pin 与显示顺序可存 GUI 本地；无 Session 删除 |
| N04 新建会话/复用 blank/命名创建 | `ui-workspace/navigation.ts` | New Session 按钮；创建前可选 `AdwDialog` 填名称/preset；复用同 cwd/preset blank | `libadwaita/session/create`、`libadwaita/session/follow` | 4 | 不保证新 ID；`cwd` 必须传 |
| N05 打开并恢复既有会话 | `ui-workspace/navigation.ts`、Session Controller | 行激活→加载页→transcript；冷列表不拉起 Agent | `libadwaita/session/list`、`libadwaita/session/follow`、`libadwaita/session/page` | 4 | follow 返回快照后再显示流，避免双份历史 |
| N06 启动时恢复当前 Session/Workspace | `ui-workspace/navigation.ts` | 启动恢复状态机 + `GtkStack` loading/conversation | `libadwaita/session/list`、`libadwaita/session/follow`；本地保存当前 ID | 4 | blank 重新核对 preset；无上次会话则选最近 cwd |
| N07 官方品牌与首页鲸鱼层 | `ui-brand-official` | `GtkPicture`/应用图标；首页 `GtkDrawingArea` 动画 | 无需 bridge | 6 | 尊重系统减少动效；不做 Canvas 逐像素复刻 |
| N08 “关闭”当前会话视图 | `ui-workspace/navigation.ts` | 返回无会话首页/关闭当前 view，不关 dsh、不停止 Agent | `libadwaita/session/release` | 4 | 明确不等于归档/删除 |
| N09 归档/取消归档，活动会话二次确认 | `ui-workspace`、Workspace Controller | `AdwAlertDialog` 确认停止并归档；归档行置灰 | `libadwaita/session/archive`（`archived:true/false`）、`libadwaita/session/list` | 4 | 归档路径丢弃 pending inbox；与普通 Stop 区分 |
| N10 手动重命名 | `RenameSession.tsx` | 标题双击/行菜单/快捷键打开 `AdwDialog` 单行输入 | **G02**；成功后消费 `session.title` | 4 | 当前协议只有标题通知，没有 rename 方法 |
| N11 自动标题 | `session-title*` | `AdwHeaderBar` 标题与侧栏行绑定同一 title 状态 | `session.title`；创建后 `session.event` | 4 | 自动生成在 Host；无模型时不承诺 LLM 成功 |
| N12 Fork 会话 | `MessageIconActions.tsx` | 完成轮次操作菜单“分支”；成功后打开新 Session | **G03**；新会话随后 `libadwaita/session/follow` | 6 | 需要 `atSeq` 与 `increaseTitle`；不能伪造成本地复制 |
| N13 会话元数据/内容搜索 | `ui-workspace`、Session Controller | `GtkSearchEntry` + 结果 `GtkListView`；本地标题过滤即时 | `libadwaita/session/list`；全文搜索 **G04** | 6 | 全文 provider 可能未启用，UI 显示不可用而非空结果 |
| N14 行菜单、Pin、Archive、Toast | `session-actions/`、`context-menus.tsx` | `GtkPopoverMenu` + `AdwToast` | `libadwaita/session/archive`；Pin/本地顺序无需 bridge | 4 | 菜单与快捷键共用动作 |
| N15 Workspace 创建/层级/重命名/删除与目录选择 | `ui-workspace`、两个 directory-picker | `GtkFileDialog.select_folder`（Portal）；层级 `GtkTreeListModel`；重命名/删除 `AdwDialog` | `libadwaita/session/create`（新 cwd）；`libadwaita/session/release`；**G01** | 4 | 切换=目标 cwd 新会话+迁移草稿+释放旧视图；**不做 workdir/set** |
| N16 右停靠栏框架 | `ui-sidebar-right`、`ui-dockkit` | `AdwTabView`/`AdwTabBar`、嵌套 `GtkPaned`、fullscreen `GtkStack` | 打开资源的数据方法见 N17–N20；框架无需 bridge | 6 | 每 Session 保存 tab/布局；无 Session 不显示 |
| N17 终端 tab | `ui-sidebar-terminal` | `Vte.Terminal`（VTE 4）+ shell `GtkDropDown` + 重命名 `AdwDialog` | **G05**（创建、输入、resize、输出流、恢复、关闭） | 6 | VTE 依赖需构建阶段确认；只读工具输出卡不属于此缺口 |
| N18 文件 tab | `ui-sidebar-files` | `GtkTreeListModel` + `GtkListView`、导航 `GtkRevealer`、刷新按钮 | **G06**（list/stat） | 6 | 根目录取 `libadwaita/session/workdir/get`；只列目录 |
| N19 文档预览 tab | `ui-sidebar-documentpreview` | Markdown/代码用 §4 渲染器；图片 `GtkPicture`；PDF 用 Poppler + `GtkPicture`；行定位滚动 | **G06**（read/readBytes/stat）；Office→PDF **G07** | 6 | 只读；HTML 不嵌 WebView，见 N20/§4.4 |
| N20 浏览器 tab | `ui-sidebar-browser` | HTTP(S) 默认由 `GtkUriLauncher` 打开系统浏览器；地址历史仍可在“链接”面板显示 | 无内嵌浏览器；外部打开无需 bridge | 6 | **有意不移植内嵌 iframe/webview**：上游 Web 默认禁用且本项目禁止 WebView；系统浏览器是原生等价物 |
| N21 子代理目录/血缘/transcript/@引用 | `ui-subagent` | 侧栏子树 + `GtkLinkButton` 面包屑；子 transcript 复用 Chat；`GtkPopover` 引用候选 | **G08**；子 Session 打开后可用 `libadwaita/session/follow` | 6 | Stop 子代理不能用普通 cancel 冒充 |
| N22 Agent Team 开关与共享任务板 | `ui-agent-team` | `GtkSwitch` + `GtkListView` 任务板 | **G09** | 6 | 新 Session 默认关闭 |
| N23 插件面板入口 | `ui-plugin-manager` | 左栏按钮切换全局 `GtkStack` 的 plugins page | `libadwaita/plugin/list`；高级能力见 P01–P10 | 5 | 页面属于 profile，不属于 Session |
| N24 自动化任务页入口 | `ui-schedule` | 左栏按钮切 schedules page；右栏 tab 复用 `AdwTabView` | **G26** | 6 | 与后台 jobs 不同 |
| N25 Workflow 运行节点与子会话导航 | `ui-workflow-run` | 折叠 `GtkExpander` + 阶段/成员列表；点击子会话走统一导航 | `session.event`（workflow 事件）、`libadwaita/session/follow` | 6 | 仅顶层运行开放导航 |
| N26 会话状态点/完成未读 | `ui-session`、`ui-workspace/tree.ts` | 行内状态圆点 `GtkDrawingArea`/`GtkLabel`；未读粗体 | `session.status`；pending 由 `approval/request`、`user-question/ask` 本地仲裁 | 4 | precedence：plan-review > question > approval；不是系统通知 |
| N27 连接状态与手动重连 | `ConnectionIndicator.tsx` | `AdwBanner`/状态 pill，点击 `GtkButton` 立即重连 | `libadwaita/initialize`；GUI 监控 Unix socket | 4 | 自动退避与恢复提示由 `BridgeConnection` 提供 |
| N28 重连状态修复 | Session Controller transport | 重连期间暂停追加；恢复后重建列表/transcript | 同代际 `libadwaita/session/page`；跨代际 `libadwaita/initialize` + `libadwaita/session/follow` | 4 | 以 `generationId` 判定；旧 stream 不跨代际复用 |

## 3. 消息与流式（清单 §2，M01–M28）

| 功能点（清单行） | 上游组件 | libadwaita/GTK 控件与交互 | 所需 bridge 能力 | 阶段 | 备注 |
|---|---|---|---|---|---|
| M01 事件装配/历史前插/settlement | `ui-conversation/assembly.ts` | Rust `ConversationTranscript` → `GtkListModel`；live 临时行与持久行原子替换 | `libadwaita/session/follow`、`libadwaita/session/page`、`session.event`、`agent.stream` | 4 | seq/revision 连续；gap 时暂停追加并修复 |
| M02 Chat 用户消息/steering/回显 | `ui-chat` message/inbox nodes | 用户右对齐气泡、steering 标记；pending 行复用同一 widget id | `libadwaita/session/prompt`、`session.event`；队列快照 **G10** | 4 | Host receipt/持久事件后清 pending |
| M03 Assistant 文本/思考/工具 delta、定稿/中断 | `ui-chat/assistant.ts` | 文本与 reasoning 分行；`GtkSpinner`；中断冻结样式 | `agent.stream`、`session.event`、`libadwaita/session/follow` baseline | 4 | end committed 替换，abandoned 丢弃临时块；无 messageId 不显示定稿操作 |
| M04 Markdown/GFM/表格/脚注/安全链接 | `AssistantMarkdown.tsx`、`ui-primitives/markdown` | §4 pulldown-cmark → `GtkTextView`/块 widget；表格 `GtkColumnView`；链接安全白名单 | 无需 bridge；打开文件/链接见 M08 | 4 | 不执行原始 HTML；禁 `javascript:` 等；流式只重解析尾部 |
| M05 代码高亮/复制/换行/行号/流式 fence | `CodeBlock.tsx` | 只读 `GtkSourceView` + `GtkSourceBuffer`；复制 `GdkClipboard`；换行 toggle | 无需 bridge | 4 | 已检测 GtkSourceView 5；长行不高亮规则保留 |
| M06 TeX 数学与复制 | `katex.tsx`、`copy-tex.ts` | `GtkSourceView`/等宽 `GtkTextView` 展示原始 TeX，复制写 `$…$`/`$$…$$` | 无需 bridge | 6 | **有意简化**：不引入 WebView/KaTeX；先保真源码与复制，不声称已数学排版，见 §4.3 |
| M07 图片/文件附件、画廊、灯箱 | `UserContent.tsx`、`ui-attachment` | `GtkFlowBox` + `GtkPicture`；`AdwDialog` 原图；文件行显示名称/类型/大小 | `libadwaita/session/prompt`（内联图片）；持久读取/通用文件 **G11** | 4（图片）/6（通用文件） | HTTP(S) 图仅安全绝对 URL；GIF 动画支持不作为首版承诺 |
| M08 文件链接行号/HTTP 打开偏好 | `ui-chat/apply.ts`、`file-link.ts` | 文件→右侧预览并滚动行；HTTP→`GtkUriLauncher`；右键 `GtkPopoverMenu` | `libadwaita/session/workdir/get`；文件内容 **G06** | 6 | 无内嵌 Browser 时始终外部打开；保留 `#L`/`:line` 解析 |
| M09 消息复制/时间/fork/扩展动作 | `MessageIconActions.tsx` | hover/focus `GtkBox` 动作条；复制 `GdkClipboard`；fork 菜单 | `session.event`；fork **G03** | 4（复制/时间）/6（fork） | 只给符合上游条件的定稿消息显示 fork |
| M10 工作过程分组/思考预览/整轮折叠 | `process-groups.ts`、`ReasoningRow.tsx` | `GtkExpander` + `GtkRevealer`；四档设置 `GtkDropDown` | `session.event`、`agent.stream`、`libadwaita/settings/get`、`libadwaita/settings/mutate` | 6 | 折叠时机与例外照抄 |
| M11 历史分页/锚点/轮次导航/回到底部 | `ChatView.tsx`、`TurnNavigator.tsx` | `GtkScrolledWindow` 上沿加载；`GtkListView` 保持锚点；轮次 `GtkPopover` 目录 | `libadwaita/session/page`；全量 turn outline **G12** | 4 | 无 outline 时先支持已加载轮次；不抢滚动位置 |
| M12 Token/速度/缓存统计与每轮面板 | `StatsPills.tsx`、`TurnUsagePanel.tsx` | `GtkLabel` 胶囊 + `AdwDialog`/`GtkPopover` 明细 | `agent.stream` usage、`session.event`；完整投影 **G12** | 6 | 用量不完整时不显示伪造合计 |
| M13 命令/压缩/重试/错误/配额卡 | `ui-chat/conversation-nodes/*` | 通用状态卡 `GtkFrame` + 图标/正文/展开详情 | `session.event`、`agent.stream`；账号配额动作 **G18** | 4（记录卡）/5（账号动作） | 历史分页不重复弹全局配额提示 |
| M14 运行状态/计时/断线暂停 | `RunningStatus.tsx` | `GtkSpinner` + 状态 `GtkLabel` + 计时器 | `session.status`；socket 状态 | 4 | 断线暂停的是本地计时显示，不是 Agent |
| M15 文本选区/右键扩展 | `text-selection.ts` | `GtkTextView` selectable、`GtkGestureClick` 右键、`GtkPopover` | 无需 bridge；具体动作按目标另列 | 4 | 流式/工具卡不提供不支持的选区动作 |
| M16 草稿/引用 chip/命令 chip/quote/恢复 | `ui-conversation/input/*` | `GtkTextView` + TextChildAnchor chips；逐 Session 草稿存应用私有状态 | `libadwaita/session/prompt`；命令/技能/引用目录 **G15** | 4（文本/quote）/6（发现） | 切换 Session 不丢草稿；文件对象不持久化 |
| M17 发送与 Host 回显确认 | `ui-conversation/service.ts` | 发送后 composer 只读、pending 气泡；失败恢复草稿并排错 | `libadwaita/session/prompt`、`session.event` | 4 | 同一时刻一条在途；不得提前清空 |
| M18 Queue/Steer 预览/编辑/删除/插话 | `QueueDock.tsx`、`submission-policy.ts` | composer 上方 `GtkRevealer` 队列；编辑 `GtkTextView`；Steer 按钮 | `libadwaita/session/prompt`（`prompt`/`steer`）；列表/edit/remove **G10** | 4（直接 steer）/6（完整队列） | Enter/Mod+Enter/Shift+Enter 与 IME 规则照抄 |
| M19 Stop 按钮与双 Esc | `stop-shortcut.ts` | 发送按钮切为停止；`GtkEventControllerKey` 记录同一 Session/turn 双 Esc | `libadwaita/session/cancel`、`agent.stream`、`session.status` | 4 | 默认 500ms；保留 inbox；模态/终端/IME 不触发 |
| M20 失败后“继续生成” | `InputBar.tsx`、`continuation.ts` | 错误卡下 `GtkButton`；仅在六码资格且 composer 空时显示 | `libadwaita/session/prompt`（空内容、prompt 模式）、`session.event` | 4 | 不插入隐藏“继续”文本；资格由 transcript 判定 |
| M21 附件选择/粘贴/拖放/上传进度/重试 | `ui-conversation`、`file-upload` | `GtkFileDialog.open_multiple`、clipboard/DnD、`GtkProgressBar`、失败重试按钮 | 图片经 `libadwaita/session/prompt` base64；通用文件/staged receipt **G11** | 4（图片）/6（通用文件） | PNG/JPEG/WebP/GIF；Host 拒绝不可重试原因要保留 |
| M22 上下文占用率与组成 | `ContextMeter.tsx` | `GtkDrawingArea` 圆环/`GtkLevelBar` + `GtkPopover` 组成 | **G12**（contextPressure/contextBreakdown） | 6 | 容量不可用时关闭旧面板 |
| M23 Todo dock | `TodoPanel.tsx` | `GtkExpander` + 状态点/计数 `GtkLabel` | `libadwaita/session/follow`（todos）、`session.event`（`todo/write`） | 4 | 只读，不做 Todo 编辑器 |
| M24 已发送凭据遮挡与眼睛 | `ui-secret-mask` | 文本块内 mask token + 小 `GtkButton` 眼睛；复制仍取原文 | 无需 bridge | 4 | 只处理上游已识别格式；未发送 composer 不遮挡 |
| M25 Trajectory 表/检查器/时间线/搜索/虚拟行 | `ui-trajectory` | `GtkColumnView` + 右侧检查器 `GtkPaned`；`GtkDrawingArea` 时间线；搜索 `GtkSearchEntry` | `session.event`、`agent.stream`、`libadwaita/session/page`；实时结果 watch **G13** | 6 | 受 developerTools 开关；冷读不承诺实时原始结果 |
| M26 消息赞踩/反馈弹窗 | `ui-message-feedback` | 消息动作 thumbs 按钮 + `AdwDialog` 七类反馈/描述 | **G14** | 5 | CAS 冲突以 Host 返回项对账；失败保留草稿 |
| M27 Chat/Trajectory 视图选择与固定 | `view-selection.ts` | `AdwViewStack`/`AdwViewSwitcher`；更多 `GtkPopoverMenu` | `libadwaita/settings/get`、`libadwaita/settings/mutate`（开发者工具）；选择本地保存 | 4 | Chat 永远固定；插件重载概念不复制 |
| M28 Session 图片共享缓存 | `historical-images.ts` | `GdkTexture` 缓存按 Session 持有，release 时释放 | **G11**（按 attachmentId 授权读取）；`libadwaita/session/release` | 4 | Chat/Trajectory/Queue/Tool 共用，不重复下载 |

## 4. 工具与展示卡片（清单 §3，C01–C18）

| 功能点（清单行） | 上游组件 | libadwaita/GTK 控件与交互 | 所需 bridge 能力 | 阶段 | 备注 |
|---|---|---|---|---|---|
| C01 工具调用树/PTC/通用 fallback | `ToolCallTree.tsx` | 通用卡基类：`GtkFrame`+`GtkExpander`；嵌套缩进 `GtkBox`；准备/执行/失败状态图标 | `agent.stream`、`session.event`（`tool/call/result`、PTC） | 4 | 所有专用卡先回退到此卡；参数结束不等于执行结束 |
| C02 `read` 文件读取卡 | `read-row.tsx` | 行号 `GtkSourceView`，头部路径/范围，点击打开预览 | `session.event`（meta）；打开内容 **G06** | 6 | §5 卡片顺序 2 |
| C03 `read_image` 图片卡 | `read-image-row.tsx` | `GtkFlowBox`+`GtkPicture`，尺寸/字节标签，灯箱 | `session.event`；图片读取 **G11** | 6 | §5 卡片顺序 8 |
| C04 `write/edit/apply_patch` diff 卡 | `file-mutation-row.tsx` | diff 专用 `GtkSourceView` + added/removed tags；多文件 `GtkListView` | `session.event`（arguments、`meta.diffs/path/stopped`） | 6 | apply_patch 不虚构运行中 diff；§5 顺序 3 |
| C05 Bash/Pwsh/terminal_send 输出卡 | `bash-sample.tsx` | 命令/cwd 头部 + 只读等宽 `GtkTextView` + 退出状态胶囊 | `session.event`；无需交互终端 RPC | 6 | 与 N17 交互终端区分；§5 顺序 4 |
| C06 `todo_write` 摘要与 diff | `todo-row.tsx` | 完成计数头部 + 状态分组 `GtkListBox` + 新增/删除/移动标记 | `session.event`、`libadwaita/session/page` | 6 | 早期基线未加载则标不可比较；§5 顺序 5 |
| C07 `grep/glob` 搜索卡 | `search-row.tsx` | 路径/匹配 `GtkListView`，截断说明，文件点击预览 | `session.event`；打开文件 **G06** | 6 | §5 顺序 6 |
| C08 `web_search/web_fetch` 卡 | `web-row.tsx` | 回答 Markdown 块 + 来源 `GtkListBox` + HTTP 状态标签 | `session.event`；系统浏览器打开无需 bridge | 6 | fetch 链接恒外部新打开；§5 顺序 7 |
| C09 `ask_user_question` 工具行 | `ask-question-row.tsx` | 摘要行 +“重开/查看回答”；只读回答复用问题卡 | `user-question/ask`、`session.event`；跨刷新待处理恢复 **G28** | 6 | 取消/中断不虚构答案；§5 顺序 10 |
| C10 控制回执/结构化 details 卡 | `details-row.tsx` | key/value `GtkGrid` + 可展开原始 JSON `GtkSourceView` | `session.event` | 6 | 不支持的数据回退 C01；§5 顺序 9 |
| C11 Auto-review 拒绝优先呈现 | `auto-review-denial.ts` | 警告色 `GtkFrame` + 归一化理由，隐藏原参数 | `session.event`（ToolCallBlock meta） | 6 | 优先级高于专用卡；§5 顺序 1 的规则 |
| C12 改动/交付卡与 review diff | `ui-deliverables` | 改动文件列表 + `AdwTabView` review + diff view；交付文件 `GtkListBox` | `libadwaita/workspace/changes`、`session.event`；文件 stat **G06** | 6 | bridge 须实际挂载 `workspace-changes`；无快照不显示；§5 顺序 12 |
| C13 Cordis 动态插件定义卡 run/stop | `packages/extensions/ui-cordis` | 通用详情卡 + run/stop `GtkSwitch`/按钮与进行中状态 | `session.event`；控制 **G27** | 6 | 不能用 plugin enable/disable 冒充单实例 run/stop；§5 顺序 14 |
| C14 通用 Markdown/终端/diff/读取/搜索渲染器 | `ui-primitives` | §4 渲染块与通用卡基类，统一复制/展开/安全链接 | 无需 bridge | 4 | 所有 C 卡复用，不逐卡重复实现 |
| C15 下载/另存/图片复制 PNG/菜单 | `ui-file-actions` | `GtkFileDialog.save`、复制 `GdkClipboard`、成功 `AdwToast` | 文件 bytes/stat **G06**；本机保存无需 bridge | 6 | Portal 保存是 Web download/saveAs 的原生等价物 |
| C16 拖放字节传递 | `ui-file-drag` | 窗口 `GtkDropTarget` + 全窗口拖放遮罩；文件逐条进 composer | 图片 `libadwaita/session/prompt`；通用文件 **G11** | 4 | 不复制浏览器 token；仅接受当前窗口拖放 |
| C17 在应用中打开 Workspace/文件 | `ui-open-in-app` | `GtkMenuButton` 应用列表 + `GtkFileLauncher`/`GAppInfo` | `libadwaita/session/workdir/get`；文件 stat 可选 **G06**；本机打开无需 bridge | 6 | 应用目录由本机 `GAppInfo` 获取，不调用 Web Host RPC |
| C18 右键菜单框架 | `ui-context-menu` | `GtkPopoverMenu`/`GtkPopover` + 目标对象；异步 action resolve | 无需 bridge；动作自身依赖另列 | 4 | 菜单不触发卡片展开/打开 |

## 5. 审批与提问（清单 §4，A01–A10）

| 功能点（清单行） | 上游组件 | libadwaita/GTK 控件与交互 | 所需 bridge 能力 | 阶段 | 备注 |
|---|---|---|---|---|---|
| A01 工具审批卡接管 composer | `ui-approval` | inline `GtkFrame`：理由、关联命令、允许一次（suggested）/拒绝（destructive）；一次性锁 | `approval/request`、`session.event` | 4 | 无 always-allow；断线/取消 fail-closed |
| A02 审批 Enter/Esc | `ui-approval` | `GtkEventControllerKey`；焦点在详情时 Enter 允许、Esc 拒绝 | `approval/request` | 4 | 输入控件/IME/按钮自身 Enter 保留原生行为 |
| A03 审批风险说明浮层 | `ApprovalWarning.tsx` | 非模态 `GtkPopover`，hover/focus/点击打开 | `approval/request`；扩展 warning/displayReason **G28** | 4 | Esc/失焦只关 popover，不拒绝请求 |
| A04 通用提问卡/选项/自定义/翻页/跳过 | `QuestionComposer.tsx` | inline 卡：`GtkCheckButton` 单/多选、`GtkTextView` 自定义、前后翻页与跳过 | `user-question/ask`、`user-question.cancelled` | 4 | 一次仅一个请求占 composer；选项草稿本地保存 |
| A05 提问倒计时/聚焦暂停/超时后作答 | `ui-user-questions` | `GtkLabel` 倒计时 +“慢慢回答”；超时保留可答卡 | `user-question/ask`、`user-question.cancelled`；remainingMs/等待状态 **G28** | 4 | 迟到响应可能 `-32005`，不得假报成功 |
| A06 迟到回答/草稿恢复/关闭重开 | `ui-user-questions` | 草稿存应用私有状态；关闭仅隐藏有 callId 卡；工具行重开 | `user-question/ask`、`user-question.cancelled`、`session.event`；可恢复等待快照 **G28** | 4 | 无 callId 关闭回 `{cancelled:true}` |
| A07 已回答气泡与只读回看 | `QuestionReplyView.tsx` | 右对齐气泡 + 每题只读列表、复制按钮 | `session.event`（tool result、late user/message） | 6 | 原生/PTC/late 仅生成一个气泡 |
| A08 Plan review 专用卡 | `PlanReviewPanel.tsx` | inline 卡：摘要/全文、“同意执行” suggested、“要求修改”普通 | `user-question/ask` | 4 | 同意回 intent.approve label；要求修改必须回 `{cancelled:true}`；**不是 approval** |
| A09 Plan 模式徽章与退出 | `PlanModeControl.tsx` | composer 上方 `GtkLabel` 徽章 + 退出按钮；等待投影确认 | `libadwaita/session/follow`（planMode）、`session.event`；`/plan off` 命令 **G15** | 4 | 失败不乐观撤下徽章 |
| A10 已提交计划卡/完整计划预览 | `ui-plan` | 计划卡 + 右侧文档 tab，Markdown 渲染；历史分页加载 | `session.event`（`exit_plan_mode`）、`libadwaita/session/page` | 6 | 待审计划自动打开每客户端一次；§5 顺序 11 |

## 6. 设置与模型（清单 §5，S01–S20）

| 功能点（清单行） | 上游组件 | libadwaita/GTK 控件与交互 | 所需 bridge 能力 | 阶段 | 备注 |
|---|---|---|---|---|---|
| S01 设置外壳/配置表单/冲突保留草稿 | `ui-settings*` | `AdwPreferencesDialog`/Page/Group；各 schema 对应 Switch/Entry/SpinRow | `libadwaita/settings/get`、`libadwaita/settings/mutate`；schema/revision/更新通知 **G16** | 5 | 无 schema 时不得把任意 JSON 当完整表单等价物 |
| S02 语言选择 | `locale/LanguageRow.tsx` | `AdwComboRow` 8 语言，立即重载字典 | `libadwaita/settings/get`、`libadwaita/settings/mutate` | 5 | 无偏好跟随系统再回退英文 |
| S03 代码工作工具开关 | `ui-settings` | `AdwSwitchRow` | `libadwaita/settings/get`、`libadwaita/settings/mutate` | 5 | 控制 Trajectory/开发预览等 |
| S04 连接恢复/服务版本变化/版本行 | `ui-settings-general` | `AdwBanner`、`AdwAlertDialog`、版本 `AdwActionRow` | `libadwaita/initialize`（dshVersion/serverInfo/generationId）；socket 状态 | 4 | release 变化在重连握手后检测；刷新动作=重连并 rebaseline |
| S05 打开配置文件 | `ui-settings-general`、settings-controller | `GtkFileLauncher` 调系统编辑器 | `libadwaita/settings/get`；GUI 自有 profile 路径，无需新 bridge | 5 | 仅本机；不下载明文配置 |
| S06 Models provider 行/编辑/排序/增删 | `ui-settings-models` | `AdwPreferencesGroup` provider 卡、`AdwEntryRow`、`AdwComboRow`、拖放/键盘排序 | `libadwaita/model/catalog`、`libadwaita/settings/get/mutate`；模型发现 **G17**；表单 schema **G16** | 5 | 一次展开一张；发现草稿 key 不落日志 |
| S07 API Key 只写/校验/保存/删除范围 | `ApiKeyInput.tsx`、operations | `AdwPasswordEntryRow` + 显示切换；保存中 spinner | `libadwaita/credentials/status/inject/rotate/remove`、`libadwaita/settings/mutate` | 5 | 不回读；普通留空=保留；remove 只用于上游允许的自有 provider 删除流程 |
| S08 首启 DeepSeek 引导与版本说明 | `DeepSeekOnboardingDialog.tsx`、`WelcomeNotice.tsx` | `AdwDialog` + `AdwStatusPage` + key 表单；版本说明确认 | `libadwaita/credentials/status/inject`、`libadwaita/model/catalog`、`libadwaita/settings/get/mutate` | 5 | 无可用 provider 才引导；详细流程 §7.9 |
| S09 DeepSeek 账号登录/资料/余额/充值 | `ui-settings-account` | 账号 `AdwPreferencesPage`、登录 `AdwDialog`、头像 `GtkImage`、余额行 | **G18**；API key 路径仍用现有 credentials 方法 | 5 | 浏览器 OAuth/账号记录不能由 key inject 冒充 |
| S10 Agent loop 并行上限 | `ui-settings-agent-loop` | `AdwSpinRow` | `libadwaita/settings/get/mutate`；schema **G16** | 5 | Host 无 namespace 时隐藏 |
| S11 子代理深度/并行/模型设置 | `ui-settings-subagent` | `AdwSpinRow` + 模型 `AdwComboRow` | `libadwaita/settings/get/mutate`、`libadwaita/model/catalog`；schema **G16** | 5 | 两次独立写入保持独立错误状态 |
| S12 Web 搜索/获取 provider 与结果数 | `ui-settings-web-search` | `AdwComboRow`、`AdwSpinRow`、key 行 | `libadwaita/settings/get/mutate`、credentials 方法；provider 列表 **G17** | 5 | 凭据跟随会话语义需 bridge 对齐 |
| S13 终端超时/输出上限 | `ui-settings-shell` | `AdwSpinRow` | `libadwaita/settings/get/mutate`；schema **G16** | 5 | 只编辑设置，不等于终端控制 |
| S14 会话日志上传开关 | `ui-settings-session-log` | `AdwSwitchRow` + 说明/警告 | `libadwaita/settings/get/mutate` | 5 | Host 无 namespace 时不可用 |
| S15 默认权限与 `/permission` 会话选择 | `ui-permission-presets` | 默认 `AdwComboRow` + composer `GtkDropDown`/`GtkPopover` | 默认设置可用 settings 方法；权限目录/会话权限 **G19** | 5 | 默认只影响未来 Session |
| S16 Agent Preset 卡/默认值/YAML | `ui-agent-preset` | `GtkFlowBox` preset 卡 + 默认 `AdwComboRow` + YAML `GtkSourceView` | **G20**；创建时 `libadwaita/session/create(agentPreset)` | 5 | Web 不编辑 preset，GTK 也不新增编辑器 |
| S17 Goal 条带编辑/暂停/恢复/清除 | `ui-goal` | composer 上方 `GtkRevealer` + 文本/动作按钮 | **G21**；goal 投影可并入 **G12** | 6 | Host 状态与 preset 无关 |
| S18 `/model` 与会话模型切换 | `ui-model-selection` | composer `GtkDropDown` + 搜索 `GtkPopover`；reasoning `GtkDropDown` | `libadwaita/model/catalog`、`libadwaita/session/model/get/set` | 4 | 当前 step 不变，后续请求生效；失败保留旧选择 |
| S19 主题/字号/字体 | `ui-theme` | 外观 `AdwComboRow`、字号 `AdwSpinRow`、字体 `GtkFontDialogButton`；`AdwStyleManager` 跟随系统 | `libadwaita/settings/get/mutate` | 5 | 深浅色/强调色/减少动效优先跟随 GNOME；不复制 CSS token |
| S20 快捷键速查/录制/恢复 | `ui-shortcuts` | `AdwDialog` + `GtkSearchEntry` + `GtkListView`；`GtkEventControllerKey` 录制 | 无需 bridge；键位存 GUI 私有设置 | 5 | `Mod+/`；固定项只读；冲突检测与 IME 保护 |

## 7. 插件与凭据（清单 §6，P01–P12）

| 功能点（清单行） | 上游组件 | libadwaita/GTK 控件与交互 | 所需 bridge 能力 | 阶段 | 备注 |
|---|---|---|---|---|---|
| P01 插件列表/状态诊断/刷新 | `PluginManagerPage.tsx` | 全局 plugins page：`GtkListView` 卡 + 状态标签 + 刷新按钮/骨架 | `libadwaita/plugin/list`；完整 Loader 诊断 **G24** | 5 | 无受管 profile 时显示不可用，不显示空列表 |
| P02 Bundle/组件启停与 restart-required | `manager-store.ts` | `GtkSwitch` + 进行中；`AdwBanner`“需要重启/立即重启” | `libadwaita/plugin/enable`、`libadwaita/plugin/disable`、`libadwaita/plugin/list`；重启用 `libadwaita/shutdown` 后 supervisor 冷启动+initialize | 5 | 流程见 §7.8；不得提示已热生效 |
| P03 插件配置表单 | `PluginManagerPage.tsx` | 插件详情 `AdwPreferencesPage` + 动态 rows | `libadwaita/settings/get/mutate`；schema **G16**；完整清单 **G24** | 5 | Bundle/row/官方配置命名空间分别保存 |
| P04 Bundle 详情文档与错误重试 | `detail-store.ts` | `AdwDialog`/详情页 + Markdown 渲染 + 图片 + 重试 | **G22** | 5 | 外部图片需明确隐私边界 |
| P05 添加插件与 inspect 预检 | `PluginManagerPage.tsx` | `AdwDialog` 来源输入 + 类型/安全说明 + inspect 结果 | **G22** | 5 | npm/Git/archive/绝对路径；明确以用户权限运行 |
| P06 安装进度/日志/取消/恢复/构建审批 | `manager-store.ts` | `AdwDialog`：阶段、`GtkProgressBar`、只读日志 `GtkTextView`、取消/恢复/审批 | **G22** | 5 | 一次一个任务；应用阶段不可取消等状态不可简化 |
| P07 卸载/更新/版本检查与恢复 | `versions.tsx` | 卸载 `AdwAlertDialog`；更新 `AdwDialog` 版本列表与恢复状态 | **G22**；启停仍用现有 enable/disable | 5 | 更新可能先删后装；失败不得谎称保留旧版 |
| P08 安装源选择/记忆/探测 | `manager-store.ts` | registry `GtkDropDown` + 自定义 `AdwEntryRow` + 测速状态 | **G22** | 5 | GUI 可本地记忆选择；私有凭据不进入页面 |
| P09 插件市场索引/搜索/分页/安装确认 | `ui-plugin-market` | “发现” `AdwViewStackPage`：索引管理、搜索、筛选、卡片 `GtkFlowBox` | **G23**；安装复用 **G22** | 5 | 第三方 HTTP(S) 索引与信任提示必须保留 |
| P10 设置→内置插件只读清单/页面重试 | `ui-settings-plugins`、`ui-settings-plugin-inventory` | `AdwPreferencesPage` + tabs + 搜索/展开只读 rows | `libadwaita/plugin/list`；完整 inventory/retry **G24** | 5 | 重试页面模块的概念不照搬；GTK 对应重连/重载 profile 状态 |
| P11 凭据 describe/set/unset 与完整 provider seam | credentials service | API key 行复用 S07；账号授权记录显示在账号页 | API key：`libadwaita/credentials/status/inject/rotate/remove`；record/OAuth grant **G29** | 5 | secret 永不回读；Secret Service 与 bridge 内存双层边界不变 |
| P12 入口定制/Pin/插件草稿 | `ui-entry-customization` | 区域“更多” `GtkPopoverMenu`、Pin 本地排序；创建草稿切 conversation composer | `libadwaita/session/create`；插件目录 `libadwaita/plugin/list`；草稿无需 bridge | 6 | 只追加草稿，不发送消息 |

## 8. 其他外围（清单 §7，O01–O16）

| 功能点（清单行） | 上游组件 | libadwaita/GTK 控件与交互 | 所需 bridge 能力 | 阶段 | 备注 |
|---|---|---|---|---|---|
| O01 启动页/失败报告 | `web/boot-page.ts` | `AdwStatusPage` + `GtkSpinner`；失败 `GtkListBox` 逐项显示插件/阶段原因 | `libadwaita/initialize`；supervisor stderr 诊断 | 4 | Web 插件审计由 dsh/profile 启动错误与 bridge 握手等价呈现 |
| O02 浏览器 token/Cookie/信任检查 | `connection/browser-auth.ts` | **有意不移植 Web 认证 UI**；启动页只显示本机连接状态 | Unix socket 0600 + `libadwaita/initialize` | 4 | 原生等价物是进程边界与文件权限，无 Cookie/HTTP token |
| O03 客户端模块系统/bundle 路由/页面同步重试 | `client/modules` | **有意不移植 React 模块系统**；插件页显示 dsh 插件状态 | `libadwaita/plugin/list`；重连 `libadwaita/initialize` | 5 | GTK widget 静态链接；dsh 插件仍由 plugin 方法管理 |
| O04 HMR/开发 bundle 重建 | `client/hmr` | **有意不移植页面 HMR**；配置变更显示重启 banner | `libadwaita/plugin/enable/disable`、`libadwaita/shutdown`、重新 initialize | 5 | 本 profile 明确禁 HMR；不假装支持热替换 |
| O05 RPC/WebSocket/Fetch 路由 | `client/connection` | **有意不移植 Web 传输**；`BridgeConnection` 统一 NDJSON | Unix socket 上现有 `libadwaita/*`、通知与双向请求 | 4 | 无第二条 Web Host 私有 RPC 通路 |
| O06 Worker/流式上传/receipt | `file-upload` | GUI 读取文件并显示进度；图片直接组 prompt content block | 图片 `libadwaita/session/prompt`；通用文件/staged receipt **G11** | 4（图片）/6（通用文件） | 不复制 Worker；大文件需要流式桥避免超大 base64 |
| O07 快捷键引擎/持久化 | `client/shortcuts` | `GtkShortcutController`、`GtkEventControllerKey`、应用键位配置 | 无需 bridge | 4 | 默认键见 §7；输入区域/终端/模态仲裁照抄 |
| O08 Locale 词典/fallback | `client/locale` | Rust i18n 字典 + 系统 locale；语言行见 S02 | `libadwaita/settings/get/mutate` | 5 | 动态插件文案若无法翻译，保留上游原文并标缺口而非硬编码中文 |
| O09 observable store/useResource | `client/store`、`resources` | Rust 领域对象 + `glib::Sender`/信号 + `GtkListModel`；tab 持有资源 handle | 各领域现有方法；无需另设 store RPC | 4 | 不把业务真相复制到 widget；release 时取消订阅 |
| O10 React renderer/Slots | `ui-renderer`、`ui-slots` | **有意不移植 React/Slot 运行时**；按本文固定组成原生 widget | 无需 bridge | 4 | 仅移植信息架构与扩展位置，不引入 JS 运行时 |
| O11 Desktop 产品埋点 | `product-analytics` | **有意不移植** | 无需 bridge | 6 | 上游普通 Web 不上报；非用户功能且避免新增遥测面 |
| O12 会话后台 jobs 列表/输出/双击停止 | `ui-jobs` | 会话头部 jobs `GtkMenuButton` + 输出 `GtkTextView`；3 秒内第二击确认停止 | **G25** | 6 | 清空只隐藏本 GUI 行，不删 Host 记录 |
| O13 自动化计划任务 CRUD/运行/历史/时钟 | `ui-schedule` | 全局任务页 + `AdwDialog` 编辑器 + 历史 `GtkListView` + 时钟标签 | **G26** | 6 | 持久 schedule 与 jobs 不混用 |
| O14 PWA/manifest/子路径挂载 | `apps/web` manifest | **有意不移植 PWA**；以应用图标、`.desktop`、D-Bus 激活替代 | 无需 bridge | 6 | 属阶段 7 打包落地；无浏览器安装/URL scope 概念 |
| O15 系统通知与提示音 | Web 无对应；Desktop 更新提醒仅作对照 | `GNotification` 完成提醒；声音用 GNOME 提示音能力，可设置关闭 | `session.status`（非主视图 running→idle） | 6 | **下游桌面增强，不伪称上游 Web 功能**；阶段 7 做桌面联调 |
| O16 测试支撑包 | `test-runtime` | **无 UI 可移植**；用 Rust widget 测试与 bridge 集成测试覆盖 | 无需 bridge | 6 | 不在产品窗口放置测试入口 |

## 9. 展示卡片清单与阶段 6 顺序

### 9.1 共用卡片结构

- 外框：`GtkFrame`（无重阴影）→ `GtkBox(vertical)`；头部为可键盘激活的 `GtkButton`/`GtkExpander`，包含工具名、路径/域名、状态 `GtkSpinner`/`GtkImage`、结果胶囊。
- 正文：`GtkRevealer` 控制展开；长输出放 `GtkScrolledWindow`，复制按钮直接写 `GdkClipboard`；原始 JSON 使用只读 `GtkSourceView`。
- diff：只读 `GtkSourceView`，逐行 added/removed/context `GtkTextTag`，路径与 hunk 头独立标签；折叠时遵守上游最多 9 行，不从运行中 `apply_patch` 虚构 diff。
- 状态：准备/执行/完成/失败/中断严格由事件推导；数据不符专用 schema 时回退通用卡，不丢原始 arguments/result。

| 顺序 | 卡片（清单行） | GTK 组成（头部/状态/正文） | 数据源（事件/meta/bridge） | 阶段 6 完成标准 |
|---|---|---|---|---|
| 1 | 通用工具树 + Auto-review 优先规则（C01/C11） | 工具名+call 状态；嵌套 PTC；原始输入/输出；拒绝警告卡 | `agent.stream` tool-call-delta；`session.event` `tool/call`、`tool/result`、`tool/ptc-dispatch*`；Auto-review meta | 任意未知工具均可展开、复制、Inspect；拒绝时隐藏参数 |
| 2 | `read`（C02） | 路径/读取范围；成功或错误；带行号源码窗口 | 调用 `file_path/offset/limit`；结果 `meta.path/lines/totalLines/lang` | 行号与 offset 一致；点击在右侧预览对应行（G06） |
| 3 | `write` / `edit` / `apply_patch`（C04） | 路径、多文件列表、输入 KB/停止状态；diff 行标签 | 流式 arguments；完成 `meta.diffs/path/stopped` | write/edit 运行中可显示预期 diff；完成后以已应用 meta 为准；apply_patch 不虚构 |
| 4 | `bash` / `pwsh` / `terminal_send` 输出卡（C05） | 命令、cwd、running/background；等宽输出与退出码/信号胶囊 | 调用 `command/workdir/run_in_background`；结果文本 `[exit code:…]`/signal；`meta.cwd` | 非零、信号、无退出码状态准确；不推断后台进程仍存活 |
| 5 | `todo_write`（C06） | done/total、首个进行中项、`+n`；新增/状态/移动/删除列表 | `todo/write`、`tool/call`、PTC；历史基线 `libadwaita/session/page` | 未加载早期基线时明确“不可比较”；dock 与卡片数据一致 |
| 6 | `grep` / `glob`（C07） | pattern/include/path；文件/行匹配；截断说明 | 调用参数；`meta.shape/files/paths/total/truncated` | 空 include、非法字段回退通用卡；恢复定位文本保留 |
| 7 | `web_search` / `web_fetch`（C08） | 查询/URL/status；回答 Markdown、来源链接、截断说明 | 调用 `queries/url`；`meta.answer/sources/url/statusCode/truncated` | 仅 root 成功且 metadata 完整用专用卡；fetch 链接外部打开 |
| 8 | `read_image`（C03） | 路径、尺寸/原尺寸、字节数；图片网格与原图灯箱 | 结果 image blocks attachment refs；`meta.path`；**G11** 读取 | 图片不从 meta 凭空复制；失败保留通用文本 |
| 9 | 控制回执/details（C10） | 实体类型与调用结果；key/value 网格 + 原始 JSON | 已记录 arguments/result/metadata | goal/schedule/subagent/job/LSP 等能展示；不支持数据回退 |
| 10 | `ask_user_question` 工具行（C09） | 问题摘要、可回答/已答/取消状态；重开或只读回答 | `user-question/ask`、`session.event` tool result/late reply；跨刷新恢复 **G28** | 原生/PTC/late 只一张只读记录；不虚构答案 |
| 11 | 计划卡（A10） | 首个 Markdown `#` 标题、审批后仍保留；右侧完整 Markdown | `exit_plan_mode.arguments.plan`；历史 `session/page` | 无标题/参数无效时回退通用工具行；分页可恢复全文 |
| 12 | 改动文件/交付卡（C12） | 文件增删统计、review tabs、diff、交付文件按钮 | `libadwaita/workspace/changes` summary/diff；`session.event` 文件引用；stat **G06** | 指定 turn/fileIndex 可查看；无快照不显示空卡 |
| 13 | 命令/压缩/重试/错误/配额状态卡（M13） | 图标、状态、摘要、可展开诊断；配额动作 | `session.event` `command/*`、`compaction/*`、`llm/retry*`、`turn/end`；配额动作 **G18** | 历史不错误重复弹窗；成功隐藏项与上游一致 |
| 14 | Cordis 定义卡（C13） | 插件定义摘要、run/stop 状态、诊断；原始详情 | `cordis_define` 工具事件；控制 **G27** | 控制结果回读真实状态；不把 Bundle 开关当实例开关 |
| 15 | 附件图片卡（M07/M28/C03 共用） | 文件名/类型/大小、进度/失败、画廊/灯箱 | prompt content blocks；`session.event` attachment refs；读取 **G11** | 缓存按 Session 释放；上传失败可重试规则一致 |

## 10. Markdown、代码、数学与文档技术选型

### 10.1 Markdown 主路径（阶段 4）

1. `pulldown-cmark` 开启 GFM tables/footnotes/strikethrough；解析为应用自有块模型，不把 HTML 字符串交给 widget。
2. 段落/标题/列表/引用/行内样式用只读但可选择文字的 `GtkTextView` + `GtkTextTag`；链接使用白名单 scheme，激活时走统一 LinkController。
3. 表格用 `GtkColumnView`；简单表格可用 `GtkGrid`，长表格必须横向滚动，不压缩代码列。
4. 代码块独立只读 `GtkSourceView`，不在一个巨大 TextBuffer 中混排整个 transcript；消息块由 `GtkListView` 虚拟化。
5. 流式只保留并解析尾部未完成块，fence 闭合后冻结；持久 `assistant/message` 到达时原子替换 live widget。
6. 原始 HTML 不渲染、不执行；`javascript:`/`vbscript:`/危险 `data:`/`blob:` 链接拒绝激活。

### 10.2 GtkSourceView 5 本机检测（已执行）

```text
$ pkg-config --modversion gtksourceview-5
5.22.0
```

结论：**开发文件可用，采用 GtkSourceView 5 主方案，无需纯 GTK fallback。** 若目标构建环境缺失，后备为只读 `GtkTextView` + 手工 `GtkTextTag` 关键字着色、保留等宽/复制/行号，不阻止普通代码块显示；不得以“无高亮”为由嵌 WebView。

### 10.3 TeX（阶段 6，有意简化）

- 首版保留原始 TeX 的等宽显示、流式字面显示与选区复制 `$…$`/`$$…$$`，这与上游数据一致。
- **不声称完成 KaTeX 视觉排版**：GTK 无现成 MathML 渲染控件，嵌入 KaTeX/WebView 又违反项目约束。后续若验收要求像素级数学排版，应单独评审离线 TeX→SVG 原生管线及依赖，而不是悄悄降级或引浏览器。
- 行内常用 Unicode 数学字符仍按普通文本显示；不做不完整的“正则转图片”。

### 10.4 文档与重型内容

| 内容 | 决策 |
|---|---|
| PDF | 采用 Poppler GLib 渲染页到 `GdkTexture`/`GtkPicture`，提供页码/缩放；依赖在构建阶段确认 |
| Office | 经 **G07** 转 PDF 后复用 Poppler；不可转换时给系统默认应用打开，不显示空白预览 |
| HTML | **不内嵌渲染**；可显示源码/安全文本提取，完整页面交给系统浏览器，避免 WebView 与远端脚本 |
| 远程图片 | 安全 HTTP(S) 客户端下载为 `GdkTexture`，限制大小/类型；失败显示占位与原始链接 |
| 交互终端 | 右栏采用 VTE 4；工具结果卡仅需只读 `GtkTextView`，两者不混用 |
| 鲸鱼动画 | `GtkDrawingArea` 简化动画，遵循减少动效；不做 Canvas 逐像素移植 |

## 11. 交互流程对照（实现时按此走查）

### 11.1 创建 / 恢复 / Fork / 归档（N03–N15、M09）

1. **创建**：New Session → 若无 cwd，`GtkFileDialog.select_folder` → `libadwaita/session/create{cwd,agentPreset?}` → `follow`；同 cwd/preset blank 可复用。
2. **恢复**：点击冷列表行 → `follow{sessionId}` 获取 header/latestSeq/projections/assistantBaseline → 需要更早历史时 `page`；选择状态本地保存供下次启动恢复。
3. **Fork**：完成轮次菜单 → **G03**（需传 sessionId/atSeq/increaseTitle）→ 成功返回新 ID 后 `follow` 并切主视图。协议补齐前按钮标不可用，不能复制 transcript 冒充。
4. **归档**：活动会话先 `AdwAlertDialog` → `libadwaita/session/archive{archived:true}`；bridge 先停止并丢 pending inbox；取消归档用同方法 `archived:false`。
5. **关闭视图**：仅 `libadwaita/session/release`，不停止 Agent、不归档。

### 11.2 提交 / Queue / Steer / Stop（M17–M20）

1. 空闲 Enter：`session/prompt{contentBlocks,mode:"prompt"}`；收到 `messageId` 后才清草稿，失败恢复。
2. 运行中 Enter 默认 Queue；Mod/Ctrl+Enter 采用互补模式；直接插话用 `mode:"steer"`；Shift+Enter 换行，IME Enter 不提交。
3. QueueDock 的列表、纯文本编辑、删除、全部转 steer 依赖 **G10**；未补齐前只实现协议已有的直接 prompt/steer，并明确功能缺失。
4. Stop 按钮或同一 Session/turn 内双 Esc（默认 500ms）→ `libadwaita/session/cancel`；收到 abandoned/end/idle 后清 live 临时块，Queue 保留。
5. 六码失败且符合上游空闲条件时显示“继续生成”，调用空 contentBlocks 的 prompt，不注入隐藏文本。

### 11.3 审批（A01–A03）

1. 收到 server request `approval/request`，按 sessionId 路由到对应 composer；其他会话只显示 pending 状态点。
2. 详情聚焦 Enter → 回同 request id `{outcome:"allowed-once"}`；Esc/拒绝 → `rejected`；关闭/取消 → `cancelled`。
3. 每次请求只允许一个响应；断线、超时、GUI 退出由 bridge fail-closed。风险 popover 关闭不响应审批。
4. 通用审批没有 always-allow；warning/displayReason 等扩展字段需 **G28**。

### 11.4 用户提问与 Plan review（A04–A10、C09）

1. `user-question/ask` 接管 composer；单选/多选/自定义答案逐题保存草稿，末题一次提交全部 answers；跳过为 `selected:[]`。
2. 有 callId 的关闭只隐藏，可从工具行重开；无 callId 的关闭回 `{cancelled:true}`；`user-question.cancelled` 强制关闭。
3. 超时后仍提交时按同一 server request id 回复；若 `-32005 QUESTION_ALREADY_SETTLED` 则显示已结算，不假报成功。remainingMs/跨刷新恢复需 **G28**。
4. `intent.kind=="plan-review"` 使用专用卡：“同意执行”回 `selected:[intent.approve]`；“要求修改”回 `{cancelled:true}`，保持 plan mode。
5. Plan 徽章来自 follow/projections 或 `plan/mode` 事件；退出 `/plan off` 需 **G15**，等待 Host 确认后撤下。

### 11.5 模型切换（S06、S18）

1. 打开模型菜单 → `libadwaita/model/catalog`；当前选择由 `session/model/get` 读取。
2. 选择 provider/model/reasoningEffort → `session/model/set`；按钮保持 busy 至响应，失败恢复旧选择。
3. 当前 step 继续旧模型，新配置从后续请求生效；provider 设置变更后刷新 catalog，不伪造暂时不可路由模型的可用性。

### 11.6 工作目录 / Workspace 切换（N15）

1. `GtkFileDialog.select_folder` 选目标目录。
2. 保存当前 composer 草稿到本地，调用 `libadwaita/session/create{cwd:new}`；成功后 `follow` 新会话并迁移草稿。
3. 对旧视图调用 `libadwaita/session/release`；**绝不调用/新增 `workdir/set`**。旧会话仍保留在原 cwd。
4. 空 Workspace 的持久层级、重命名、删除需要 **G01**；在此之前只支持由 Session cwd 派生的工作区视图，并标缺失。

### 11.7 设置编辑（S01–S20）

1. 打开 `AdwPreferencesDialog` → `settings/get`；按 namespace 建立草稿。
2. 保存 → `settings/mutate{patch}`，成功后用返回 settings 更新；失败保留草稿。
3. 上游 revision 冲突语义与动态 schema 需要 **G16**；协议补齐前不得声称支持并发冲突检测，也不能用脱敏整文档覆盖 secret。
4. 语言/主题立即在 GUI 预览，但保存失败要显示错误；Host 不提供的 namespace 隐藏或显示不可用。

### 11.8 插件启停与 restart-required（P01–P10）

1. `plugin/list` 渲染 → 用户切换 → `plugin/enable` 或 `plugin/disable`。
2. 返回 `restartRequired:true` 时保持状态标记并显示 `AdwBanner`：“插件配置已更改，需要重启 dsh 才能生效”，提供“立即重启”。
3. 立即重启由 GUI supervisor 执行：`libadwaita/shutdown` → 等进程退出 → 冷启动同一 profile → `libadwaita/initialize` 检查新 generationId → 对打开 Session 全部 `follow` rebaseline。
4. `applied/overridden/restart-required` 等完整结果语义、安装/更新/市场分别需要 **G22/G23/G24**；不得把保存成功显示为运行成功。

### 11.9 首启凭据与轮换（S07–S09、P11）

1. dsh 无 key 仍启动并完成 `initialize`；`credentials/status` + `model/catalog` 判断无可用 provider 时显示首启 `AdwDialog`，不阻塞主进程。
2. 用户输入 API key → GUI 写入 Secret Service → `libadwaita/credentials/inject`；成功刷新 catalog 并关闭向导。明文不写日志、命令行、配置文件或环境变量。
3. “稍后配置”关闭当前步骤；版本化 Web 说明的已确认版本写 `settings/mutate`。
4. 设置页轮换：Secret Service 更新 → `credentials/rotate`，不重启、不中断会话；删除按上游所有权流程用 `credentials/remove`。
5. DeepSeek 浏览器登录、账号资料/余额/OAuth record 需 **G18/G29**；在补齐前仅 API key 首启可标已实现，账号页标缺失。

## 12. Bridge 缺口汇总（交给 bridge worker）

> 下列名称是**建议新增契约名**，用于路由工作，不应被 GUI 当作现有方法调用。每项需同步更新 `docs/bridge-protocol.md` §5/§7/§8、类型定义与集成测试后，才能从缺口清单移除。

| ID | 影响清单行/阶段 | 缺失能力与建议契约 | 最低验收要求 |
|---|---|---|---|
| G01 | N03/N15；4 | Workspace 目录/层级持久化：`libadwaita/workspace/list|create|rename|remove`（建议） | 支持空 Workspace、cwd 幂等创建、重命名/删除；切换仍创建新 Session，不加 workdir/set |
| G02 | N10；4 | 手动标题：`libadwaita/session/rename` | user source pin；成功发 `session.title` |
| G03 | N12/M09；6 | Fork：`libadwaita/session/fork` | 传 atSeq/increaseTitle；返回真实新 Session，可 follow |
| G04 | N03/N13；6 | 全文搜索：`libadwaita/session/search` | provider 不可用与真空结果可区分；冷搜索不恢复 Agent |
| G05 | N17；6 | 交互终端：`libadwaita/terminal/create|input|resize|close|list` + `terminal.output` 通知 | shell 选择、恢复、重命名、主题无关的字节流；关闭 tab 终止进程 |
| G06 | N18/N19/C02/C07/C12/C15/C17；6 | Workspace 文件：`libadwaita/workspace/files/list|stat|read|readBytes` | 限制根目录逃逸；支持文本/字节、文件大小与定位；Office 原始文件可读取 |
| G07 | N19；6 | Office→PDF：`libadwaita/document/convert`（建议） | 返回有界 PDF bytes/job；失败分类；不依赖 Web Host 私有路由 |
| G08 | N21；6 | 子代理：`libadwaita/subagent/list|prompt|interrupt` + 目录通知 | 血缘、运行状态、continuable 限制、真实 interrupt |
| G09 | N22；6 | Agent Team：`libadwaita/agent-team/get|set-enabled` + 任务投影通知 | 新 Session 默认关闭；共享任务状态可恢复 |
| G10 | M02/M18；4/6 | Inbox/Queue：`libadwaita/session/queue/list|edit|remove|steer` + `session.inbox` 通知 | 纯文本编辑、附件顺序、FIFO、竞态收敛；普通 Stop 保留，归档丢弃 |
| G11 | M07/M21/M28/C03/O06；4/6 | 附件：`libadwaita/attachment/upload|read`（可流式）+ 进度通知 | 通用文件与图片 receipt、按 attachmentId 授权读取、取消/失败重试；不把大文件全转 base64 |
| G12 | M11/M12/M22/S17；6 | 标准投影：`libadwaita/session/projections` + `session.projection` 通知 | turnOutline/sessionStats/tokenUsage/contextPressure/contextBreakdown/goal 等 baseline+增量，重连可恢复 |
| G13 | M25；6 | 实时规范工具结果：`libadwaita/tool-result/watch` + 结果通知 | 仅选中项订阅；冷读/淘汰明确 unavailable，不伪造历史原始值 |
| G14 | M26；5 | 反馈：`libadwaita/message-feedback/list|put|delete`、`session-feedback/record` | ifVersion CAS、冲突返回权威 item、失败保留草稿 |
| G15 | M16/A09；4/6 | 命令/技能/引用：`libadwaita/command/list|execute`、`skill/list`、`reference/list` | `/plan off`、slash 参数/附件声明、命令 chip 与目录事件可恢复 |
| G16 | S01/S03/S06/S10–S13/P03；5 | 设置 schema/revision：`libadwaita/settings/describe`、mutate expectedRevision + `settings.updated` | 动态表单描述、原子 revision 冲突、secret redaction 安全、按 namespace 订阅 |
| G17 | S06/S12；5 | 草稿模型发现/搜索 provider：`libadwaita/model/discover`、`web-search/providers` | 可用未保存配置/一次性 key 查询；不写凭据；错误/空目录可区分 |
| G18 | S09/M13；5 | 账号：`libadwaita/account/watch|start-sign-in|cancel-sign-in|sign-out|profile|balance` + 通知 | 登录各阶段、取消/过期、任务中退出确认、余额失败不显示零、配额动作 |
| G19 | S15；5 | 权限：`libadwaita/permission/catalog|get|set` + 会话投影 | 默认只影响未来会话；当前会话权限与 Host 一致 |
| G20 | S16；5 | Agent presets：`libadwaita/agent-preset/list|read|get-default|set-default` | 内置/自定义、YAML 只读、创建 Session 时可选择 |
| G21 | S17；6 | Goal：`libadwaita/goal/get|edit|pause|resume|clear` + 通知 | Host 状态与 preset 无关；操作后投影一致 |
| G22 | P04–P08；5 | Bundle 详情与安装生命周期：`plugin/detail|inspect|install|cancel-install|wait-install|remove|versions|update|registries` + install 状态/日志通知 | 所有来源、单任务、取消边界、丢响应恢复、构建审批、更新恢复与 restart-required |
| G23 | P09；5 | 市场：`libadwaita/plugin-market/list-indexes|set-indexes|search|read-detail` + changed 通知 | 索引校验、搜索/分页/版本兼容、信任提示、精确来源安装 |
| G24 | P01/P03/P10；5 | Loader 清单/诊断：`libadwaita/plugin/inventory|retry` + changed 通知 | pending/loading/failed/unloading、来源/条件、全局与 preset 分组、重试不改 Host enable |
| G25 | O12；6 | Jobs：`libadwaita/job/list|follow|kill` + 输出通知 | roster 常驻、可见时输出流、双击停止、清空仅本地隐藏 |
| G26 | N24/O13；6 | Schedule：`libadwaita/schedule/catalog|create|edit|run|history|delete|delivery-limits` + changed 通知 | 创建/编辑/暂停/恢复/立即运行/历史、CAS 冲突、投递到目标 Session |
| G27 | C13；6 | Cordis 定义实例：`libadwaita/cordis-control/run|stop|status`（建议） | 控制真实动态实例；与 Bundle enable/disable 明确区分 |
| G28 | A03/A05/A06/C09；4/6 | 交互扩展元数据与恢复：approval displayReason/warning/detail；question remainingMs/attachWait/可重开快照 | Enter/Esc 不变；超时可迟到答；重连/刷新不丢待处理请求；fail-closed 优先 |
| G29 | P11/S09；5 | 凭据 record/OAuth grant：`libadwaita/credentials/records/*`（建议） | 覆盖 readRecord/list/modify/delete 与更新事件；不回读 secret；与 API key 内存注入分层 |

## 13. 阶段完成与验收用法

- **阶段 4**：主窗口、会话冷列表/创建/恢复/归档、可靠 transcript、Markdown/代码、基础提交/停止、审批/提问、模型切换、连接恢复。每行需真实 bridge 集成测试；含 G 标号的对应部分只能标“缺失”，不能标“已实现”。
- **阶段 5**：首启/凭据/设置/插件。插件启停必须走 restart-required 冷重启并恢复所有打开 Session；账号、市场、安装等缺口单独报告。
- **阶段 6**：按 §9 顺序完成卡片，再完成右栏、子代理、Trajectory、jobs/schedule 等。每卡用同一 prompt 对照上游截图/交互与原始事件。
- **最终验收**：沿用 N/M/C/A/S/P/O 编号逐项标记 `已实现 / 缺失 / 有意不移植 / 有意简化`。只有 N20、O02–O05、O10、O11、O14 等已写明理由的项目可标“有意不移植”；M06 数学排版只能标“有意简化”，其余功能不得静默删除。
