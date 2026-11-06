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
| `libadwaita/session/list` | 列出已存储的历史会话列表 | `ctx.sessions.list` |
| `libadwaita/session/follow` | 追踪会话并获取开场快照 | `ctx.sessions` + `ctx.agents` |
| `libadwaita/session/page` | 分页拉取历史事件 / 缺口修复 | `ctx.sessions.get(id).slice` |
| `libadwaita/session/prompt` | 向会话投递新的用户提示词 | `agent.followup` / `agent.steer` |
| `libadwaita/session/queue/list` | 读取当前 inbox 队列快照 | `agent.inbox.nextTurn/nextStep` |
| `libadwaita/session/queue/edit` | 就地替换待发送队列项的文本 | `agent.inbox.replace` |
| `libadwaita/session/queue/remove` | 移除一个待发送队列项 | `agent.inbox.remove` |
| `libadwaita/session/queue/steer` | 将排队项转为当前 turn 的 steer | `agent.inbox.remove` + `agent.steer` |
| `libadwaita/session/cancel` | 停止当前正在运行的 turn | `agent.cancel({ kind: 'user' })` |
| `libadwaita/session/release` | GUI 本地关闭视图，释放追踪 | bridge 内存释放 |
| `libadwaita/session/archive` | 归档会话（清理队列，标记归档） | `agent.cancel` + 归档标记 |
| `libadwaita/session/rename` | 手动重命名会话并 pin 用户标题 | `ctx.sessionTitle.rename(session, title)` |
| `libadwaita/session/fork` | 从指定事件边界分支新会话 | `buildForkSeed` + `ctx.agents.create` |
| `libadwaita/session/workdir/get` | 读取会话当前生效的工作目录 | `ctx.workingDirectory.get` |
| `libadwaita/session/model/get` | 读取会话当前使用的模型路由 | `agent.model` / `request/context` |
| `libadwaita/session/model/set` | 切换会话模型路由配置 | `agent.model.select` |
| `libadwaita/model/catalog` | 查询可用模型与 Provider 目录 | `ctx.llm.catalog` / Provider 列表 |
| `libadwaita/settings/get` | 读取全局设置项配置表 | `ctx.settings.get` |
| `libadwaita/settings/mutate` | 原子修改配置表项 | `ctx.settings.mutate` |
| `libadwaita/plugin/list` | 查询已安装与已启用的插件 | `ctx.pluginManager.list` |
| `libadwaita/plugin/enable` | 启用插件（返回 restart-required） | `ctx.pluginManager.setPluginEnabled` |
| `libadwaita/plugin/disable` | 禁用插件（返回 restart-required） | `ctx.pluginManager.setPluginEnabled` |
| `libadwaita/credentials/status` | 查询脱敏凭据状态（无明文） | `ctx.credentials.describe` |
| `libadwaita/credentials/inject` | 向内存 Provider 注入新凭据 | 自定义内存 Provider（Spike 已验证） |
| `libadwaita/credentials/rotate` | 轮换内存中现有凭据 | 自定义内存 Provider（Spike 已验证） |
| `libadwaita/credentials/remove` | 从内存中移除指定 Provider 凭据 | 自定义内存 Provider（Spike 已验证） |
| `libadwaita/workspace/changes` | 查询工作区变更文件摘要与 diff 块 | `@deepseek-ai/dsh-workspace-changes` |
| `libadwaita/command/list` | 列出当前会话可用的 slash 命令 | `ctx.commands.list(agent)` |
| `libadwaita/command/execute` | 执行 slash 命令（如 `/plan off`） | `ctx.commands.execute` |
| `libadwaita/skill/list` | 列出当前工作区可用的 skill 目录 | `ctx.skills.snapshot` |
| `libadwaita/reference/list` | 列出 `@` 文件引用候选 | `ctx.fileReferences.list` |

---

## 6. 方法详细定义与载荷契约

### 6.1 会话创建与管理

#### `libadwaita/session/create`
- **说明**：新建一个会话并初始化 Agent。
- **重要实现细节**：根据阶段 1 Spike 结论，调用 `ctx.agents.create({ sessionId, cwd })` 内部会自动创建底层 session；**严禁**先调用 `ctx.sessions.create`，否则报 `session already exists`。
- **请求参数**：
  | 字段 | 类型 | 必选 | 说明 |
  |---|---|---|---|
  | `sessionId` | `string` | 否 | 客户端指定的 UUID。若缺省由 bridge 自动生成并返回 |
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
- **说明**：列出本地磁盘存储的所有历史会话元数据（冷列表，不拉起 Agent 实例）。
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
- **说明**：归档指定会话。若该会话正在运行则先安全停止，并丢弃 pending inbox，标记为归档状态。磁盘日志保留，不会物理删除。
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
- **说明**：获取当前 profile 下的完整设置文档。
- **请求参数**：`{ "keys"?: string[] }`（可选过滤指定 key 前缀）
- **响应结果**：`{ "settings": Record<string, unknown> }`

#### `libadwaita/settings/mutate`
- **说明**：原子更新设置。底层委托给 `ctx.settings.mutate`。
- **请求参数**：`{ "patch": Record<string, unknown> }`
- **响应结果**：`{ "settings": Record<string, unknown> }`

### 6.6 插件管理（HMR 禁用场景与 `restart-required`）

- **重要前置约束**：在我们的 profile 中，配置明确禁用了 HMR（`- id: hmr, disabled: true`）。因此，所有动态启用或禁用插件/Bundle 的操作，**均无法在当前进程中热重载生效**。配置变更将持久化保存到 profile 配置中，状态报告为 `restart-required`。
- **GUI 交互要求**：GUI 收到 `restartRequired: true` 后，必须在界面弹出 AdwBanner 或通知：“插件配置已更改，需要重启 dsh 才能生效”，并提供“立即重启”动作。

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

#### `libadwaita/plugin/enable`
- **请求参数**：`{ "id": string }`
- **响应结果**：
  ```json
  {
    "id": "tool-plugin-manager",
    "enabled": true,
    "status": "restart-required",
    "restartRequired": true
  }
  ```

#### `libadwaita/plugin/disable`
- **请求参数**：`{ "id": string }`
- **响应结果**：
  ```json
  {
    "id": "session-title-llm",
    "enabled": false,
    "status": "restart-required",
    "restartRequired": true
  }
  ```

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
---

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

---

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
| **Inbox 队列** | `packages/core/agent/src/runtime-types.ts`; `packages/api/session-controller/src/commands.ts` | `Inbox.nextTurn/nextStep/replace/remove`, `agent.steer`, `updateQueue` |
| **Slash 命令** | `packages/interaction/commands/src/index.ts`; `packages/interaction/commands/src/types.ts` | `CommandsService.list/execute`, `CommandDescriptor`, `CommandExecution` |
| **Skill 目录** | `packages/skill/skill/src/index.ts`; `packages/skill/skill-filesystem/src/index.ts` | `skills.snapshot/list`, `SkillSummary`, `SkillInvocationPolicy` |
| **文件引用候选** | `packages/context/file-reference/src/index.ts`; `packages/context/file-reference-local/src/index.ts` | `FileReferenceService.list`, `FileReferenceCandidate` |
