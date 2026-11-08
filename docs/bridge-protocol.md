# dsh-libadwaita 桥接协议规范（Bridge Protocol v1）

- 协议版本：`1`
- 锁定上游版本：`@deepseek-ai/dsh` `0.2.2-rc.1`
- 状态：定稿规范（阶段 2 权威实施依据）
- 文档定位：Rust 客户端（`app/`）与 TypeScript 桥接服务（`bridge/`）之间的通信协议唯一契约。Rust 开发者应仅凭本文档即可完整实现 `BridgeConnection` 与上层领域模型，无需查阅 TypeScript 源码。

---

## 1. 概述与核心原则

### 1.1 命名空间选型与设计理由

本项目所有 Client-to-Server 请求方法统一置于 `libadwaita/*` 命名空间下（如 `libadwaita/session/create`）。
设计理由如下：
1. **彻底杜绝方法名冲突**：上游 `@deepseek-ai/dsh-sdk-protocol` 占用了 `initialize`、`session/prompt`、`session/query` 等裸方法名；Web Controller 占用了 `session/create`、`llm/listProviders` 等。使用 `libadwaita/*` 可确保 bridge 作为独立 Cordis 插件 bundle 挂载时，绝不与上游内部方法命名冲突。
2. **明确客户端专属性**：该协议专为 Linux GTK4/libadwaita 原生桌面客户端剪裁，暴露桌面 GUI 必需的完整能力集（增量流式、历史分页、缺口修复、审批与提问交互、模型切换、设置与插件管理、内存凭据注入），不携带 Web 浏览器的无用私有端点。
3. **双向请求与事件沿用 SDK 经典词汇**：Server-to-Client 双向请求保持 `approval/request` 与 `user-question/ask`，通知保持 `session.event`、`agent.stream`、`session.status`、`session.title`、`user-question.cancelled`，最大程度兼容 SDK 既有类型定义与序列化心智。

### 1.2 进程与网络边界

- **单一连接**：GUI 进程（Rust 单二进制）为唯一客户端；`dsh --profile libadwaita` 进程（Node.js 运行时）内的 bridge 为唯一服务端。
- **传输介质**：Unix Domain Socket，文件权限强制为 `0600`（仅当前用户可读写），位于 `$XDG_RUNTIME_DIR/dsh-libadwaita/dsh-<pid>-<random>.sock`。禁止监听任何 TCP 端口。
- **帧协议**：NDJSON（Newline-Delimited JSON-RPC 2.0），以 `\n`（ASCII 0x0A）为帧终止符。每一行为一个完整的 JSON 对象，UTF-8 编码，禁止多字节分割或行内未转义换行。底层复用 `@deepseek-ai/dsh-sdk-protocol` 的 `JsonRpcLineTransport` 语义。
- **纯净标准输出**：dsh 进程的 `stdout` 严格保持 0 字节输出，绝不允许任何控制台日志、调试信息污染标准输出；全部运行时诊断、Cordis 框架输出与警告一律经由 `stderr` 写入。

### 1.3 架构基线与服务依赖边界（权威事实）

根据上游 `@deepseek-ai/dsh@0.2.2-rc.1` 源码结构及 `docs/dsh-web-feature-inventory.md`，必须明确以下底层事实：
1. **Base bundle 与 Web-app bundle 隔离**：Web 控制器（`session-controller`、`workspace-controller`、`settings-controller`、`terminal-controller`、`job-controller`）隶属于 `packages/bundle/web-app`。本项目 profile 配置为 `["@deepseek-ai/dsh-base", <bridge bundle>]`，因此 bridge **绝不能**假设存在 Web 控制器，必须直接在 base 提供的基础服务上实现：
   - 会话与代理：`ctx.agents`（`@deepseek-ai/dsh-agent`）、`ctx.sessions`（`@deepseek-ai/dsh-session`）
   - 模型管理：`ctx.llm`（`@deepseek-ai/dsh-llm`）
   - 交互仲裁：`ctx.approval`（`@deepseek-ai/dsh-user-approval`）、`ctx.userQuestions`（`@deepseek-ai/dsh-user-questions`）
   - 会话标题：`ctx.sessionTitle`（`@deepseek-ai/dsh-session-title`）
   - 工作目录：`ctx.workingDirectory`（`@deepseek-ai/dsh-working-directory`）
   - 插件与设置：`ctx.settings`（`@deepseek-ai/dsh-settings`）、`ctx.pluginManager`（`@deepseek-ai/dsh-plugin-manager`）
   - 凭据提供缝隙：`ctx.credentials`（`@deepseek-ai/dsh-credentials`）
2. **`profileContext` 受管环境前置约束**：`dsh-base` 的 patch 对 `plugin-manager` 与 `hmr` 声明了门禁 `disabled: !!js "!ctx.get('profileContext')"`。GUI 首次运行时在 `$XDG_DATA_HOME/dsh-libadwaita/profiles/libadwaita/` 下生成标准受管 profile，并通过 `dsh --profile libadwaita` 启动，以确保 `profileContext` 就绪。
3. **差异比对服务风险项**：`@deepseek-ai/dsh-workspace-changes`（生成工作区 git diff hunks 与文件变动摘要）默认在 `web-app` 层挂载。为了在 GTK4 原生界面呈现改动卡与 diff，bridge bundle patch 需显式 `insert: - id: workspace-changes, name: '@deepseek-ai/dsh-workspace-changes'` 挂载该行，或由 bridge 内部提供轻量实现。

---

## 2. 基础通信规范

### 2.1 JSON-RPC 2.0 格式基线

所有在 Unix Socket 上流动的消息均必须遵循 JSON-RPC 2.0 规范，包含单行 `\n`。

#### Client -> Server 请求帧
```json
{"jsonrpc":"2.0","id":"req-1","method":"libadwaita/session/prompt","params":{}}
```
- `id`：必须为字符串或安全非负整数。客户端保证同一连接上的在途请求 id 唯一。

#### Server -> Client 响应帧（成功）
```json
{"jsonrpc":"2.0","id":"req-1","result":{}}
```

#### Server -> Client 响应帧（失败）
```json
{"jsonrpc":"2.0","id":"req-1","error":{"code":-32002,"message":"session not found","data":{"sessionId":"sess-abc"}}}
```

#### Server -> Client 通知帧
```json
{"jsonrpc":"2.0","method":"session.event","params":{}}
```
- 通知帧绝不包含 `id` 字段，客户端无需回复。

#### Server -> Client 双向请求帧
```json
{"jsonrpc":"2.0","id":"srv-req-1","method":"approval/request","params":{}}
```
- 服务端作为发起方提供 `id`，客户端处理后必须回复同 `id` 的响应帧。

---

## 3. 连接握手与生命周期

### 3.1 握手与版本协商：`libadwaita/initialize`

GUI 连接 Socket 后的第一个请求必须为 `libadwaita/initialize`。在握手成功前，服务端拒绝处理除 `libadwaita/shutdown` 外的任何业务请求。

#### 请求参数（`InitializeParams`）
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `clientVersion` | `string` | 是 | GUI 客户端版本号（如 `"0.1.0"`） |
| `protocolVersion` | `number` | 是 | 客户端期望的协议主版本号，当前必须为 `1` |
| `capabilities` | `ClientCapabilities` | 否 | 客户端声明支持的特性集 |

`ClientCapabilities` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `userQuestions` | `boolean` | 否 | 是否启用双向 `user-question/ask` 提问，默认 `true` |
| `approvalRequests` | `boolean` | 否 | 是否启用双向 `approval/request` 审批，默认 `true` |
| `streaming` | `boolean` | 否 | 是否接收 `agent.stream` 增量流通知，默认 `true` |

#### 成功结果（`InitializeResult`）
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `protocolVersion` | `number` | 是 | 服务端实际运行的协议版本，当前为 `1` |
| `dshVersion` | `string` | 是 | 运行中的 `@deepseek-ai/dsh` 核心版本（如 `"0.2.2-rc.1"`） |
| `profileName` | `string` | 是 | 当前加载的 profile 名称（`"libadwaita"`） |
| `generationId` | `string` | 是 | 服务端实例唯一代际标识符（每次进程冷启动刷新），用于检测断线后进程是否重启 |
| `serverInfo` | `object` | 是 | 服务端元数据：`{ "name": "dsh-libadwaita-bridge", "version": "0.1.0" }` |

#### 错误处理
- 若 `protocolVersion !== 1`，服务端返回错误码 `-32001`（`PROTOCOL_VERSION_MISMATCH`），并在 `data` 中返回服务端支持的范围。GUI 收到该错误后必须阻止进入主界面，向用户显示版本升级引导。

### 3.2 进程关闭：`libadwaita/shutdown`

#### 请求参数
无参数（或 `{}`）。

#### 成功结果
返回 `{ "ok": true }`。服务端收到后开始可逆卸载：关闭 socket 监听、注销 Cordis 事件监听、等待在途持久化写入完成并退出。

---

## 4. 可靠流式与缺口修复（Reliable Streaming & Rebaseline）

上游 Web 客户端的真实实践表明：**可靠流式不能只靠无脑广播两条事件流**。必须具备开场快照、连续序号、增量组装、缺口检测与代际重连恢复机制。

### 4.1 序号单调连续性不变量
1. `session.event` 携带的 `seq` 必须是单调连续自增的整数（从 `0` 或历史末尾开始）。
2. `agent.stream` 携带的 `frame.revision` 在同一轮 Agent 生命周期内单调递增，`frame.index` 在同一 attempt 内部必须是从 `0` 开始的密集整数。
3. 若 GUI 观察到到达的 `session.event.seq !== lastReceivedSeq + 1`，即意味着网络或内部缓冲区发生丢包/断档（Gap）。GUI 不得将跳号事件直接插入视图，必须暂停界面流式渲染，发起 `libadwaita/session/page` 填补缺口；若无法恢复则重新执行 `libadwaita/session/follow` 重建基线。

### 4.2 会话追踪与基线获取：`libadwaita/session/follow`

当用户在侧栏选择某会话或打开窗口时调用。服务端返回该会话当前的完整开场快照及运行中状态，并将其加入实时事件转发名单。

#### 请求参数（`SessionFollowParams`）
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `sessionId` | `string` | 是 | 目标会话标识符 |
| `fromSeq` | `number` | 否 | 客户端当前已持有的最新事件 `seq`。若提供且在服务端缓冲区内，可跳过历史直接跟随 |

#### 成功结果（`SessionFollowResult`）
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `header` | `SessionHeader` | 是 | 会话元数据（创建时间、工作目录、版本等） |
| `latestSeq` | `number` | 是 | 当前已落盘的最新事件序号（无事件时为 `-1`） |
| `status` | `'idle' \| 'running'` | 是 | 当前会话运行状态 |
| `title` | `SessionTitleSnapshot` | 否 | 会话当前标题快照（包含来源：user / provider / fallback） |
| `projections` | `SessionProjectionsSnapshot` | 是 | 聚合投影当前状态（plan, todos 等） |
| `assistantBaseline` | `SessionAssistantBaseline` | 否 | 若当前正处于 `running` 状态，返回正在生成的未定稿 assistant 增量片段；空闲时为 `null` |

`SessionProjectionsSnapshot` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `planMode` | `boolean` | 是 | 当前是否处于 plan 模式 |
| `todos` | `TodoItem[] \| null` | 是 | 当前最新 todo 列表快照，未曾写入过时为 `null` |

`SessionAssistantBaseline` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `attemptId` | `string` | 是 | 当前未落盘的 attempt 标识符 |
| `turn` | `number` | 是 | 当前轮次编号 |
| `step` | `number` | 是 | 当前 step 编号 |
| `revision` | `number` | 是 | 当前流式版本号 |
| `chunks` | `StreamChunk[]` | 是 | 已经发送出的全部增量历史块序列 |

### 4.3 缺口修复与历史分页：`libadwaita/session/page`

用于两处场景：
1. **缺口修复（Gap Repair）**：实时通信丢帧时，拉取指定区间的持久化事件。
2. **历史加载（Load Older）**：消息视图向上滚动，按需拉取更早的历史。

#### 请求参数（`SessionPageParams`）
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `sessionId` | `string` | 是 | 会话标识符 |
| `fromSeq` | `number` | 是 | 起始 sequence number（包含） |
| `toSeq` | `number` | 否 | 结束 sequence number（包含）。缺省时按 `limit` 决定 |
| `limit` | `number` | 否 | 最大返回事件条数，默认 `50`，上限 `200` |

#### 成功结果（`SessionPageResult`）
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `events` | `SessionEvent[]` | 是 | 按 `seq` 升序排列的连续事件列表 |
| `hasMore` | `boolean` | 是 | 往前或往后是否还有更多未拉取事件 |

### 4.4 断线重连与代际恢复机制
- **同代际重连（`generationId` 未变）**：dsh 进程未挂，仅 socket 临时中断。客户端重连后，读取本地持有的 `lastSeq`，调用 `libadwaita/session/page` 补齐离线期间的差额事件。
- **跨代际重连（`generationId` 改变）**：说明 dsh 进程曾崩溃或被重启。此时未提交的内存 stream 已全部失效，所有旧在途请求作废。客户端必须对当前打开的会话重新调用 `libadwaita/session/follow`，获取最新 `latestSeq` 和落盘状态，全量刷新界面。

---

## 5. 完整 Client-to-Server 方法表

| 方法名称 | 用途 | 对应上游服务基准 |
|---|---|---|
| `libadwaita/initialize` | 协议握手与版本协商 | bridge 内部 |
| `libadwaita/shutdown` | 正常请求退出 dsh 进程 | bridge 内部 |
| `libadwaita/session/create` | 创建新会话 | `ctx.agents.create`（自动创建 session） |
| `libadwaita/session/list` | 列出活跃与会话持久化中的历史会话 | `ctx.sessionQuery.listSessions` + `ctx.sessions.list` |
| `libadwaita/session/follow` | 追踪会话并获取开场快照 | `ctx.sessions` + `ctx.agents` |
| `libadwaita/session/page` | 分页拉取历史事件 / 缺口修复 | `ctx.sessions.get(id).slice` |
| `libadwaita/session/prompt` | 向会话投递新的用户提示词 | `agent.followup` / `agent.steer` |
| `libadwaita/session/queue/list` | 读取当前 inbox 队列快照 | `agent.inbox.nextTurn/nextStep` |
| `libadwaita/session/queue/edit` | 就地替换待发送队列项的文本 | `agent.inbox.replace` |
| `libadwaita/session/queue/remove` | 移除一个待发送队列项 | `agent.inbox.remove` |
| `libadwaita/session/queue/steer` | 将排队项转为当前 turn 的 steer | `agent.inbox.remove` + `agent.steer` |
| `libadwaita/session/cancel` | 停止当前正在运行的 turn | `agent.cancel({ kind: 'user' })` |
| `libadwaita/session/release` | GUI 本地关闭视图，释放追踪 | bridge 内存释放 |
| `libadwaita/session/archive` | 归档会话（清理队列，写入持久归档集） | `workspaceRegistry.archiveSession` + `agent.cancel` |
| `libadwaita/session/rename` | 手动重命名会话并 pin 用户标题 | `ctx.sessionTitle.rename(session, title)` |
| `libadwaita/session/fork` | 从指定事件边界分支新会话 | `buildForkSeed` + `ctx.agents.create` |
| `libadwaita/session/workdir/get` | 读取会话当前生效的工作目录 | `ctx.workingDirectory.get` |
| `libadwaita/session/model/get` | 读取会话当前使用的模型路由 | `agent.model` / `request/context` |
| `libadwaita/session/model/set` | 切换会话模型路由配置 | `agent.model.select` |
| `libadwaita/model/catalog` | 查询可用模型与 Provider 目录 | `ctx.llm.catalog` / Provider 列表 |
| `libadwaita/settings/get` | 读取全局设置项配置表 | `ctx.settings.get` |
| `libadwaita/settings/mutate` | 原子修改配置表项（支持 expectedRevision） | `ctx.settings.update` |
| `libadwaita/settings/describe` | 读取设置命名空间 schema/值/revision（脱敏） | `ctx.settings.describe({redactSecrets:true})` |
| `libadwaita/workspace/list` | 列出持久化工作区 | `ctx.workspaceRegistry.list` |
| `libadwaita/workspace/create` | 按 cwd 幂等创建工作区 | `ctx.workspaceRegistry.resolveByPath/create` |
| `libadwaita/workspace/rename` | 重命名工作区显示标题 | `Workspace.setTitle` |
| `libadwaita/workspace/remove` | 移除工作区记录（不动会话） | `ctx.workspaceRegistry.delete` |
| `libadwaita/permission/catalog` | 读取权限预设目录 | `ctx.permissionPresets.catalog()` |
| `libadwaita/permission/get` | 读取会话当前权限预设 | `ctx.permissionPresets.current(session)` |
| `libadwaita/permission/set` | 切换会话权限预设 | `ctx.permissionPresets.set(session, preset)` |
| `libadwaita/goal/get` | 读取会话当前 goal | `ctx.goals.get(agent)` |
| `libadwaita/goal/edit` | 编辑 goal 目标/轮次上限 | `ctx.goals.edit` |
| `libadwaita/goal/pause` | 暂停 goal | `ctx.goals.pause` |
| `libadwaita/goal/resume` | 恢复 goal | `ctx.goals.resume` |
| `libadwaita/goal/clear` | 清除 goal | `ctx.goals.clear` |
| `libadwaita/files/list` | 列出会话 cwd 内目录项 | bridge 本地安全文件访问 |
| `libadwaita/files/stat` | 查询文件元数据 | bridge 本地安全文件访问 |
| `libadwaita/files/read` | 读取会话 cwd 内文本文件 | bridge 本地安全文件访问 |
| `libadwaita/files/readBytes` | 读取会话 cwd 内字节文件 | bridge 本地安全文件访问 |
| `libadwaita/session/projections` | 读取会话可用投影基线 | `ctx.sessionProjections.snapshot` + `ctx.goals.get` |
| `libadwaita/session/search` | 元数据搜索会话标题与 cwd | bridge 侧元数据过滤（非全文） |
| `libadwaita/tool-result/watch` | 订阅/取消会话工具结果通知 | bridge 侧派生自 `session.event` |
| `libadwaita/message-feedback/list` | 列出消息反馈 | `ctx.messageFeedback.list` |
| `libadwaita/message-feedback/put` | 写入消息反馈（CAS） | `ctx.messageFeedback.put` |
| `libadwaita/message-feedback/delete` | 删除消息反馈 | `ctx.messageFeedback.delete` |
| `libadwaita/session-feedback/record` | 记录会话级反馈 | `ctx.sessionFeedback.record` |
| `libadwaita/question/reopen` | 取回仍在等待中的用户提问快照 | bridge 内存快照 |
| `libadwaita/job/list` | 列出会话后台 jobs | `ctx.jobs.list(sessionId)` |
| `libadwaita/job/follow` | 读取并跟随 job 输出 | `ctx.jobs.get/readAt` |
| `libadwaita/job/kill` | 请求停止 job | `ctx.jobs.kill` |
| `libadwaita/attachment/upload` | 上传会话附件（base64，限 16 MiB） | bridge 本地会话附件存储 |
| `libadwaita/attachment/read` | 读取会话附件 | bridge 本地会话附件存储 |
| `libadwaita/schedule/catalog` | 列出计划任务 | `ctx.schedule.catalog` |
| `libadwaita/schedule/create` | 创建计划任务 | `ctx.schedule.create` |
| `libadwaita/schedule/edit` | 编辑计划任务 | `ctx.schedule.edit` |
| `libadwaita/schedule/run` | 立即运行任务 | `ctx.schedule.run` |
| `libadwaita/schedule/history` | 读取投递历史 | `ctx.schedule.history` |
| `libadwaita/schedule/delete` | 删除任务 | `ctx.schedule.edit({action:'delete'})` |
| `libadwaita/schedule/delete-inactive` | 删除全部已结束任务 | `ctx.schedule.deleteInactive` |
| `libadwaita/schedule/delivery-limits` | 读取新会话投递最小间隔 | `ctx.schedule.deliveryLimits` |
| `libadwaita/terminal/create` | 创建会话 PTY | `ctx.terminals.spawn` |
| `libadwaita/terminal/list` | 列出会话 PTY | `ctx.terminals.list` |
| `libadwaita/terminal/input` | 写入 PTY 并等待就绪 | `ctx.terminals.startSend` |
| `libadwaita/terminal/output` | 读取 PTY 回滚缓冲 | `ctx.terminals.read` |
| `libadwaita/terminal/close` | 关闭 PTY | `ctx.terminals.kill` |
| `libadwaita/plugin/list` | 查询已安装与已启用的插件 | `ctx.pluginManager.list` |
| `libadwaita/plugin/enable` | 启用插件行（返回上游 ChangeResult） | `ctx.pluginManager.setPluginEnabled` |
| `libadwaita/plugin/disable` | 禁用插件行（返回上游 ChangeResult） | `ctx.pluginManager.setPluginEnabled` |
| `libadwaita/plugin/bundle/enable` | 启用 Bundle（返回上游 ChangeResult） | `ctx.pluginManager.setBundleEnabled` |
| `libadwaita/plugin/bundle/disable` | 禁用 Bundle（返回上游 ChangeResult） | `ctx.pluginManager.setBundleEnabled` |
| `libadwaita/plugin/registry-probe` | 探测最快 npm registry | bridge 本地 HTTPS ping（npmjs/npmmirror 竞速） |
| `libadwaita/plugin-market/list-indexes` | 列出插件市场索引源 | `ctx.pluginMarket.listIndexes` |
| `libadwaita/plugin-market/set-indexes` | 设置插件市场索引源 | `ctx.pluginMarket.setIndexes` |
| `libadwaita/plugin-market/search` | 搜索插件市场索引 | `ctx.pluginMarket.search` |
| `libadwaita/plugin-market/read-entry-detail` | 读取市场条目详情文档 | `ctx.pluginMarket.readEntryDetail` |
| `libadwaita/credentials/status` | 查询脱敏凭据状态（无明文） | `ctx.credentials.describe` |
| `libadwaita/credentials/inject` | 向内存 Provider 注入新凭据 | 自定义内存 Provider（Spike 已验证） |
| `libadwaita/credentials/rotate` | 轮换内存中现有凭据 | 自定义内存 Provider（Spike 已验证） |
| `libadwaita/credentials/remove` | 从内存中移除指定 Provider 凭据 | 自定义内存 Provider（Spike 已验证） |
| `libadwaita/workspace/changes` | 查询工作区变更文件摘要与 diff 块 | `@deepseek-ai/dsh-workspace-changes` |
| `libadwaita/command/list` | 列出当前会话可用的 slash 命令 | `ctx.commands.list(agent)` |
| `libadwaita/command/execute` | 执行 slash 命令（如 `/plan off`） | `ctx.commands.execute` |
| `libadwaita/skill/list` | 列出当前工作区可用的 skill 目录 | `ctx.skills.snapshot` |
| `libadwaita/reference/list` | 列出 `@` 文件引用候选 | `ctx.fileReferences.list` |
| `libadwaita/plugin/detail` | 读取 Bundle 详情文档 | `ctx.pluginManager.readBundleDetail` |
| `libadwaita/plugin/inspect` | 预检安装规格（本地路径/包/git） | `ctx.pluginManager.inspect` |
| `libadwaita/plugin/install` | 安装 Bundle | `ctx.pluginManager.installBundle` |
| `libadwaita/plugin/cancel-install` | 取消安装 | `ctx.pluginManager.cancelInstall` |
| `libadwaita/plugin/wait-install` | 等待安装结果 | `ctx.pluginManager.waitForInstall` |
| `libadwaita/plugin/remove` | 卸载 Bundle | `ctx.pluginManager.removeBundle` |
| `libadwaita/plugin/versions` | 列出版本 | `ctx.pluginManager.listVersions` |
| `libadwaita/plugin/update` | 更新 Bundle | `ctx.pluginManager.updateBundle` |
| `libadwaita/plugin/registries` | 读取 registry 列表 | `ctx.pluginManager.registries` |
| `libadwaita/plugin/inventory` | 插件 + Bundle 清单 | `listPlugins` + `listBundles` |
| `libadwaita/plugin/retry` | 重读插件清单 | 同 inventory |
| `libadwaita/model/discover` | 用未保存配置发现模型 | `ctx.llm.discoverModels` |
| `libadwaita/web-search/providers` | 列出 search/fetch provider | `ctx.web.listProviders` |
| `libadwaita/subagent/list` | 递归列出子代理目录 | `ctx.subagents.listDescendants` |
| `libadwaita/subagent/prompt` | 向子代理投递续接消息 | `ctx.subagents.prompt` |
| `libadwaita/subagent/interrupt` | 用户中断子代理 | `ctx.subagents.interrupt/interruptByUser` |
| `libadwaita/agent-team/get` | 读取 Team 开关与任务 | `ctx.agentTeams.isEnabled/listTasks` |
| `libadwaita/agent-team/set-enabled` | 设置 Team 开关 | `ctx.agentTeams.setEnabled` |
| `libadwaita/credentials/records/list` | 列出凭据记录元数据 | `CredentialProvider.listRecords` |
| `libadwaita/credentials/records/read` | 读取脱敏凭据记录 | `describeRecord` + `readRecord`（脱敏） |
| `libadwaita/credentials/records/modify` | 写入凭据记录 | `CredentialProvider.modifyRecord` |
| `libadwaita/credentials/records/delete` | 删除凭据记录 | `CredentialProvider.deleteRecord` |
| `libadwaita/account/status` | 读取账号状态 | `ctx.deepseekAccount.getState` |
| `libadwaita/account/profile` | 读取账号资料 | `ctx.deepseekAccount.getProfile` |
| `libadwaita/account/balance` | 读取账号余额 | `ctx.deepseekAccount.getBalance` |
| `libadwaita/account/sign-out` | 退出账号 | `ctx.deepseekAccount.signOut` |

---

## 6. 方法详细定义与载荷契约

### 6.1 会话创建与管理

#### `libadwaita/session/create`
- **说明**：新建一个会话并初始化 Agent；`sessionId` 命名已存在会话时复用该会话（返回其现有 header/agent，必要时 resume），仅对不存在的 id 调用 `ctx.agents.create`。
- **重要实现细节**：根据阶段 1 Spike 结论，调用 `ctx.agents.create({ sessionId, cwd })` 内部会自动创建底层 session；session 存储对重复 id 报 `session already exists`，因此命名已存在的 id 时必须走复用路径，不得重复 create（与上游 Host 复用命名 blank 会话的行为一致）。创建或复用成功后，bridge 通过 `workspaceRegistry.resolveByPath(cwd)` 找到匹配工作区并 `attachSession(sessionId)`，使侧边栏按工作区分组且成员关系跨重启保留；无匹配工作区时保持“未分组”。
- **请求参数**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 否 | 客户端指定的 UUID。缺省由 bridge 自动生成并返回；命名已存在会话时复用该会话并返回同一 ID |
  | `cwd` | `string` | 是 | 关联的项目根绝对目录路径 |
  | `provider` | `string` | 否 | 指定 LLM provider（缺省使用系统默认配置） |
  | `model` | `string` | 否 | 指定模型名称（缺省使用系统默认配置） |
  | `agentPreset` | `string` | 否 | 代理预设（如 `"developer"`） |
- **响应结果**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 确定的会话唯一 ID |
  | `header` | `SessionHeader` | 是 | 写入的不可变头部信息 |

#### `libadwaita/session/list`
- **说明**：合并列出活跃会话与本地磁盘持久化的历史会话（冷列表，不拉起 Agent 实例）。冷条目的标题与轮次取自 `sessionProjectionCache`，无缓存时回退到 cwd 标题与 `turnCount: 0`；结果先整体排序再按 `limit` 截断。
- **归档语义**：归档集合来自 `ctx.workspaceRegistry.archivedSessionIds`（持久化，跨 bridge/GUI 重启保留），`archived: true` 表示“包含已归档会话”而非“仅已归档”；客户端按 `isArchived` 再过滤。
- **请求参数**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `archived` | `boolean` | 否 | 是否包含已归档会话，默认 `false` |
  | `limit` | `number` | 否 | 返回条数上限，默认 `100` |
- **响应结果**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessions` | `SessionListItem[]` | 是 | 会话摘要列表，按创建或最近活动时间降序 |

`SessionListItem` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `id` | `string` | 是 | 会话标识符 |
| `createdAt` | `number` | 是 | 毫秒时间戳 |
| `updatedAt` | `number` | 是 | 最近一条事件时间戳 |
| `cwd` | `string` | 是 | 项目目录 |
| `title` | `string` | 是 | 当前标题 |
| `turnCount` | `number` | 是 | 轮次数量 |
| `isArchived` | `boolean` | 是 | 是否已归档 |

#### `libadwaita/session/cancel`
- **说明**：停止当前正在执行的 turn。对应 Web UI 的停止按钮与双 Esc 快捷键。
- **语义约束**：调用 `agent.cancel({ kind: 'user' }, { keepInbox: true })`。这会使正在执行的 LLM 生成或工具调用中断，关闭当前 turn，并将已接收但尚未执行的 pending 消息保留在 inbox 中。不会销毁会话。
- **请求参数**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 目标会话标识符 |
  | `reason` | `string` | 否 | 可选取消原因说明（用于审计日志） |
- **响应结果**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `cancelled` | `boolean` | 是 | 若当前恰有运行中的 turn 被取消返回 `true`；若当时已处于空闲返回 `false` |

#### `libadwaita/session/release`（关闭视图与内存解绑）
- **说明**：明确上游无 `session/close` RPC。关闭视图是 GUI 侧本地行为。此 RPC 仅用于告知 bridge：当前 GUI 客户端已不再查看该会话，bridge 可注销对该会话的 live 监听并释放相关的内存缓存句柄。
- **请求参数**：`{ "sessionId": string }`
- **响应结果**：`{ "ok": true }`

#### `libadwaita/session/archive`
- **说明**：归档指定会话。通过 `workspaceRegistry.archiveSession(id, { stopActivity: true })` 写入持久归档集（跨重启保留），随后安全停止运行中的 turn 并清空 pending inbox。取消归档走 `unarchiveSession`，保留原工作区排序槽位。磁盘日志保留，不会物理删除。
- **请求参数**：`{ "sessionId": string, "archived": boolean }`
- **响应结果**：`{ "sessionId": string, "archived": boolean }`

#### `libadwaita/session/rename`

- **说明**：手动重命名会话。调用上游 `ctx.sessionTitle.rename(session, title)`，追加 `session/title` 且 `source.kind = 'user'`，从而 pin 用户标题并停止后续自动标题生成。成功后同时发出 `session.title` 通知。
- **上游实现事实**：`SessionTitleService.rename` 接收 live `Session` 对象，而非 sessionId；bridge 负责先解析（必要时 resume）会话。标题为空或全空白时抛出 `SessionTitleInvalidError`。
- **请求参数（`SessionRenameParams`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 目标会话标识符 |
  | `title` | `string` | 是 | 用户输入的原始标题；服务端会规范化并截断到 `maxTitleBytes` |
- **响应结果（`SessionRenameResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 会话标识符 |
  | `title` | `string` | 是 | 规范化后实际写入的标题 |
  | `source` | `object` | 是 | 标题来源，重命名后固定为 `{ "kind": "user" }` |
- **错误**：未知会话 `-32002 SESSION_NOT_FOUND`；空标题 `-32012 TITLE_INVALID`。

#### `libadwaita/session/fork`

- **说明**：从源会话的指定事件边界分支出一个全新会话。使用上游 `buildForkSeed(events, boundary)` 复制闭区间前缀并追加 `session/end-seed` 与未闭合 turn/step 的 `forked` 收尾事件，再调用 `ctx.agents.create` 创建带 seed 的 live 子会话。子会话拥有独立日志与 Agent，不与源会话共享后续写入。
- **边界语义**：`atSeq` 为包含式事件序号；缺省时取最近一个 `turn/end` 之后、下一 `turn/start`/用户消息/inbox 变更之前的稳定前缀。无效边界返回 `-32019 FORK_UNAVAILABLE`。
- **标题递增**：`increaseTitle` 仅由 bridge 消费。若源会话存在持久化标题，子会话在创建后被重命名为尾部 `(N)` / `（N）` 递增、并跳过已占用标题的形式；源会话无标题时跳过。
- **请求参数（`SessionForkParams`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 源会话标识符 |
  | `atSeq` | `number` | 否 | 包含式源事件序号；缺省取最近完成 turn 的稳定前缀 |
  | `increaseTitle` | `boolean` | 否 | 是否递增源标题作为子会话标题，默认 `false` |
- **响应结果（`SessionForkResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 新建子会话标识符 |
  | `header` | `SessionHeader` | 是 | 子会话头部；`isSeeded` 为 `true`，`cwd` 继承源会话 |
- **使用流程**：成功后 GUI 立即调用 `libadwaita/session/follow` 获取子会话开场快照并切换主视图。

### 6.2 工作目录（CWD）与工作区切换语义

#### `libadwaita/session/workdir/get`
- **说明**：读取当前会话最新生效的工作目录。
- **请求参数**：`{ "sessionId": string }`
- **响应结果**：`{ "cwd": string }`

#### 工作区切换核心设计结论（关于 Drop `workdir/set`）
- **权威事实确认**：经严密查验上游源码，dsh Web UI 中切换工作区**绝非**调用某个全局 API 就地更改现有活跃会话的 cwd。在上游体系中，切换 Workspace 必定是：**在目标工作区目录中打开或新建一个会话**，并将之前的草稿输入（draft）迁移过来。
- **协议裁决**：**不在协议方法表中暴露 `session/workdir/set` 方法**。切换工作区这一业务行为完全映射为：GUI 调用 `libadwaita/session/create` 传入新的 `cwd`，并关闭前一会话视图。这严格遵循上游 Web UI 的架构逻辑，规避让 Agent 在运行期产生不可预期的路径错乱风险。

### 6.3 输入提交与队列控制：`libadwaita/session/prompt`

- **说明**：向指定会话提交人类消息、附件或命令输入。
- **请求参数（`SessionPromptParams`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 会话标识符 |
  | `contentBlocks` | `SdkPromptContentBlock[]` | 是 | 提交的输入块序列 |
  | `mode` | `'queue' \| 'steer' \| 'prompt'` | 否 | 投递模式：`queue`（旧名 `prompt`）追加为下一 turn；`steer` 插入活跃 turn 的下一步。默认 `queue` |

`SdkPromptContentBlock` 联合类型定义（引用自 `@deepseek-ai/dsh-sdk-protocol/types`）：
1. 文本块：`{ "type": "text", "text": string }`
2. 图片块：
   ```json
   {
     "type": "image",
     "data": "<base64 encoded raster bytes>",
     "mimeType": "image/png" | "image/jpeg" | "image/webp" | "image/gif",
     "name": "screenshot.png"
   }
   ```
- **响应结果（`SessionPromptResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `messageId` | `string` | 是 | 成功写入队列的消息 ID。GUI 据此将本地乐观草稿替换为正式气泡 |

#### 队列快照与变更

Bridge 直接投影上游 `agent.inbox`：`nextTurn` 为等待独立 turn 的排队项（`placement: 'queued'`），`nextStep` 为等待下一步的插话/上下文项（用户来源为 `steering`，其余为 `context`）。任何 `agent/inbox/spliced` 事件都会触发 `session.inbox` 通知，携带完整快照；GUI 以快照整体替换本地队列，不做增量拼接。

#### `libadwaita/session/queue/list`

- **请求参数**：`{ "sessionId": string }`
- **响应结果（`QueueListResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `items` | `QueueItem[]` | 是 | 按 `nextTurn` 后 `nextStep` 顺序排列的完整快照 |

`QueueItem` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `id` | `string` | 是 | 待发送消息的 `messageId`，与 prompt 响应一致 |
| `placement` | `'queued' \| 'steering' \| 'context'` | 是 | 队列位置分类 |
| `content` | `unknown[]` | 是 | 上游消息 content 数组（文本/图片等） |

#### `libadwaita/session/queue/edit`

- **说明**：就地替换一个仍待发送的队列项文本。与上游 `session/updateQueue` 一致：仅接受 `text` 块，且必须包含非空白文本；消息 id 保持不变。
- **请求参数（`QueueEditParams`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 会话标识符 |
  | `itemId` | `string` | 是 | 目标队列项的消息 id |
  | `content` | `TextContentBlock[]` | 是 | 替换后的纯文本内容 |
- **响应结果**：`{ "accepted": true }`
- **错误**：项已不在队列 `-32013 QUEUE_ITEM_NOT_FOUND`。

#### `libadwaita/session/queue/remove`

- **说明**：移除一个仍待发送的队列项。
- **请求参数**：`{ "sessionId": string, "itemId": string }`
- **响应结果**：`{ "accepted": true }`
- **错误**：项已不在队列 `-32013 QUEUE_ITEM_NOT_FOUND`。

#### `libadwaita/session/queue/steer`

- **说明**：把一个 `nextTurn` 排队项转为当前活跃 turn 的 steer。与上游一致：仅当 agent 处于 `running` 且项位于 `nextTurn` 时可用；bridge 先 `agent.inbox.remove` 再 `agent.steer(message)`。
- **请求参数**：`{ "sessionId": string, "itemId": string }`
- **响应结果**：`{ "accepted": true }`
- **错误**：当前 turn 不再接受插话或项不在 `nextTurn` 时返回 `-32014 QUEUE_STEER_UNAVAILABLE`。

### 6.4 模型管理与模型目录

#### `libadwaita/model/catalog`
- **说明**：获取当前系统支持并已配置好适配器的 Provider 与模型列表。
- **请求参数**：无参数。
- **响应结果（`ModelCatalogResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `defaultProvider` | `string` | 是 | 默认 Provider（如 `"deepseek"`） |
  | `defaultModel` | `string` | 是 | 默认模型名（如 `"deepseek-chat"`） |
  | `providers` | `ModelProviderInfo[]` | 是 | 可选的 Provider 列表 |

`ModelProviderInfo` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `id` | `string` | 是 | 提供商标识（如 `"deepseek"`） |
| `name` | `string` | 是 | 显示名称 |
| `configured` | `boolean` | 是 | 凭据是否已就绪 |
| `models` | `ModelDescriptor[]` | 是 | 该提供商下的模型项 |

`ModelDescriptor` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `id` | `string` | 是 | 模型名称（如 `"deepseek-coder"`） |
| `displayName` | `string` | 是 | 界面展示名称 |
| `contextWindow` | `number` | 否 | 上下文窗口 Token 数 |
| `supportsReasoning`| `boolean` | 是 | 是否支持推理过程（Reasoning） |

#### `libadwaita/session/model/get`
- **请求参数**：`{ "sessionId": string }`
- **响应结果**：`{ "provider": string, "model": string, "reasoningEffort"?: string }`

#### `libadwaita/session/model/set`
- **说明**：为当前会话切换后续 turn 所使用的模型。若会话正处于运行中的 step，当前 step 保持原模型，切换在下一步生效。
- **请求参数**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 会话标识符 |
  | `provider` | `string` | 是 | 目标 Provider |
  | `model` | `string` | 是 | 目标模型名称 |
  | `reasoningEffort` | `string` | 否 | 可选推理强度（如 `"high"`、`"medium"`、`"low"`） |
- **响应结果**：`{ "ok": true }`

### 6.5 全局设置读写

#### `libadwaita/settings/get`
- **说明**：获取当前 profile 下的设置值（永远脱敏）。
- **请求参数**：`{ "keys"?: string[] }`（可选过滤命名空间）
- **响应结果**：`{ "settings": Record<string, unknown> }`，键为 settings 命名空间。

#### `libadwaita/settings/describe`
- **说明**：返回设置命名空间的完整描述，用于动态表单。调用上游 `ctx.settings.describe({ redactSecrets: true })`；secret 字段已被上游标记并替换，协议层绝不返回明文。
- **请求参数**：`{ "keys"?: string[] }`
- **响应结果（`SettingsDescribeResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `namespaces` | `SettingsDescriptor[]` | 是 | 描述符列表 |

`SettingsDescriptor` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `ns` | `string` | 是 | 命名空间（对应 profile 行 id） |
| `autoGenerate` | `boolean` | 是 | 是否自动生成设置页 |
| `schema` | `unknown` | 是 | 上游表单 JSON Schema |
| `value` | `unknown` | 是 | 脱敏后的当前值 |
| `base` | `unknown` | 否 | 继承层值 |
| `user` | `unknown` | 否 | 用户层值 |
| `revision` | `number` | 是 | 当前修订号；`mutate` 冲突检测依据 |
| `secrets` | `unknown` | 否 | 上游 secret 标记位 |

#### `libadwaita/settings/mutate`
- **说明**：原子更新设置。支持两种形态：单命名空间 `namespace + patch`（`patch` 为该命名空间的字段合并表），或多命名空间 `patch` 映射表（键为命名空间，值为字段合并表）。`expectedRevision` 可选；提供时底层委托 `ctx.settings.update(ns, patch, expectedRevision)`，命名空间自上次 describe 以来发生变更则拒绝。
- **请求参数（`SettingsMutateParams`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `namespace` | `string` | 否 | 单命名空间写；提供时 `patch` 为该命名空间字段 |
  | `patch` | `Record<string, unknown>` | 是 | 字段合并表或命名空间映射 |
  | `expectedRevision` | `number` | 否 | CAS 期望修订号 |
- **响应结果**：`{ "settings": Record<string, unknown> }`（脱敏后的完整设置）
- **错误**：修订冲突返回 `-32020 SETTINGS_CONFLICT`，`data` 含 `expected` 与 `actual`。

### 6.6 插件管理（HMR 禁用场景与 `restart-required`）

- **重要前置约束**：在我们的 profile 中，配置明确禁用了 HMR（`- id: hmr, disabled: true`）。因此，所有动态启用或禁用插件/Bundle 的操作，**均无法在当前进程中热重载生效**。配置变更将持久化保存到 profile 配置中，上游 `ChangeResult.application` 报告为 `restart-required`（并带 `restartReason`）。
- **GUI 交互要求**：GUI 收到 `application: "restart-required"` 后，必须在界面弹出 AdwBanner 或通知：“插件配置已更改，需要重启 dsh 才能生效”，并提供“立即重启”动作；`application: "failed"` 时展示 `error.code` / `error.diagnostic`，不得伪造成成功。

#### `libadwaita/plugin/list`
- **请求参数**：无参数。
- **响应结果（`PluginListResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `plugins` | `PluginInfoItem[]` | 是 | 插件清单 |

`PluginInfoItem` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `id` | `string` | 是 | 插件行 ID（如 `"session-title-llm"`） |
| `name` | `string` | 是 | npm 包名或模块路径 |
| `enabled` | `boolean` | 是 | 当前是否处于启用状态 |
| `restartRequired` | `boolean` | 是 | 是否处于等待重启生效状态 |
| `description` | `string` | 否 | 插件描述文案 |

#### `libadwaita/plugin/enable` / `libadwaita/plugin/disable`
- **请求参数**：`{ "id": string }`（插件行 ID，`plugin/inventory` 的 `entryId`）
- **响应结果（`ChangeResult`，原样透传 `ctx.pluginManager.setPluginEnabled`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `changed` | `boolean` | 是 | profile 文件是否实际发生写入变化 |
  | `application` | `"applied" \| "restart-required" \| "overridden" \| "cancelled" \| "failed"` | 是 | 应用结果；HMR 禁用时通常为 `restart-required` |
  | `restartReason` | `string` | 否 | `restart-required` 的原因（`no-hmr` / `package-moved` / `host-module` / `unmanaged-plugin` / `module-comparison`） |
  | `stage` | `"install" \| "enable" \| "remove"` | 是 | 最后尝试的步骤 |
  | `target` | `string` | 是 | 插件行 ID |
  | `enabled` | `boolean` | 否 | 请求的目标启用状态 |
  | `error` | `{ code, diagnostic?, incompatible? }` | 否 | `application: "failed"` 时的上游 `ManagementError` |

  未知插件行等失败由上游返回 `application: "failed"` 的结果（不再吞掉）；服务不可用或调用直接抛错时返回 `-32038`。
  ```json
  {
    "changed": true,
    "application": "restart-required",
    "restartReason": "no-hmr",
    "stage": "enable",
    "target": "session-title-llm",
    "enabled": true
  }
  ```

#### `libadwaita/plugin/bundle/enable` / `libadwaita/plugin/bundle/disable`
- **请求参数**：`{ "name": string }`（Bundle 包名，`plugin/inventory` 的 `bundles[].name`）
- **响应结果**：与行级开关相同的上游 `ChangeResult`（`target` 为 Bundle 包名），原样透传 `ctx.pluginManager.setBundleEnabled`。

#### `libadwaita/plugin/registry-probe`
- **请求参数**：无参数。
- **响应结果**：`{ "result": "https://registry.npmmirror.com/" | "https://registry.npmjs.org/" | null }`
- **语义**：bridge 本地并发 GET `https://registry.npmjs.org/-/ping` 与 `https://registry.npmmirror.com/-/ping`（各 1.5s 超时），取最先成功响应者；两者均失败返回 `null`。结果缓存 5 分钟，并发调用共享同一次探测。用于安装对话框与 README 详情读取时的镜像推荐。

### 6.7 凭据注入与安全轮换（下游自有 Seam 规范）

- **安全边界**：
  1. 上游 `credentials/describe` **绝不向任何客户端返回明文 secret**，仅返回元数据（是否已配置、是否只读）。
  2. 我们通过 profile patch 将原磁盘明文凭据行 `credentials` 设为 `disabled: true`，并在 bridge 中挂载自写的内存凭据 Provider。
  3. API 凭据在 GUI 侧安全存储于系统级的 `libsecret` / Secret Service 中，并在 dsh 启动后经由 Unix Socket RPC 注入至 dsh 进程内存；绝不落盘，绝不写进 `.credentials.yaml`，绝不出现在命令行参数或环境变量中。

#### `libadwaita/credentials/status`
- **说明**：获取凭据就绪状态（元数据，无任何敏感信息）。
- **请求参数**：无参数。
- **响应结果**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `items` | `CredentialStatusItem[]` | 是 | 已知 Provider 凭据配置状态 |

`CredentialStatusItem` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `provider` | `string` | 是 | 提供商名称（如 `"DEEPSEEK_API_KEY"`） |
| `configured` | `boolean` | 是 | 内存中是否已持有有效 key |
| `masked` | `string` | 否 | 脱敏展示位（如 `"sk-...3w1o"`），未配置时为 `null` |

#### `libadwaita/credentials/inject`
- **说明**：GUI 启动或首启向导完成后，将 Secret Service 中的密钥注入 dsh 内存。
- **请求参数**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `provider` | `string` | 是 | 凭据名称（如 `"DEEPSEEK_API_KEY"`） |
  | `key` | `string` | 是 | 明文字符串（仅在 Unix Socket 0600 通道中单向流动） |
  | `baseURL` | `string` | 否 | 自定义 endpoint 地址（可选） |
- **响应结果**：`{ "provider": string, "configured": true }`

#### `libadwaita/credentials/rotate`
- **说明**：热轮换凭据，直接原子替换内存中的 key，无需重启 dsh 进程。
- **请求参数**：`{ "provider": string, "key": string }`
- **响应结果**：`{ "provider": string, "configured": true }`

#### `libadwaita/credentials/remove`
- **说明**：从内存 Provider 中清空指定凭据。
- **请求参数**：`{ "provider": string }`
- **响应结果**：`{ "provider": string, "configured": false }`

### 6.8 工作区差异比对：`libadwaita/workspace/changes`

- **说明**：查询某一轮 turn 引起的文件变动摘要及 diff hunks，用于在原生 UI 呈现 Git 改动卡。
- **前置风险项**：此服务依赖 `@deepseek-ai/dsh-workspace-changes` 插件。
- **请求参数**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 会话标识符 |
  | `turn` | `number` | 否 | 查询指定轮次，缺省时返回最新一轮的变更 |
  | `fileIndex` | `number` | 否 | 若指定，则额外计算并返回该文件的详细 diff hunks |
- **响应结果（`WorkspaceChangesResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `summary` | `WorkspaceChangesSummary` | 是 | 改动文件列表及行增删统计 |
  | `diff` | `WorkspaceFileDiff` | 否 | 当请求了 `fileIndex` 且计算可用时的比对详情 |


### 6.9 命令、技能与文件引用

#### `libadwaita/command/list`

- **说明**：列出指定会话当前生效的 slash 命令描述符。直接调用上游 `ctx.commands.list(agent)`，返回按名称排序的命令。
- **请求参数**：`{ "sessionId": string }`
- **响应结果（`CommandListResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `commands` | `CommandDescriptor[]` | 是 | 命令描述符 |

`CommandDescriptor` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `name` | `string` | 是 | 不带前导 `/` 的命令名（如 `plan`） |
| `description` | `string` | 是 | 人类可读摘要 |
| `input` | `object` | 否 | 可选自由输入描述：`{ hint: string, attachments?: boolean }` |
- **错误**：`commands` 服务未挂载时返回 `-32015 COMMANDS_UNAVAILABLE`。

#### `libadwaita/command/execute`

- **说明**：执行一条 slash 命令，不发送给模型。Bridge 拼接 `/<command> [args]` 后调用上游 `ctx.commands.execute(agent, line, [], signal)`；上游负责记录 `command/run` / `command/done` 事件。
- **请求参数（`CommandExecuteParams`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 会话标识符 |
  | `command` | `string` | 是 | 命令名，可带或不带前导 `/` |
  | `args` | `string` | 否 | 命令原始参数文本 |
- **响应结果（`CommandExecuteResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `commandId` | `string` | 否 | 本次执行的生命周期配对 id |
  | `kind` | `'success' \| 'error'` | 是 | 命令处理器归一化结果 |
  | `text` | `string` | 否 | 处理器返回的展示文本 |
  | `sourceEventSeq` | `number` | 否 | 更丰富展示所引用的权威事件序号 |
- **错误**：未知命令 `-32016 COMMAND_NOT_FOUND`；服务未挂载 `-32015 COMMANDS_UNAVAILABLE`。

#### `libadwaita/skill/list`

- **说明**：列出当前工作区可用的 skill 摘要。调用上游 `ctx.skills.snapshot({ cwd? })`；`sessionId` 用于从会话头部补全 `cwd`。
- **请求参数（`SkillListParams`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 否 | 提供则从该会话读取 cwd |
  | `cwd` | `string` | 否 | 显式工作目录，优先于会话 cwd |
- **响应结果（`SkillListResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `skills` | `SkillSummaryItem[]` | 是 | 合并去重后的 skill 摘要 |
  | `complete` | `boolean` | 是 | 所有 provider 是否在稳定目录版本内完成发现 |

`SkillSummaryItem` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `name` | `string` | 是 | kebab-case 标识符 |
| `description` | `string` | 是 | 路由描述 |
| `whenToUse` | `string` | 否 | 额外使用指引 |
| `path` | `string` | 否 | 绝对指令文件路径；虚拟 skill 无 |
| `source` | `string` | 是 | 发现来源（`bundled`/`project`/`user`/`runtime` 等） |
| `provider` | `string` | 是 | 拥有正文的 provider 名 |
| `invocation` | `object` | 是 | `{ modelInvocable: boolean, userInvocable: boolean }` |
- **错误**：`skills` 服务未挂载时返回 `-32017 SKILLS_UNAVAILABLE`。

#### `libadwaita/reference/list`

- **说明**：列出 `@` 文件引用候选。调用上游 `ctx.fileReferences.list(agent, query, signal)`。Bridge bundle 显式挂载 `@deepseek-ai/dsh-file-reference-local`，使该服务在 base 组合中可用；协议不伪造候选。
- **请求参数（`ReferenceListParams`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 是 | 提供 cwd 上下文的会话 |
  | `query` | `string` | 是 | `@` 后或 `@"` 后的路径文本 |
- **响应结果（`ReferenceListResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `candidates` | `Array<{ path: string, kind: 'file' \| 'directory' }>` | 是 | 确定性路径候选；目录保持补全打开 |
- **错误**：服务未挂载时返回 `-32018 REFERENCES_UNAVAILABLE`。

### 6.10 工作区持久化、权限、Goal 与会话内文件

#### `libadwaita/workspace/list`

- **说明**：列出持久化工作区记录。上游 `ctx.workspaceRegistry` 由 bridge bundle 显式挂载 `@deepseek-ai/dsh-workspace` 提供（base 默认不挂载）。`status` 为实时目录检查：`ok` 或 `missing-dir`。
- **请求参数**：无参数。
- **响应结果（`WorkspaceListResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `workspaces` | `WorkspaceView[]` | 是 | 工作区列表 |

`WorkspaceView` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `id` | `string` | 是 | 稳定工作区 id（uuid） |
| `path` | `string` | 是 | 规范化目录绝对路径 |
| `title` | `string` | 是 | 显示标题 |
| `createdAt` / `updatedAt` | `string` | 是 | ISO-8601 时间 |
| `sessionIds` | `string[]` | 是 | 已校验归属的会话 id 列表 |
| `status` | `'ok' \| 'missing-dir'` | 否 | 目录实时状态 |
- **错误**：服务不可用 `-32023 WORKSPACE_UNAVAILABLE`。

#### `libadwaita/workspace/create`

- **说明**：按 cwd 幂等创建工作区。先 `resolveByPath`；存在则返回 `created:false`。
- **请求参数**：`{ "path": string, "title"?: string }`
- **响应结果**：`{ "workspace": WorkspaceView, "created": boolean }`
- **错误**：路径不存在或非目录时返回上游错误（映射为 `-32023`）。

#### `libadwaita/workspace/rename`

- **说明**：重命名工作区显示标题。与 Web UI 一致，不允许与其它工作区重名。
- **请求参数**：`{ "workspaceId": string, "title": string }`
- **响应结果**：`{ "workspace": WorkspaceView }`
- **错误**：未知工作区 `-32021 WORKSPACE_NOT_FOUND`；重名 `-32022 WORKSPACE_NAME_CONFLICT`。

#### `libadwaita/workspace/remove`

- **说明**：移除工作区记录。不会删除或归档任何会话日志。
- **请求参数**：`{ "workspaceId": string }`
- **响应结果**：`{ "deleted": true }`
- **错误**：未知工作区 `-32021`。

#### `libadwaita/permission/catalog`

- **说明**：读取进程级权限预设目录。调用上游 `ctx.permissionPresets.catalog()`（base 默认挂载 `permission` 行）。
- **请求参数**：无参数。
- **响应结果（`PermissionCatalogResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `options` | `PresetOptionItem[]` | 是 | 当前可选预设 |
  | `defaultOptions` | `PresetOptionItem[]` | 是 | 可作为未来会话默认值的预设 |
  | `defaultPreset` | `string` | 是 | 配置缺省时的生效默认 |

`PresetOptionItem` 结构：`{ "value": string, "name": string, "description"?: string }`
- **错误**：服务未挂载 `-32024 PERMISSIONS_UNAVAILABLE`。

#### `libadwaita/permission/get`

- **说明**：读取会话当前权限值（配置预设、动态 `auto` 或派生 `custom`）。
- **请求参数**：`{ "sessionId": string }`
- **响应结果**：`{ "currentValue": string, "catalog": PermissionCatalogResult }`

#### `libadwaita/permission/set`

- **说明**：为会话选择权限预设。写入 `permission/preset` 持久事件，并通过 `permission.updated` 通知推送新值。
- **请求参数**：`{ "sessionId": string, "preset": string }`
- **响应结果**：`{ "currentValue": string }`
- **错误**：未知预设抛出上游错误（映射为 `-32024` 上下文错误信息）。

#### `libadwaita/goal/get`

- **说明**：读取会话当前 goal。上游 `ctx.goals` 为 base 默认服务。
- **请求参数**：`{ "sessionId": string }`
- **响应结果**：`{ "goal": GoalView \| null }`

`GoalView` 结构：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `id` | `string` | 是 | goal id |
| `revision` | `number` | 是 | 每次持久变更递增；CAS 依据 |
| `objective` | `string` | 是 | 目标文本 |
| `phase` | `'active' \| 'paused' \| 'blocked' \| 'complete'` | 是 | 持久阶段 |
| `blockedReason` | `object` | 否 | 仅 `blocked` 时存在 |
| `maxGoalRounds` | `number` | 是 | 自动延续轮次上限 |
| `roundsStarted` | `number` | 是 | 已开始的最高轮次 |
| `createdAt` / `updatedAt` | `number` | 是 | 毫秒时间戳 |
| `activation` | `'armed' \| 'disarmed'` | 是 | 进程内自动延续状态 |
- **错误**：服务未挂载 `-32025 GOAL_UNAVAILABLE`。

#### `libadwaita/goal/edit` / `goal/pause` / `goal/resume` / `goal/clear`

- **说明**：以 `{ id, revision }` 作为 CAS 引用执行目标编辑、暂停、恢复与清除。`clear` 成功后返回 `{ "goal": null }`。
- **请求参数**：`{ "sessionId": string, "ref": GoalRef, ... }`；`edit` 额外接受 `objective?` 与 `maxGoalRounds?`。
- **响应结果**：`{ "goal": GoalView \| null }`
- **错误**：服务未挂载 `-32025`；引用过期、无当前 goal 或非法状态迁移映射为 `-32026 GOAL_INVALID`，`data.code` 保留上游错误码（如 `GOAL_NOT_FOUND`）。

#### `libadwaita/files/list`

- **说明**：列出会话当前工作目录内的目录项。该方法使用 **bridge 本地安全文件访问**（非上游 workspaceFiles 服务）：以会话 cwd 的 `realpath` 为根，目标路径同样 `realpath` 后必须位于根内，拒绝逃逸与失效符号链接。
- **请求参数**：`{ "sessionId": string, "path"?: string }`（`path` 缺省为 `"."`，可为根内相对或绝对路径）
- **响应结果（`FilesListResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `path` | `string` | 是 | 根内显示路径 |
  | `entries` | `FileEntry[]` | 是 | 按名称排序；上限 2000 条 |

`FileEntry` 结构：`{ "name": string, "path": string, "kind": 'file' | 'directory' | 'symlink' | 'other', "size": number }`
- **错误**：目录不存在 `-32027 FILE_NOT_FOUND`；非目录 `-32028 FILE_ACCESS_DENIED`。

#### `libadwaita/files/stat`

- **说明**：查询根内路径元数据（`lstat`，不跟随符号链接）。
- **请求参数**：`{ "sessionId": string, "path": string }`
- **响应结果**：`{ "path": string, "kind": ..., "size": number, "mtime": number }`
- **错误**：`-32027` / `-32028`。

#### `libadwaita/files/read`

- **说明**：读取根内文本文件。默认最多读取 1 MiB（`maxBytes` 上限 8 MiB）；超出时 `truncated:true`。包含 NUL 字节或非法 UTF-8 时返回 `{ "kind": "binary", "size": ... }`，绝不返回二进制文本。
- **请求参数**：`{ "sessionId": string, "path": string, "maxBytes"?: number }`
- **响应结果（`FilesReadResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
| `kind` | `'text' \| 'binary'` | 是 | 内容分类 |
| `path` | `string` | 是 | 根内显示路径 |
| `text` | `string` | `text` 时 | 文本内容 |
| `size` | `number` | 是 | 原始文件大小 |
| `truncated` | `boolean` | `text` 时 | 是否被 `maxBytes` 截断 |

#### `libadwaita/files/readBytes`

- **说明**：读取根内文件字节并以 base64 返回。默认上限 16 MiB，超限拒绝（不截断）。
- **请求参数**：`{ "sessionId": string, "path": string, "maxBytes"?: number }`
- **响应结果**：`{ "path": string, "data": string, "size": number }`
- **错误**：`-32027`；非普通文件 `-32028`；超过上限 `-32029 FILE_TOO_LARGE`。

### 6.11 投影、工具结果、反馈、Jobs、搜索与附件

#### `libadwaita/session/projections`

- **说明**：读取会话当前可用的投影基线。上游 `ctx.sessionProjections.snapshot` 提供 `tokenUsage`、`contextPressure`、`contextBreakdown`（由 base 的 `token-meter` 行提供）；`goal` 由 `ctx.goals.get` 提供。上游 `session-stats` 与 `session-turn-outline` 仅在 web-app 挂载，因此 `turnOutline` 与 `sessionStats` 在 base 组合下**始终省略**，协议不会伪造这些字段。
- **请求参数**：`{ "sessionId": string }`
- **响应结果（`SessionProjectionsResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `asOfSeq` | `number` | 是 | 基线所处日志游标 |
  | `tokenUsage` | `unknown` | 否 | 累计 token 用量（上游存在时） |
  | `contextPressure` | `unknown` | 否 | 最新请求压力与容量 |
  | `contextBreakdown` | `unknown` | 否 | system/tools/message 组成 |
  | `goal` | `GoalView \| null` | 否 | 当前 goal |
  | `turnOutline` / `sessionStats` | `unknown` | 否 | 仅在未来挂载对应上游 provider 时存在 |
- **增量通知**：见 §7.10 `session.projection`。

#### `libadwaita/session/search`

- **说明**：元数据搜索。按标题与 cwd 的子串（大小写不敏感）过滤实时会话并按最近事件时间倒序返回。**不是全文搜索**：base 的 `session-query-sqlite` 以 `openAt: never` 挂载，全文 provider 默认禁用（见 `packages/bundle/base/cordis.patch.yml` 第 149–161 行）。若未来 profile 显式启用全文 provider，本协议仍需另行评估。
- **请求参数**：`{ "query": string, "limit"?: number, "archived"?: boolean }`
- **响应结果（`SessionSearchResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `items` | `SessionSearchItem[]` | 是 | `{ id, title, cwd, updatedAt }` |
  | `hasMore` | `boolean` | 是 | 是否超出 `limit` |
  | `fullText` | `false` | 是 | 恒为 `false`，标识仅元数据匹配 |

#### `libadwaita/tool-result/watch`

- **说明**：订阅或取消会话的工具结果通知。上游 `sessionToolResults`（`toolResults.watch`）属于 web-app 的 trajectory Host 插件，base 不可用；本方法由 bridge **派生实现**：对已 follow 会话，把每条 `session.event` 的 `tool/result` 直接转为 `tool-result.updated`。只覆盖持久化后的最终结果，不承诺原始未落盘中间值。
- **请求参数**：`{ "sessionId": string, "subscribe": boolean }`
- **响应结果**：`{ "sessionId": string, "subscribed": boolean }`
- **通知**：见 §7.11 `tool-result.updated`。

#### `libadwaita/message-feedback/list` / `put` / `delete`

- **说明**：消息反馈。bridge bundle 挂载 `@deepseek-ai/dsh-message-feedback`（`maxNoteBytes: 8192`），复用上游持久化服务与 `ifVersion` CAS 语义。
- **`list`**：`{ "sessionId": string }` → `{ "items": MessageFeedbackItem[] }`。
- **`put`**：`{ "sessionId", "messageId", "rating": 'positive' | 'negative', "note"?: string, "category"?: string, "ifVersion": string | null }` → `{ "item": MessageFeedbackItem }`。`ifVersion` 为 `null` 表示要求此前无反馈项；不匹配时返回 `-32031 FEEDBACK_CONFLICT`（`data.item` 为权威当前项）。
- **`delete`**：`{ "sessionId", "messageId", "ifVersion": string }` → `{ "absent": true }`；项已不存在时幂等成功。
- **`MessageFeedbackItem`**：`{ messageId, rating, note?, category?, version, createdAt, updatedAt }`。
- **错误**：目标消息不存在 `-32032 FEEDBACK_TARGET_NOT_FOUND`；版本冲突 `-32031`；会话不存在 `-32002`。

#### `libadwaita/session-feedback/record`

- **说明**：会话级反馈（base 已挂载 `command-feedback`）。
- **请求参数**：`{ "sessionId": string, "text"?: string, "category"?: string }`
- **响应结果**：`{ "recorded": true }`；会话不存在返回 `-32002`。

#### `libadwaita/question/reopen`

- **说明**：取回仍在等待回答的用户提问快照。Bridge 在为 `user-question/ask` 建立请求时保存 payload，响应后清除。上游没有独立 `question/reopen` RPC；GUI 隐藏卡片后可据此恢复同一请求，并以原 JSON-RPC `id` 回复。迟到回答仍由 fail-closed 规则仲裁。
- **请求参数**：`{ "sessionId": string }`
- **响应结果**：`{ "request": SdkUserQuestionRequest }`
- **错误**：当前无等待中的提问 `-32030 QUESTION_NOT_PENDING`。

#### `libadwaita/job/list`

- **说明**：列出会话拥有的后台 jobs（base 的 `jobs-local` 服务）。`JobWireView` 复用上游 `JobView`：`id`、`kind`、`label`、`owner?`、`status`、`progress?`、`detail?`、`startedAt`、`finishedAt?`、`output:{total, earliest, spillPaths?}`。
- **请求参数**：`{ "sessionId": string }`
- **响应结果**：`{ "jobs": JobWireView[] }`
- **错误**：未知或跨会话 job `-32033 JOB_NOT_FOUND`。

#### `libadwaita/job/follow`

- **说明**：读取 job 的当前输出并开启 250ms 间隔的输出跟随。首个结果包含 `readAt(from)` 的块、下一偏移和 job 快照；随后用 `job.output` 通知推送新增块。job 进入终态后自动停止跟随。
- **请求参数**：`{ "sessionId": string, "jobId": string, "from"?: number }`
- **响应结果（`JobFollowResult`）**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `job` | `JobWireView` | 是 | job 快照 |
  | `chunks` | `JobChunkWire[]` | 是 | 输出块：`{ at, text, channel?, gapBefore? }` |
  | `next` | `number` | 是 | 下次读取偏移 |
  | `lossy` | `boolean` | 是 | 是否因保留窗口丢弃了更早字节 |

#### `libadwaita/job/kill`

- **说明**：请求取消 job。返回上游 `requested`（运行中）或 `already-finished`（已结束）。
- **请求参数**：`{ "sessionId": string, "jobId": string, "reason"?: string }`
- **响应结果**：`{ "status": 'requested' | 'already-finished' }`

#### `libadwaita/attachment/upload`

- **说明**：桥本地附件暂存（上游通用 `fileUploads` 为 web-app 服务）。base64 解码后写入 `$DSH_HOME/attachments/libadwaita/<sessionId>/<uuid>__<safeName>`（目录 0700、文件 0600），单附件上限 16 MiB，名称做 basename 与字符清洗，禁止路径穿越。
- **请求参数**：`{ "sessionId": string, "name": string, "data": string, "mimeType"?: string }`
- **响应结果**：`{ "attachmentId": string, "name": string, "size": number, "mimeType": string }`
- **错误**：大小超限 `-32036 ATTACHMENT_TOO_LARGE`；参数/编码非法 `-32037 ATTACHMENT_INVALID`。

#### `libadwaita/attachment/read`

- **说明**：按 `attachmentId` 读取本会话暂存附件；`encoding: 'text'` 时要求 UTF-8 且无 NUL，否则返回错误，不提供伪造文本。
- **请求参数**：`{ "sessionId": string, "attachmentId": string, "encoding"?: 'base64' | 'text' }`
- **响应结果**：`{ "attachmentId", "name", "size", "mimeType", "encoding", "data" }`
- **错误**：不存在 `-32035 ATTACHMENT_NOT_FOUND`；非法 id/编码 `-32037`。
- **prompt 图片链路**：`session/prompt` 的 `{type:'image', data, mimeType, name?}` 先经上游 `ctx.attachments.saveImages` 验证与持久化，再以 `{type:'image', attachment: ref}` 进入模型历史，复用 base 的 `attachment-local` 限制。
---

### 6.12 插件生命周期、模型发现、子代理、Team、凭据记录与账号

#### 插件生命周期

`plugin/detail` 参数 `{name, language?, npmFileSources?}`，`plugin/inspect` 参数 `{spec, registry?}`，`plugin/install` 参数 `{spec, enabled?, saveExact?, requestId?, approvedBuilds?, registry?}`，`plugin/cancel-install` / `wait-install` 参数 `{requestId}`，`plugin/remove` 参数 `{name}`，`plugin/versions` 参数 `{name, registry?}`，`plugin/update` 参数 `{name, version, requestId?, registry?}`，`plugin/registries` 无参数。以上均返回 `{result: unknown}`，原样承载上游 plugin-manager 结果；`plugin/inventory` 返回 `{plugins, bundles}`，`plugin/retry` 为同一清单重读。`npmFileSources` 为 npm 文件 CDN 优先序列（如 `["jsdelivr","npmmirror"]`），在 Bundle 文件不可用时供上游从 CDN 读取 README；`update` 的 `requestId` 用于关联 `plugin.update-state` 通知与安装日志/取消。安装过程通过 `plugin.install-state` / `plugin.install-log` 通知推送。网络安装需要用户环境可达 registry；本地路径/git/压缩包规格由上游 inspect/install 处理。**限制**：README 详情文档中的内联图片依赖 Host 路由 `api/plugin-manager/detail-file`，本客户端未实现图片代理，GUI 只渲染 Markdown 文本并忽略图片（不在本次协议范围）。

#### 插件市场（Discover）

Bridge bundle patch 挂载 `@deepseek-ai/dsh-plugin-market`（行 `plugin-market`，无 profile 时 disabled）；服务名 `ctx.pluginMarket`，依赖 `ctx.pluginManager`，无索引源时所有读取返回空/拒绝，不访问网络。

- `plugin-market/list-indexes` 无参数 → `{result: MarketIndexSummary[]}`，`MarketIndexSummary = {url, title?, description?, error?}`（`title`/`description` 为本地化文本对象；`error` 为上次读取失败信息）。永不联网。
- `plugin-market/set-indexes` 参数 `{urls: string[]}` → `{result: null}`；经 Settings 持久化，非法/重复 URL 返回 `-32038`（`data` 保留上游原因），提交后推送 `plugin.market-changed`。
- `plugin-market/search` 参数 `{url, query}` → `{result: MarketSearchResult}`；`query` 为上游 `MarketQuery`：`{text?, category?, labels?, sort: {kind:'index'}|{kind:'relevance'}|{kind:'published',direction}|{kind:'key',id,direction}, offset, limit, refresh?, fetchedAt?}`。`url` 必须是已配置索引源。
- `plugin-market/read-entry-detail` 参数 `{url, name, version, language?, refresh?, npmFileSources?}` → `{result: PluginDetail}`；`PluginDetail = {status:'found', origin: 'locale'|'readme'|'repository', markdown, document} | {status:'none'}`。**限制**：市场详情文档中的内联图片需要 Host HTTP 路由 `api/plugin-market/detail-file` 解析，本客户端不实现该图片代理，GUI 应忽略或占位显示图片（不在本次协议范围）。

#### 模型发现

`model/discover` 参数 `{settingsNs, provider?, baseURL?, api?, apiKey?}` → `{models: unknown[]}`。调用 `ctx.llm.discoverModels`；`apiKey` 仅用于本次查询，bridge 不持久化。base 仅 `llm-pi-ai` 注册 discovery，且协议必须由该构建支持模型列表；不支持时返回 `-32044`。`web-search/providers` 返回 `{search, fetch}`，来自 `ctx.web.listProviders()`；base 组合下 provider 列表可能为空，不伪造。

#### 子代理

- `subagent/list` `{sessionId}` → `{entries}`（`ctx.subagents.listDescendants`）。
- `subagent/prompt` `{sessionId, childSessionId, requestId?, delivery?: 'queue'|'steer', content}` → `{messageId}`；图片内容由上游 admission 处理。
- `subagent/interrupt` `{sessionId, childSessionId}` → `{accepted:true}`；优先 `interruptByUser`，否则调用公开 `interrupt(child,{kind:'user'})`。

#### Agent Team

- `agent-team/get` `{sessionId}` → `{enabled, tasks}`。
- `agent-team/set-enabled` `{sessionId, enabled}` → `{enabled}`。
- base-only profile 下 `ctx.agentTeams` 在本机集成验证中未激活（见 §11），调用返回 `-32040`，不伪造 Team 状态。

#### 凭据记录（脱敏）

`credentials/records/list` → `{records:[{key,kind,configured,writable}]}`；`read` `{key}` 仅返回 `{configured,kind,writable,hasKey?,envNames?,payloadRedacted?}`，**绝不返回 key/payload**；`modify` `{key,record}` 写入 api-key/grant；`delete` `{key}` 幂等删除。复用 bridge 内存 CredentialProvider。

#### 账号

`account/status` 返回 `{state}`，`account/profile` / `account/balance` 返回只读资料/余额，`account/sign-out` 返回 `{state}`。均调用 base 的 `ctx.deepseekAccount`；浏览器 OAuth 登录流程依赖 Host webServer 回调，不在本协议内实现（见 §11）。

### 6.13 计划任务与交互终端

#### 计划任务

Bridge bundle patch 挂载 `@deepseek-ai/dsh-schedule`；服务名 `ctx.schedule`，真实 dsh 集成测试已验证创建（含 `new_once` 目标）/运行/历史/删除/清理。

- `schedule/catalog` 无参数 → `{result: ScheduleCatalogEntry[]}`。
- `schedule/create` `{title, prompt, timing, target?}` → `{result: ScheduleCatalogEntry}`。`timing` 为上游 `ScheduleTiming`（`after`/`at`/`every`/`daily`/`weekly`/`cron`/`rrule`，字段为 camelCase：`afterSeconds` 等），`target` 为 `{kind:'session',sessionId}`、`{kind:'new_once', create?}` 或 `{kind:'new', create?}`；`create` 为 `{cwd?, agentPreset?}`。
- `schedule/edit` `{id, action:'update'|'delete', patch?, expectedUpdatedAt?}` → `{result}`。
- `schedule/run` `{id}` → `{result}`；成功投递时 `result.lastDelivery` 为 `{scheduledAt, deliveredAt, messageId, targetSessionId}`；`{id, code:'schedule_execution_pending'}` 表示该任务已有执行在排队/运行或处于投递下限内。`schedule/history` `{id, limit?, before?}` → `{result: {records, retention, earlierRecordsPruned, earlierRecordsUnavailable, nextBefore?}}`。
- `schedule/delete` `{id, expectedUpdatedAt?}` → `{result:{deleted:true}}`（等价 edit delete）。
- `schedule/delete-inactive` 无参数 → `{result:{deleted:number}}`，删除全部 `status:'ended'` 任务（active/paused 保留）。
- `schedule/delivery-limits` 无参数 → `{result:{newSessionIntervalSeconds:number}}`，即 `target.kind:'new'` 任务两次投递的最小间隔。
- **Session 激活 Seam**：bridge 提供 `sessionController` 服务（`create({sessionId?, cwd?, agentPreset?})` 复用 `session/create` 的 agents.create + 默认模型选择；`resolveAgent(sessionId)` 返回 `{agent}` 或 `{error}`，内部对未创建会话抛出 `SessionPersistenceNotFoundError`），并注册 `schedule/resolve-agent` waterfall 监听（先尝试 `agents.get`，再 `agents.resume`，失败时交回 `next()`）。这使得 `new_once`/`new` 目标可创建会话、`schedule/run` 与定时投递可激活/恢复目标会话。
- **错误码保留**：上游 `ScheduleInputError`（`code` 如 `invalid_rule`、`not_future`、`subagent_session`）与 `RemoteError`（如 `schedule/target-archived`）经 `-32045 SCHEDULE_OPERATION_FAILED` 返回，`data.code` 为上游字符串码、`data.details` 为上游结构化载荷；服务未挂载仍为 `-32034`。
- 任务变更发出 `schedule.changed`（§7.14）。

#### 交互终端

Bridge patch 挂载 `@deepseek-ai/dsh-terminal` 与 `@deepseek-ai/dsh-terminal-bash`（`backendType: shell`）；真实 dsh 集成测试已验证创建、输入 `echo hi`、读取输出与关闭。

- `terminal/create` `{sessionId, cwd?}` → `{terminalId, sessionId, pid, cwd}`；cwd 默认取会话当前工作目录，服务端按 owner Agent 授权。
- `terminal/list` `{sessionId}` → `{terminals:[{terminalId,pid,cwd,running}]}`。
- `terminal/input` `{sessionId, terminalId, data}` → `{result: TerminalSendResult}`；`submit` 由末尾换行推导。
- `terminal/output` `{sessionId, terminalId, offset?, count?}` → `{text,totalLines,lineBegin,lineEnd,truncated}`（有界回滚缓冲）。
- `terminal/close` `{sessionId, terminalId}` → `{closed:boolean}`。
- **限制**：上游终端服务无独立 resize RPC；恢复视图通过 `terminal/list` + `terminal/output` 重建。终端随 Agent/session 释放由上游 owner cleanup 终止。

## 7. 服务端推流通知（Server-to-Client Notifications）

通知一律无 `id` 字段。客户端被动接收并更新本地领域模型。

### 7.1 `session.event`：持久化事件流

严格复用上游 `@deepseek-ai/dsh-session` 的 `SessionEvent` 数据结构。**禁止自造第二套事件模型**。

#### 载荷（`SessionEventNotification`）
```json
{
  "jsonrpc": "2.0",
  "method": "session.event",
  "params": {
    "sessionId": "sess-uuid",
    "event": {
      "type": "tool/call",
      "seq": 4,
      "time": 1762419200123,
      "data": {
        "turn": 1,
        "step": 0,
        "callId": "call-1",
        "name": "read",
        "arguments": "{\"path\":\"src/main.rs\"}"
      }
    }
  }
}
```

`event` 结构字段：
| 字段 | 类型 | 必选 | 说明 |
|---|---|---|---|
| `type` | `string` | 是 | 事件类型鉴别符（来自 `SessionEventType`，如 `user/message`、`assistant/message`、`tool/call`、`tool/result`、`todo/write`、`plan/mode`、`workspace/changes`、`command/run`、`turn/end` 等） |
| `seq` | `number` | 是 | 会话内单调递增的安全非负整数 |
| `time` | `number` | 是 | Unix Epoch 毫秒时间戳 |
| `data` | `unknown` | 是 | 对应 `SessionEventMap[type]` 的强类型载荷 |
| `ignorable` | `true` | 否 | 若存在且为 `true`，表示该事件为纯信息型记录，客户端遇到无法识别的 `type` 时可安全忽略 |
| `surfaceOp` | `string \| object` | 否 | 仅在 Surface 事件（`user/message`, `assistant/message`, `tool/result`, `system/message`）上出现 |
| `sourceEventSeqs` | `number[]` | 否 | 引用前置事件的序号列表 |

### 7.2 `agent.stream`：实时增量帧

转发自上游 process-local 的 `agent/assistant-stream` 事件。增量块主要用于在 LLM 生成过程中向 GTK 文本视图实时刷入 token，**不落盘**。在生成定稿后，上游会追加一条包含完整 stream 的 `assistant/message` 持久化事件，并发送 `end` 帧。

#### 帧类型定义（`AssistantStreamFrame`）
载荷包含 `sessionId` 与 `frame`：

1. **`start` 帧**：
   ```json
   {
     "jsonrpc": "2.0",
     "method": "agent.stream",
     "params": {
       "sessionId": "sess-uuid",
       "frame": {
         "type": "start",
         "attemptId": "att-1",
         "revision": 1,
         "turn": 1,
         "step": 0
       }
     }
   }
   ```
2. **`chunk` 帧**：
   ```json
   {
     "jsonrpc": "2.0",
     "method": "agent.stream",
     "params": {
       "sessionId": "sess-uuid",
       "frame": {
         "type": "chunk",
         "attemptId": "att-1",
         "revision": 1,
         "index": 0,
         "time": 1762419200456,
         "chunk": {
           "type": "text-delta",
           "index": 0,
           "text": "Hello"
         }
       }
     }
   }
   ```
   `chunk` 联合体类型（`StreamChunk`，引自 `@deepseek-ai/dsh-llm`）：
   - `text-delta`：`{ "type": "text-delta", "index": number, "text": string }`
   - `reasoning-delta`：`{ "type": "reasoning-delta", "index": number, "text": string }`（推理/思考内容增量）
   - `tool-call-delta`：`{ "type": "tool-call-delta", "index": number, "id": string, "name"?: string, "argumentsDelta": string }`
   - `block-start`：`{ "type": "block-start", "index": number, "blockType": string }`
   - `block-end`：`{ "type": "block-end", "index": number, "block": ContentBlock }`
   - `usage`：`{ "type": "usage", "usage": TokenUsage }`
   - `finish`：`{ "type": "finish", "reason": string }`
3. **`end` 帧**：
   ```json
   {
     "jsonrpc": "2.0",
     "method": "agent.stream",
     "params": {
       "sessionId": "sess-uuid",
       "frame": {
         "type": "end",
         "attemptId": "att-1",
         "revision": 1,
         "index": 12,
         "outcome": {
           "kind": "committed",
           "eventType": "assistant/message",
           "seq": 5
         }
       }
     }
   }
   ```
   `outcome` 可能为：
   - `{ "kind": "committed", "eventType": "assistant/message" | "assistant/attempt", "seq": number }`（正常定稿提交）
   - `{ "kind": "abandoned" }`（流被中断或废弃，不落盘）

### 7.3 `session.status`：运行状态变化通知

#### 载荷（`SessionStatusNotification`）
```json
{
  "jsonrpc": "2.0",
  "method": "session.status",
  "params": {
    "sessionId": "sess-uuid",
    "status": "running"
  }
}
```
- `status` 取值仅限 `'idle' | 'running'`。

### 7.4 `session.title`：标题更新通知

当第一轮 prompt 完成后 LLM 自动生成标题，或用户手动重命名会话时推送。

#### 载荷
```json
{
  "jsonrpc": "2.0",
  "method": "session.title",
  "params": {
    "sessionId": "sess-uuid",
    "title": "实现 Bridge 协议解析器",
    "source": { "kind": "provider", "provider": "first-prompt-llm" }
  }
}
```

### 7.5 `user-question.cancelled`：提问取消通知

当运行时因超时、turn 取消或其它原因不再等待先前提出的提问时推送。客户端收到后应关闭正在展示的提问面板。

#### 载荷
```json
{
  "jsonrpc": "2.0",
  "method": "user-question.cancelled",
  "params": {
    "requestId": "q-req-1",
    "sessionId": "sess-uuid"
  }
}
```

### 7.6 `session.inbox`：队列快照通知

当已 follow 的会话发生 `agent/inbox/spliced` 事件时推送当前完整 inbox 快照。GUI 以该快照整体替换本地队列；不得据此推断未包含的增量。

#### 载荷（`SessionInboxNotification`）
```json
{
  "jsonrpc": "2.0",
  "method": "session.inbox",
  "params": {
    "sessionId": "sess-uuid",
    "items": [
      {
        "id": "msg-queued-1",
        "placement": "queued",
        "content": [{ "type": "text", "text": "下一条消息" }]
      }
    ]
  }
}
```
- `items` 与 `libadwaita/session/queue/list` 的返回结构完全一致。

### 7.7 `settings.updated`：设置命名空间变更通知

当 profile 设置文档发生外部或桥内变更时推送（上游 `settings/document-updated` 事件）。

#### 载荷（`SettingsUpdatedNotification`）
```json
{
  "jsonrpc": "2.0",
  "method": "settings.updated",
  "params": { "namespace": "agent-loop", "revision": 1 }
}
```

### 7.8 `permission.updated`：会话权限变更通知

当已 follow 的会话记录 `permission/preset`、`sandbox/mode` 或 `approval/policy` 事件时推送当前权限值。

#### 载荷（`PermissionUpdatedNotification`）
```json
{
  "jsonrpc": "2.0",
  "method": "permission.updated",
  "params": { "sessionId": "sess-uuid", "currentValue": "workspace-write" }
}
```

### 7.9 `goal.updated`：会话 Goal 变更通知

当已 follow 的会话发生 `goal/change` 事件或 goal 激活状态变化时推送完整 `GoalView`；清除后 `goal` 为 `null`。

#### 载荷（`GoalUpdatedNotification`）
```json
{
  "jsonrpc": "2.0",
  "method": "goal.updated",
  "params": {
    "sessionId": "sess-uuid",
    "goal": {
      "id": "goal-uuid",
      "revision": 2,
      "objective": "Ship batch B safely",
      "phase": "active",
      "maxGoalRounds": 256,
      "roundsStarted": 0,
      "createdAt": 1762419200000,
      "updatedAt": 1762419201000,
      "activation": "armed"
    }
  }
}
```

### 7.10 `session.projection`：投影增量通知

当已 follow 会话的 `assistant/message`、`step/end`、`turn/end` 或 `compaction/end` 引起投影值变化时推送。Bridge 对 `tokenUsage`、`contextPressure`、`contextBreakdown` 做签名去重；`revision` 取触发该变化的事件 seq 的单调最大值。`kind` 为投影键名。

#### 载荷（`SessionProjectionNotification`）
```json
{
  "jsonrpc": "2.0",
  "method": "session.projection",
  "params": {
    "sessionId": "sess-uuid",
    "kind": "tokenUsage",
    "value": { "inputTokens": 120, "outputTokens": 16, "totalTokens": 136 },
    "revision": 14
  }
}
```

### 7.11 `tool-result.updated`：工具结果通知

已订阅会话的 `tool/result` 事件推送（bridge 派生）。`seq` 为事件序号，`data` 为原始事件 `data` 字段；`callId`、`name` 在可用时附带。

#### 载荷（`ToolResultUpdatedNotification`）
```json
{
  "jsonrpc": "2.0",
  "method": "tool-result.updated",
  "params": {
    "sessionId": "sess-uuid",
    "seq": 21,
    "callId": "call_1",
    "name": "read",
    "data": { "turn": 1, "step": 1, "message": { "role": "tool" } }
  }
}
```

### 7.12 `job.output`：后台 Job 输出通知

跟随中的 job 每 250ms 轮询输出并推送新增块；`job` 为最新快照，`next` 为下一读取偏移。

#### 载荷（`JobOutputNotification`）
```json
{
  "jsonrpc": "2.0",
  "method": "job.output",
  "params": {
    "sessionId": "sess-uuid",
    "jobId": "bash-abc123",
    "job": {
      "id": "bash-abc123",
      "kind": "bash",
      "label": "sleep 3; echo hi",
      "owner": "sess-uuid",
      "status": "running",
      "startedAt": 1762419200000,
      "output": { "total": 5, "earliest": 0 }
    },
    "chunks": [{ "at": 0, "text": "hello\n", "channel": "stdout" }],
    "next": 5,
    "lossy": false
  }
}
```

---

### 7.13 `plugin.install-state` / `plugin.install-log`：安装进度通知

`plugin.install-state` 载荷 `{requestId, phase, attempt?}`，phase 为 `installing` / `cancelling` / `applying`。`plugin.install-log` 载荷 `{requestId?, jobId, argv, cwd, stream, text, exitCode?}`，为 pnpm/安装运行的一次输出块。

```json
{"jsonrpc":"2.0","method":"plugin.install-state","params":{"requestId":"req-1","phase":"installing"}}
{"jsonrpc":"2.0","method":"plugin.install-log","params":{"requestId":"req-1","jobId":"job-1","argv":["pnpm","add","file:/tmp/plugin"],"cwd":"/home/u/.local/share/dsh-libadwaita/profiles/libadwaita","stream":"stdout","text":"Progress: resolved 1","exitCode":0}}
```

### 7.14 `schedule.changed`：计划任务变更通知

上游 `schedule/changed` 事件转发，载荷 `{changed:true}`；GUI 收到后重新调用 `schedule/catalog`。
```json
{"jsonrpc":"2.0","method":"schedule.changed","params":{"changed":true}}
```

### 7.15 `plugin.changed` / `plugin.update-state` / `plugin.market-changed`：插件生命周期通知

- `plugin.changed`：上游 `plugin-manager/changed` 事件转发，载荷 `{reason}`，`reason` 为 `"plugin" | "bundle" | "install" | "remove"`。GUI 收到后重新调用 `plugin/inventory`（并刷新当前详情页/市场安装状态）。
- `plugin.update-state`：上游 `plugin-manager/update-state` 事件转发，载荷 `{requestId, step}`，`step` 为 `"checking" | "removing" | "installing" | "restoring"`；仅当 `plugin/update` 携带 `requestId` 时产生。
- `plugin.market-changed`：上游 `plugin-market/changed` 事件转发，载荷 `{changed:true}`；GUI 收到后重新调用 `plugin-market/list-indexes`。
```json
{"jsonrpc":"2.0","method":"plugin.changed","params":{"reason":"plugin"}}
{"jsonrpc":"2.0","method":"plugin.update-state","params":{"requestId":"req-update-1","step":"checking"}}
{"jsonrpc":"2.0","method":"plugin.market-changed","params":{"changed":true}}
```


## 8. 双向交互请求（Server-to-Client Requests）与 Fail-Closed 语义

此类消息由 bridge 主动发起，GUI 作为响应端。GUI 处理完毕后必须回传具有相同 `id` 的响应帧。

### 8.1 操作审批：`approval/request`

当 Agent 调用了需要人工授权的危险工具（如执行 bash 命令、写入特定系统文件）时触发。

#### 请求参数（`ApprovalRequestParams`）
```json
{
  "jsonrpc": "2.0",
  "id": "appr-1",
  "method": "approval/request",
  "params": {
    "sessionId": "sess-uuid",
    "toolName": "bash",
    "callId": "call-123",
    "reason": "执行系统构建命令"
  }
}
```

#### 客户端响应结果（`ApprovalRequestResult`）
```json
{
  "jsonrpc": "2.0",
  "id": "appr-1",
  "result": {
    "outcome": "allowed-once"
  }
}
```
`outcome` 取值约束：
- `"allowed-once"`：仅允许本次调用（上游通用工具审批**无 always-allow** 语义）。
- `"rejected"`：用户显式拒绝该操作。
- `"cancelled"`：用户取消或弹窗关闭。

可选扩展字段（直接透传上游 `ApprovalRequestEvent`）：`displayReason?: Record<string, string>` 为本地化展示文案。`displayReason` 不参与持久化审计，仅用于 GUI 展示；缺省时 GUI 回退到 `reason`。

### 8.2 用户提问与计划评审：`user-question/ask`

用于向用户索取必要信息、分支选择，以及**计划模式下的 Plan Review**。

#### 请求参数（`SdkUserQuestionRequest`）
```json
{
  "jsonrpc": "2.0",
  "id": "q-1",
  "method": "user-question/ask",
  "params": {
    "requestId": "q-req-1",
    "sessionId": "sess-uuid",
    "questions": [
      {
        "id": "plan-review",
        "question": "审查执行计划",
        "detail": "### 变更计划\n1. 修改 Cargo.toml\n2. 实现协议解析",
        "intent": {
          "kind": "plan-review",
          "approve": "同意执行",
          "callId": "call-exit-plan"
        },
        "options": [
          { "label": "同意执行", "description": "退出计划模式并开始执行" },
          { "label": "要求修改", "description": "留在计划模式，要求 AI 调整方案" }
        ],
        "multiSelect": false
      }
    ]
  }
}
```

可选扩展字段（直接透传上游 `AskUserQuestionRequestEvent.wait`）：`callId?: string`（问题对应的工具调用）与 `timed?: boolean`（是否为前台计时等待）。`remainingMs` 需要持续消费上游 `attachWait` 流，本版不伪造固定值；GUI 的倒计时从 `timed` 与本地收到的等待状态推导。

#### 客户端正常答复响应（`SdkUserQuestionResult`）
```json
{
  "jsonrpc": "2.0",
  "id": "q-1",
  "result": {
    "answers": [
      {
        "id": "plan-review",
        "selected": ["同意执行"]
      }
    ]
  }
}
```

#### 客户端放弃或取消响应
```json
{
  "jsonrpc": "2.0",
  "id": "q-1",
  "result": {
    "cancelled": true
  }
}
```

### 8.3 Plan Review 专用流程与“要求修改”语义

根据 `docs/dsh-web-feature-inventory.md` 的架构审计事实：
1. **通道归属**：Plan review 走 `ctx.userQuestions.ask`（携带 `intent: { kind: 'plan-review' }`），**绝对不是**走 `approval/request`。
2. **同意执行**：用户在 GUI 点击“同意执行”，返回 `selected: ["同意执行"]`（对应 `intent.approve` 指定的 label 值），底层 `exit_plan_mode` 成功，关闭 plan mode，进入正式执行。
3. **要求修改**：用户在 GUI 点击“要求修改”，客户端**必须返回 `{ "cancelled": true }`**。服务端收到后向提问者抛出 `ASK_CANCELLED`，会话保持在 plan mode（`active: true` 不撤销），Agent 接收到修改信号后在当前计划轮次中继续优化方案。

### 8.4 迟到回答（Late Answers）与超时处理

1. **倒计时与超时**：GUI 界面可以为提问展示倒计时。若在客户端超时未作答，底层可能会因等待超时解除阻塞并标记 `ASK_TIMED_OUT`。
2. **迟到提交**：若用户在超时后依然在界面上点击了提交，GUI 发出 response。若服务端检测到该 question 对应的 tool call 仍允许 `attachWait` 挂载，答案仍会被接收并落盘为 `user-question-reply`；若底层 turn 已彻底关闭且无法挂载，服务端返回错误码 `-32005`（`QUESTION_ALREADY_SETTLED`）。

### 8.5 严格 Fail-Closed（故障关断）安全原则

本协议对所有挂起的双向交互请求实施严格的 Fail-Closed 保证：
1. **连接异常断开（Socket Drop）**：若 Unix Socket 连接意外断开、GUI 崩溃或退出，bridge 内部必须将当前所有等待中的 `approval/request` **立即判定为 `rejected`**，将所有等待中的 `user-question/ask` **立即判定为 `cancelled`**。绝不允许在失去人工监督的情况下自动放行特权操作。
2. **客户端超时或无应答**：若客户端在限定时间（默认 300 秒）内未回传响应帧，服务端自动注销该等待并执行 Fail-Closed 拒绝。

---

## 9. 错误码与异常分类体系

协议严格区分 JSON-RPC 2.0 规范标准错误与 bridge 业务扩展错误。

### 9.1 JSON-RPC 2.0 预定义错误码
| 错误码 | 名称 | 触发条件 |
|---|---|---|
| `-32700` | Parse error | 接收到的行不是有效的 JSON 字符串 |
| `-32600` | Invalid Request | 缺少 `jsonrpc` 标识或基本结构不合法 |
| `-32601` | Method not found | 请求的 `method` 不存在于当前协议中 |
| `-32602` | Invalid params | 参数类型错误、必选字段缺失或不符合 schema |
| `-32603` | Internal error | 服务端未捕获的内部非预期异常 |

### 9.2 Bridge 业务专属错误码（-32000 ~ -32099）
| 错误码 | 标识常量 | 含义与处置策略 |
|---|---|---|
| `-32000` | `BRIDGE_INTERNAL_ERROR` | bridge 内部一般性错误，详见 `message` 与 `data` |
| `-32001` | `PROTOCOL_VERSION_MISMATCH` | 协议版本不匹配。握手协商失败，GUI 必须展示版本升级弹窗 |
| `-32002` | `SESSION_NOT_FOUND` | 指定的 `sessionId` 不存在或已被清理 |
| `-32003` | `SESSION_BUSY` | 会话当前正在运行 turn，无法执行并发的冲突操作 |
| `-32004` | `TURN_NOT_CANCELLABLE` | 请求取消当前 turn 时，会话实际处于空闲态（idle） |
| `-32005` | `QUESTION_ALREADY_SETTLED` | 提问已超时结算或已被取消，迟到的答案被拒绝 |
| `-32006` | `APPROVAL_FAIL_CLOSED` | 审批因超时、用户取消或连接断开被安全拒绝 |
| `-32007` | `CREDENTIAL_REJECTED` | 内存注入的凭据格式非法或被提供商验证拒绝 |
| `-32008` | `MODEL_NOT_CONFIGURED` | 请求的 Provider/Model 尚未注入凭据或未适配 |
| `-32009` | `SEQUENCE_GAP_UNRECOVERABLE`| 事件序号断档且缺口超出修复窗口，GUI 必须重置基线（Rebaseline） |
| `-32010` | `RESTART_REQUIRED` | 插件配置已变更，由于 HMR 禁用，需要重启 dsh 才能生效 |
| `-32011` | `WORKDIR_UNAVAILABLE` | 请求的工作目录不存在、非目录或当前进程无权访问 |
| `-32012` | `TITLE_INVALID` | 重命名标题规范化后为空或全空白 |
| `-32013` | `QUEUE_ITEM_NOT_FOUND` | 队列项已被消费或移除，无法编辑/删除 |
| `-32014` | `QUEUE_STEER_UNAVAILABLE` | 当前 turn 已不接受 steer，或项不在 `nextTurn` |
| `-32015` | `COMMANDS_UNAVAILABLE` | `commands` 服务未在当前组合中挂载 |
| `-32016` | `COMMAND_NOT_FOUND` | slash 命令名无法解析 |
| `-32017` | `SKILLS_UNAVAILABLE` | `skills` 服务未在当前组合中挂载 |
| `-32018` | `REFERENCES_UNAVAILABLE` | `fileReferences` 服务未在当前组合中挂载 |
| `-32019` | `FORK_UNAVAILABLE` | 指定边界不存在、无已完成 turn 或创建子会话失败 |
| `-32020` | `SETTINGS_CONFLICT` | 设置命名空间 `expectedRevision` 与当前修订不一致（`data` 含 expected/actual） |
| `-32021` | `WORKSPACE_NOT_FOUND` | 指定工作区 id 不存在 |
| `-32022` | `WORKSPACE_NAME_CONFLICT` | 工作区标题与其它记录冲突 |
| `-32023` | `WORKSPACE_UNAVAILABLE` | `workspaceRegistry` 服务不可用或路径不合法 |
| `-32024` | `PERMISSIONS_UNAVAILABLE` | `permissionPresets` 服务不可用或预设无效 |
| `-32025` | `GOAL_UNAVAILABLE` | `goals` 服务未挂载 |
| `-32026` | `GOAL_INVALID` | Goal 引用过期、无当前 goal 或非法状态迁移 |
| `-32027` | `FILE_NOT_FOUND` | 会话 cwd 内目标路径不存在 |
| `-32028` | `FILE_ACCESS_DENIED` | 路径逃逸会话 cwd、符号链接逃逸或目标类型不合法 |
| `-32029` | `FILE_TOO_LARGE` | 文件超过读取上限 |
| `-32030` | `QUESTION_NOT_PENDING` | 请求重开的用户提问已结算或不存在 |
| `-32031` | `FEEDBACK_CONFLICT` | 消息反馈 `ifVersion` CAS 冲突，`data.item` 为权威当前项 |
| `-32032` | `FEEDBACK_TARGET_NOT_FOUND` | 反馈指向的 assistant 消息不存在 |
| `-32033` | `JOB_NOT_FOUND` | 指定后台 job 不存在或跨会话不可见 |
| `-32034` | `SCHEDULE_UNAVAILABLE` | 计划任务服务未在 base 组合中挂载（文档保留，未实现方法） |
| `-32035` | `ATTACHMENT_NOT_FOUND` | 指定会话附件不存在 |
| `-32036` | `ATTACHMENT_TOO_LARGE` | 附件超过 16 MiB 限制 |
| `-32037` | `ATTACHMENT_INVALID` | 附件 id、base64、文件名或文本编码非法 |
| `-32038` | `PLUGIN_OPERATION_FAILED` | 插件管理操作失败或服务不可用 |
| `-32039` | `SUBAGENTS_UNAVAILABLE` | 子代理服务/中断能力不可用 |
| `-32040` | `TEAM_UNAVAILABLE` | Agent Team 服务在当前组合中不可用 |
| `-32041` | `TERMINAL_UNAVAILABLE` | 终端服务在当前组合中不可用 |
| `-32042` | `TERMINAL_NOT_FOUND` | 终端 id 不存在或跨会话不可见 |
| `-32043` | `ACCOUNT_UNAVAILABLE` | 账号服务不可用 |
| `-32044` | `MODEL_DISCOVERY_FAILED` | 模型发现未注册、协议不支持列表或请求失败 |
| `-32045` | `SCHEDULE_OPERATION_FAILED` | 计划任务操作失败；`data.code` 保留上游 `ScheduleInputError` / `RemoteError` 字符串码，`data.details` 保留结构化载荷 |

---

## 10. 核心交互流程完整 NDJSON 示例

说明：`->` 表示客户端发往服务端的请求/响应；`<-` 表示服务端推往客户端的响应/通知/请求。每一行均为单行紧凑 NDJSON。

### 10.1 启动握手流程
```json
-> {"jsonrpc":"2.0","id":"init-1","method":"libadwaita/initialize","params":{"clientVersion":"0.1.0","protocolVersion":1,"capabilities":{"userQuestions":true,"approvalRequests":true,"streaming":true}}}
<- {"jsonrpc":"2.0","id":"init-1","result":{"protocolVersion":1,"dshVersion":"0.2.2-rc.1","profileName":"libadwaita","generationId":"gen-20261106-9921","serverInfo":{"name":"dsh-libadwaita-bridge","version":"0.1.0"}}}
```

### 10.2 会话创建与 `session/follow` 开场快照
```json
-> {"jsonrpc":"2.0","id":"c-1","method":"libadwaita/session/create","params":{"cwd":"/home/arcat/Develop/my-project"}}
<- {"jsonrpc":"2.0","id":"c-1","result":{"sessionId":"sess-88a1","header":{"version":4,"id":"sess-88a1","createdAt":1762419200000,"cwd":"/home/arcat/Develop/my-project","isSeeded":false}}}
-> {"jsonrpc":"2.0","id":"f-1","method":"libadwaita/session/follow","params":{"sessionId":"sess-88a1"}}
<- {"jsonrpc":"2.0","id":"f-1","result":{"header":{"version":4,"id":"sess-88a1","createdAt":1762419200000,"cwd":"/home/arcat/Develop/my-project","isSeeded":false},"latestSeq":-1,"status":"idle","title":null,"projections":{"planMode":false,"todos":null},"assistantBaseline":null}}
```

### 10.3 提交 Prompt、实时增量与事件提交流程
```json
-> {"jsonrpc":"2.0","id":"p-1","method":"libadwaita/session/prompt","params":{"sessionId":"sess-88a1","contentBlocks":[{"type":"text","text":"你好，请帮我检查代码"}]}}
<- {"jsonrpc":"2.0","id":"p-1","result":{"messageId":"msg-user-1"}}
<- {"jsonrpc":"2.0","method":"session.status","params":{"sessionId":"sess-88a1","status":"running"}}
<- {"jsonrpc":"2.0","method":"session.event","params":{"sessionId":"sess-88a1","event":{"type":"turn/start","seq":0,"time":1762419201000,"data":{"turn":1}}}}
<- {"jsonrpc":"2.0","method":"session.event","params":{"sessionId":"sess-88a1","event":{"type":"user/message","seq":1,"time":1762419201010,"data":{"role":"user","content":[{"type":"text","text":"你好，请帮我检查代码"}],"source":{"kind":"human"}},"surfaceOp":"append"}}}
<- {"jsonrpc":"2.0","method":"session.event","params":{"sessionId":"sess-88a1","event":{"type":"step/start","seq":2,"time":1762419201050,"data":{"turn":1,"step":0}}}}
<- {"jsonrpc":"2.0","method":"agent.stream","params":{"sessionId":"sess-88a1","frame":{"type":"start","attemptId":"att-0","revision":1,"turn":1,"step":0}}}
<- {"jsonrpc":"2.0","method":"agent.stream","params":{"sessionId":"sess-88a1","frame":{"type":"chunk","attemptId":"att-0","revision":1,"index":0,"time":1762419201200,"chunk":{"type":"text-delta","index":0,"text":"收到"}}}}
<- {"jsonrpc":"2.0","method":"agent.stream","params":{"sessionId":"sess-88a1","frame":{"type":"chunk","attemptId":"att-0","revision":1,"index":1,"time":1762419201250,"chunk":{"type":"text-delta","index":0,"text":"，我正在查看。"}}}}
<- {"jsonrpc":"2.0","method":"session.event","params":{"sessionId":"sess-88a1","event":{"type":"assistant/message","seq":3,"time":1762419201500,"data":{"turn":1,"step":0,"message":{"role":"assistant","content":[{"type":"text","text":"收到，我正在查看。"}]},"stream":[]},"surfaceOp":"append"}}}
<- {"jsonrpc":"2.0","method":"agent.stream","params":{"sessionId":"sess-88a1","frame":{"type":"end","attemptId":"att-0","revision":1,"index":2,"outcome":{"kind":"committed","eventType":"assistant/message","seq":3}}}}
<- {"jsonrpc":"2.0","method":"session.event","params":{"sessionId":"sess-88a1","event":{"type":"step/end","seq":4,"time":1762419201520,"data":{"turn":1,"step":0}}}}
<- {"jsonrpc":"2.0","method":"session.event","params":{"sessionId":"sess-88a1","event":{"type":"turn/end","seq":5,"time":1762419201530,"data":{"turn":1,"reason":{"kind":"completed"}}}}}
<- {"jsonrpc":"2.0","method":"session.status","params":{"sessionId":"sess-88a1","status":"idle"}}
```

### 10.4 工具调用与审批流（允许单次）
```json
<- {"jsonrpc":"2.0","method":"session.event","params":{"sessionId":"sess-88a1","event":{"type":"tool/call","seq":6,"time":1762419205000,"data":{"turn":2,"step":0,"callId":"call-bash-1","name":"bash","arguments":"{\"command\":\"git status\"}"}}}}
<- {"jsonrpc":"2.0","id":"srv-appr-1","method":"approval/request","params":{"sessionId":"sess-88a1","toolName":"bash","callId":"call-bash-1","reason":"检查 git 状态"}}
-> {"jsonrpc":"2.0","id":"srv-appr-1","result":{"outcome":"allowed-once"}}
<- {"jsonrpc":"2.0","method":"session.event","params":{"sessionId":"sess-88a1","event":{"type":"tool/result","seq":7,"time":1762419205800,"data":{"turn":2,"step":0,"message":{"role":"tool","content":[{"type":"text","text":"On branch master\nnothing to commit"}],"callId":"call-bash-1"},"meta":null},"surfaceOp":"append","sourceEventSeqs":[6]}}}
```

### 10.5 计划评审流程（Plan Review：“要求修改”与“同意执行”）

#### 场景 A：用户点击“要求修改”
```json
<- {"jsonrpc":"2.0","id":"srv-q-1","method":"user-question/ask","params":{"requestId":"q-req-plan","sessionId":"sess-88a1","questions":[{"id":"plan-review","question":"审查方案","detail":"### 架构草案\n先删除所有旧代码","intent":{"kind":"plan-review","approve":"同意执行","callId":"call-exit"},"options":[{"label":"同意执行"},{"label":"要求修改"}],"multiSelect":false}]}}
-> {"jsonrpc":"2.0","id":"srv-q-1","result":{"cancelled":true}}
```
（服务端保持 plan mode 处于激活状态，继续由 Agent 调整方案）。

#### 场景 B：用户点击“同意执行”
```json
-> {"jsonrpc":"2.0","id":"srv-q-1","result":{"answers":[{"id":"plan-review","selected":["同意执行"]}]}}
```
（服务端完成 `exit_plan_mode`，追加 `plan/mode: { active: false }` 事件，退出计划模式）。

### 10.6 中途取消运行中 Turn（Cancel Turn）
```json
-> {"jsonrpc":"2.0","id":"cnc-1","method":"libadwaita/session/cancel","params":{"sessionId":"sess-88a1"}}
<- {"jsonrpc":"2.0","id":"cnc-1","result":{"cancelled":true}}
<- {"jsonrpc":"2.0","method":"agent.stream","params":{"sessionId":"sess-88a1","frame":{"type":"end","attemptId":"att-2","revision":3,"index":5,"outcome":{"kind":"abandoned"}}}}
<- {"jsonrpc":"2.0","method":"session.event","params":{"sessionId":"sess-88a1","event":{"type":"turn/end","seq":10,"time":1762419210000,"data":{"turn":3,"reason":{"kind":"aborted","reason":{"kind":"user"}}}}}}
<- {"jsonrpc":"2.0","method":"session.status","params":{"sessionId":"sess-88a1","status":"idle"}}
```

### 10.7 插件启用（返回 restart-required）
```json
-> {"jsonrpc":"2.0","id":"plg-1","method":"libadwaita/plugin/enable","params":{"id":"tool-plugin-manager"}}
<- {"jsonrpc":"2.0","id":"plg-1","result":{"id":"tool-plugin-manager","enabled":true,"status":"restart-required","restartRequired":true}}
```

### 10.8 内存凭据注入流程
```json
-> {"jsonrpc":"2.0","id":"cr-1","method":"libadwaita/credentials/inject","params":{"provider":"DEEPSEEK_API_KEY","key":"sk-dsh-production-key-example"}}
<- {"jsonrpc":"2.0","id":"cr-1","result":{"provider":"DEEPSEEK_API_KEY","configured":true}}
-> {"jsonrpc":"2.0","id":"cr-2","method":"libadwaita/credentials/status","params":{}}
<- {"jsonrpc":"2.0","id":"cr-2","result":{"items":[{"provider":"DEEPSEEK_API_KEY","configured":true,"masked":"sk-...mple"}]}}
```

### 10.9 缺口修复与重连流程（Gap Repair）
```json
// GUI 收到跳号通知（已持有 seq 12，却收到 seq 15）
<- {"jsonrpc":"2.0","method":"session.event","params":{"sessionId":"sess-88a1","event":{"type":"step/start","seq":15,"time":1762419220000,"data":{"turn":4,"step":1}}}}
// GUI 暂停视图追加，立即请求补齐 [13, 14]
-> {"jsonrpc":"2.0","id":"pg-1","method":"libadwaita/session/page","params":{"sessionId":"sess-88a1","fromSeq":13,"toSeq":14}}
<- {"jsonrpc":"2.0","id":"pg-1","result":{"events":[{"type":"tool/result","seq":13,"time":1762419219000,"data":{}},{"type":"step/end","seq":14,"time":1762419219500,"data":{}}],"hasMore":false}}
// GUI 依序向前端渲染器提交 13、14，再提交 15，序号重归严密自增。
```

---

## 11. 明确的非目标（Explicit Non-Goals）

1. **不做自动化专用方法 `session/wait`**：桌面客户端由人类驱动，事件由通知推流与 GTK 响应回路消费，不需要无超时轮询阻塞型的 `session/wait`。
2. **不做通用工作目录就地修改（`workdir/set`）**：切换工作区通过创建新会话实现，保证会话与工作区目录的不可变强绑定。
3. **不做虚假的 `session/close`**：上游不存在销毁持久会话日志的 RPC，视图关闭是 GUI 本地资源回收，不虚构未证实的后端行为。
4. **不暴露明文凭据回读**：`credentials/status` 与任何其他方法绝不返回明文 secret，阻断密码泄露链条。
5. **不内联 Web Host 私有 RPC**：不模拟 Web 端构建生成的 Typert 私有 mux 通道，所有通信完全收敛在 bridge 定义的标准命名空间中。

6. **G20 Agent Preset 目录暂为 base 阻塞缺口**：上游 `AgentPresetRegistry`（`packages/preset/agent-preset-registry`）仅在 `packages/bundle/web-app` 挂载（`cordis.patch.yml` 第 598 行）；其 preset 声明（`packages/bundle/web-app/presets/standard.patch.yml`）会再次挂载 bash/fs/skill/goal 等工具插件，与 base 已全局挂载的行冲突。为遵守"不伪造、不重复挂载"原则，本协议暂不定义 `libadwaita/agent-preset/*`，GUI 的 Agent Preset 卡（映射表 S16）标记为缺失，直至上游提供 base 安全的注册表组合。

7. **G26 Schedule 已通过 bridge bundle patch 挂载**：`@deepseek-ai/dsh-schedule` 插入 `bridge/cordis.patch.yml`，真实 dsh 集成测试验证 catalog/create/history/delete 与 `schedule.changed`。上游服务本身 base 兼容，无需 webServer。

8. **G12 中 `turnOutline` / `sessionStats` 为 base 省略字段**：`packages/session/session-stats` 与 `session-turn-outline` 仅由 `packages/bundle/web-app/cordis.patch.yml` 第 98–104 行挂载；协议只允许省略这些字段，不允许桥侧合成。

9. **G04 全文搜索保持禁用**：base 的 `session-query-sqlite` 以 `openAt: never` 配置（`packages/bundle/base/cordis.patch.yml` 第 157–161 行），因此 `session/search` 只做标题与 cwd 元数据匹配，`fullText` 恒为 `false`。

10. **G28 中 `remainingMs` 不伪造**：上游 `attachWait` 是异步流（`packages/interaction/user-questions/src/index.ts`），bridge 以原请求快照支持 `question/reopen`，不承诺静态剩余毫秒值。

11. **G05 Terminal 已通过 bridge bundle patch 挂载**：插入 `@deepseek-ai/dsh-terminal` + `@deepseek-ai/dsh-terminal-bash`（backendType shell），真实 dsh 测试验证 spawn/input/output/close；上游无独立 resize RPC，恢复通过 list+output。

12. **G07 Office→PDF 为 base 阻塞缺口**：`ctx.officeToPdf` 属于 `packages/bundle/web-app` 的文档预览组合（`packages/api/remotes` 的 officeToPdf Remote、web-app e2e 测试）；base 不提供该服务。GUI 回退为系统默认应用打开。

13. **G09 Agent Team 服务在 base-only 环境中未激活**：base patch 第 366–367 行虽然插入 `agent-team` 行，但本机集成探针在启动后 `ctx.get('agentTeams')` 及 loose 读取均为 `undefined`，`agent-team/*` 返回 `-32040`。协议不伪造 Team 状态。

14. **G17 模型发现/Provider 目录的 base 限制**：base 只有 `llm-pi-ai` 注册 `discoverModels`，且具体协议需该构建支持模型列表；不支持时返回 `-32044`。`web-search/providers` 返回 `ctx.web.listProviders()`，本机 base-only 组合中列表为空；协议不合成 provider。

15. **G18 浏览器登录不在协议内**：`ctx.deepseekAccount` 的 OAuth 回调依赖 Host webServer（`packages/credentials/deepseek-account-platform`），base-only 组合无 webServer；本协议只提供状态/资料/余额/退出。

16. **G27 Cordis define 实例控制为 base 阻塞缺口**：`cordis_define` 动态实例控制属于 `packages/extensions/ui-cordis`/web-app 层，base 不挂载对应 Host runner；协议不定义 `cordis-control/*`。

---

## 12. 上游源码引用与映射依据

为确保协议的可溯源性与长期六个月可维护性，本文档定义的全部词汇、状态机与数据结构直接对齐上游以下源文件：

| 领域模块 | 上游文件路径（在 `@deepseek-ai/dsh` 0.2.2-rc.1 中） | 引用与映射词汇 |
|---|---|---|
| **会话事件模型** | `packages/core/session/src/types.ts` | `SessionEvent`, `SessionEventType`, `SessionHeader`, `SurfaceOp`, `TurnEndReason` |
| **已知事件字典** | `packages/core/session/src/known-event-types.ts` | `KNOWN_SESSION_EVENT_TYPES` 全集 |
| **SDK 协议契约** | `packages/sdk/protocol/src/types.ts` | `InitializeParams`, `ApprovalRequestParams`, `ApprovalRequestResult`, `SdkUserQuestionRequest`, `SdkPromptContentBlock` |
| **流式帧与状态** | `packages/core/agent/src/runtime-types.ts` | `AssistantStreamFrame`（`start` / `chunk` / `end`）, `StreamChunk` |
| **LLM 增量块** | `packages/llm/llm/src/types.ts` | `StreamChunk`（`text-delta`, `reasoning-delta`, `tool-call-delta`） |
| **提问与计划评审** | `packages/interaction/user-questions/src/types.ts` | `AskUserQuestionItem`, `AskUserQuestionIntent`, `plan-review` 意图与选项 |
| **计划模式状态** | `packages/plan/plan-mode/src/index.ts` | `plan/mode`, `EXIT_PLAN_MODE`, `ASK_CANCELLED` 修改语义 |
| **会话标题管理** | `packages/session/session-title/src/types.ts` | `SessionTitleSnapshot`, `SessionTitleEventData`, `SessionTitleSource` |
| **Todo 列表投影** | `packages/todo/tool-todo/src/types.ts` | `TodoItem`（`pending` \| `in_progress` \| `completed`）, `todo/write` |
| **工作区变更与对比** | `packages/deliverables/workspace-changes/src/types.ts`| `WorkspaceChangesSummary`, `WorkspaceChangedFile`, `WorkspaceFileDiff`, `WorkspaceDiffHunk` |
| **工作目录绑定** | `packages/session/working-directory/src/index.ts` | `WorkingDirectoryService.get`, `working-directory/change` |
| **客户端流组装** | `packages/client/ui-conversation/src/client/conversation/assembly.ts` | 连续 `seq` 校验、`assistantBaseline` 替换规则与装配策略 |
| **会话传输控制器** | `packages/api/session-controller/src/client/transport.ts` | `SessionJournalPage`, `assistantStream`, `session/follow` 开场快照范型 |
| **凭据提供方接口** | `packages/credentials/credentials/src/types.ts` | `CredentialProvider`, `CredentialDescriptor` |
| **会话重命名** | `packages/session/session-title/src/index.ts` | `SessionTitleService.rename`, `SessionTitleInvalidError` |
| **会话分支** | `packages/core/session/src/fork.ts`; `packages/api/session-controller/src/commands.ts` | `buildForkSeed`, `latestCompletedPrefixBoundary`, `incrementedTitle` |
| **投影基线与增量** | `packages/session/session-projection/src/index.ts`; `packages/llm/token-meter/src/projection.ts` | `SessionProjectionRegistry.snapshot/onEventChanged`, `tokenUsage`, `contextPressure`, `contextBreakdown` |
| **工具结果检查** | `packages/client/ui-trajectory/src/index.ts` | `sessionToolResults`（web-app only；bridge 派生自 `tool/result` 事件） |
| **反馈服务** | `packages/feedback/message-feedback/src/index.ts`; `packages/feedback/command-feedback/src/index.ts` | `MessageFeedbackService.list/put/delete`, `SessionFeedbackService.record`, `ifVersion` CAS |
| **后台 Jobs** | `packages/jobs/jobs/src/index.ts`; `packages/jobs/jobs/src/view.ts` | `JobRegistry.list/get/readAt/kill`, `JobView`, `JobChunk` |
| **计划任务** | `packages/schedule/schedule/src/index.ts` | `ScheduleService`（web-app only，base 阻塞） |
| **附件暂存与图片** | `packages/attachment/attachment/src/index.ts` | `AttachmentStore.saveImages/admitPromptContent`（bridge 附件上传为本地实现） |
| **Inbox 队列** | `packages/core/agent/src/runtime-types.ts`; `packages/api/session-controller/src/commands.ts` | `Inbox.nextTurn/nextStep/replace/remove`, `agent.steer`, `updateQueue` |
| **Slash 命令** | `packages/interaction/commands/src/index.ts`; `packages/interaction/commands/src/types.ts` | `CommandsService.list/execute`, `CommandDescriptor`, `CommandExecution` |
| **Skill 目录** | `packages/skill/skill/src/index.ts`; `packages/skill/skill-filesystem/src/index.ts` | `skills.snapshot/list`, `SkillSummary`, `SkillInvocationPolicy` |
| **文件引用候选** | `packages/context/file-reference/src/index.ts`; `packages/context/file-reference-local/src/index.ts` | `FileReferenceService.list`, `FileReferenceCandidate` |
| **工作区持久化** | `packages/workspace/workspace/src/index.ts`; `packages/api/workspace-controller/src/commands.ts` | `WorkspaceRegistry.create/list/delete/resolveByPath`, `Workspace.setTitle/status` |
| **设置表单** | `packages/settings/settings/src/index.ts` | `SettingsForms.describe/update`, `SettingsDescriptor`, `SettingsConflictError` |
| **权限预设** | `packages/interaction/permission-presets/src/index.ts`; `packages/interaction/permission-presets/src/types.ts` | `PermissionPresetService.catalog/current/set`, `PermissionCatalog`, `PresetOption` |
| **Goal 状态机** | `packages/goal/goal/src/index.ts`; `packages/goal/goal/src/types.ts` | `GoalService.get/edit/pause/resume/clear`, `GoalView`, `GoalRef` |
