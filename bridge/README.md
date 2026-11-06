# @dsh-libadwaita/bridge

Linux GTK4/libadwaita 原生客户端专用的 dsh bridge bundle。

锁定上游版本：`@deepseek-ai/dsh` `0.2.2-rc.1`  
传输：Unix domain socket (0600)，NDJSON JSON-RPC 2.0 (JsonRpcLineTransport)

---

## 1. 首启 Profile 生成指南（Rust Supervisor 实施规范）

GUI 在首次启动时，若 `$XDG_DATA_HOME/dsh-libadwaita/profiles/libadwaita` 不存在，必须按本规范在用户空间自动生成该受管 profile。

### 1.1 目录布局
```text
$XDG_DATA_HOME/dsh-libadwaita/
└── profiles/
    └── libadwaita/
        ├── package.json
        ├── cordis.patch.yml
        └── node_modules/
            └── @dsh-libadwaita/
                └── bridge -> /usr/lib/dsh-libadwaita/bridge (符号链接指向系统安装位置)
```

### 1.2 `package.json` 精确内容

在 `$XDG_DATA_HOME/dsh-libadwaita/profiles/libadwaita/package.json` 中写入：

```json
{
  "name": "libadwaita-profile",
  "version": "0.1.0",
  "private": true,
  "dsh": {
    "profile": {
      "bundles": [
        "@deepseek-ai/dsh-base",
        "@dsh-libadwaita/bridge"
      ]
    }
  },
  "dependencies": {
    "@deepseek-ai/dsh-base": "0.2.2-rc.1",
    "@dsh-libadwaita/bridge": "file:/usr/lib/dsh-libadwaita/bridge"
  }
}
```

> **注意**：开发环境下，`@dsh-libadwaita/bridge` 的路径替换为实际的本地代码目录绝对路径。在生成 profile 时，必须在 `node_modules/@dsh-libadwaita/bridge` 创建指向 bridge bundle 目录的软链接，以便 Node 模块解析器与 dsh 的 `createRequire(anchor).resolve.paths(...)` 能够双锚点正确定位。

### 1.3 `cordis.patch.yml` 精确内容（生产配置）

在 `$XDG_DATA_HOME/dsh-libadwaita/profiles/libadwaita/cordis.patch.yml` 中写入：

```yaml
- id: hmr
  name: '@deepseek-ai/dsh-hmr'
  disabled: true
  config:
    root: []
- id: credentials
  name: '@deepseek-ai/dsh-credentials-local'
  disabled: true
```

#### 关键行 ID 与覆盖语义
- **`id: hmr`**（模块名 `'@deepseek-ai/dsh-hmr'`）：在生产配置中显式禁用 config-only HMR，保证运行期长连接观察到的服务拓扑稳定，不被动态重载中断。
- **`id: credentials`**（模块名 `'@deepseek-ai/dsh-credentials-local'`）：必须禁用。上游该行会在本地磁盘 `$DSH_HOME` 写入明文 `.credentials.yaml` 文件；将其禁用后，bridge bundle 挂载的内存凭据提供方接管 `ctx.credentials`，完全杜绝磁盘秘钥泄露。
- **bridge bundle 自身 patch**：`bridge/cordis.patch.yml` 在 `@deepseek-ai/dsh-base` 之上插入 `tool-ask-user`、`file-reference-local`、`bridge-credentials` 与 `bridge` 四行。GUI 无需在 profile 用户层重复声明这些行。

### 1.4 测试 Profile 补丁（仅供测试，禁止进生产）

测试环境下通过 mock Anthropic SSE 服务器代理 LLM 请求时，需在 `cordis.patch.yml` 追加以下配置：

```yaml
- id: llm-deepseek
  name: '@deepseek-ai/dsh-llm-deepseek-api-key'
  config:
    baseURL: http://127.0.0.1:<MOCK_PORT>/anthropic
```

生产环境下绝对不能包含 `baseURL` 覆盖，保持上游默认的官方服务网关。

---

## 2. 进程启动与通信参数

GUI 启动 dsh 进程时的命令行与环境变量：

```sh
dsh --profile libadwaita
```

- **环境变量**：
  - `DSH_HOME`: 设置为 `$XDG_DATA_HOME/dsh-libadwaita`
  - `XDG_RUNTIME_DIR`: 系统 `$XDG_RUNTIME_DIR`
  - `DSH_LIBADWAITA_SOCKET`: 目标 Unix Socket 路径（位于 `$XDG_RUNTIME_DIR/dsh-libadwaita/dsh-<pid>-<random>.sock`）
- **标准输入**：经由 `/dev/null` 重定向（输入 EOF 不会导致进程退出）。
- **标准输出**：严格保持 0 字节输出；所有日志走 stderr。
- **套接字权限**：服务端在监听时通过 `chmodSync(socketPath, 0o600)` 强制限制仅当前用户可访问。

---

## 3. 凭据注入流程

1. dsh 启动并完成 `libadwaita/initialize` 握手。
2. GUI 从系统 Secret Service (`libsecret`) 读取存储的 API Key。
3. GUI 调用 `libadwaita/credentials/inject` 将 Key 注入 dsh 内存。
4. 调用 `libadwaita/credentials/status` 确认 `configured: true`。

---

## 4. 构建与测试

```sh
pnpm typecheck
pnpm build
pnpm test
pnpm lint
```
