# dsh-libadwaita

`dsh-libadwaita` 是 [dsh（deepseek-harness）](https://github.com/deepseek-ai/deepseek-harness) 的下游 Linux GTK4/libadwaita 客户端。上游负责 agent 内核、插件体系、模型适配、会话持久化与工具执行；本项目通过原生 GUI 和薄 bridge 提供 Linux 桌面体验，不重写或复制上游业务逻辑。

上游锁定目标：`@deepseek-ai/dsh` `0.2.2-rc.1`（2026-11-05 的 npm `latest` 版本）。

## 非目标

不做 macOS/Windows、不 fork 上游、不搬 React/TypeScript 客户端代码、不嵌 WebView、不做 Flatpak 沙箱、远程访问、多用户或账号体系。

## 仓库布局

- `app/`：Rust crate，包含 GUI、领域对象、bridge 客户端与集成测试。
- `bridge/`：TypeScript 包，包含 Cordis 插件与 dsh bundle。
- `docs/`：设计文档、实施计划、bridge 协议与 Web UI 映射表。
- `packaging/`：Arch PKGBUILD、桌面文件、图标与 D-Bus 服务文件。

## 构建决策

- Rust 应用使用纯 cargo，不引入 Meson；UI 控件暂由 Rust 代码构建，不使用 Blueprint。
- bridge 使用 TypeScript 与 `tsc` 构建，包管理器为 pnpm。

## 前置条件

- Rust 与 cargo，以及 GTK4、libadwaita 的系统开发文件。
- Node.js >= 24.9。
- pnpm 11。
本机实测环境：Node 26.13.0、pnpm 12.17.0、rustc/cargo 1.93.1 stable（设计基线写的是 nightly，实际为 stable）、gtk4-rs 0.10.3 / libadwaita-rs 0.8.1。

## 构建与测试入口

在仓库根目录执行：

```sh
cargo check --manifest-path app/Cargo.toml
cargo build --manifest-path app/Cargo.toml
cargo test --manifest-path app/Cargo.toml
```

```sh
pnpm -C bridge install
pnpm -C bridge typecheck
pnpm -C bridge build
pnpm -C bridge test
pnpm -C bridge lint
```

## 启动与运行

### 开发模式

在仓库根目录先构建 bridge 与应用：

```sh
pnpm -C bridge install
pnpm -C bridge build
cargo build --manifest-path app/Cargo.toml
```

随后启动 GUI（本机 `WAYLAND_DISPLAY=wayland-0`；`DSH_BIN` 使用仓库内绝对路径）：

```sh
DSH_BIN="$PWD/bridge/node_modules/.bin/dsh" cargo run --manifest-path app/Cargo.toml
```

也可以把仓库内 `dsh` 放到 `PATH` 前缀，不设置 `DSH_BIN`（本机实测可行）：

```sh
PATH="$PWD/bridge/node_modules/.bin:$PATH" cargo run --manifest-path app/Cargo.toml
```

本机以 `mkdtemp` 创建的隔离 XDG 目录连续运行 25 秒：app 与 `dsh --profile libadwaita` 子进程存活，`$XDG_RUNTIME_DIR/dsh-libadwaita/*.sock` 权限 `0600`，stdout 为 0 字节，stderr 为空，SIGTERM 后退出码为 0 且无残留子进程。`dsh` 必须可达；否则 `HarnessSupervisor::start()` 无法 spawn，启动页显示 `dsh failed to start` 与 Retry banner。

### 安装后

`packaging/PKGBUILD` 安装 `/usr/bin/dsh-libadwaita`、`/usr/lib/dsh-libadwaita/bridge/`、`/usr/share/applications/io.github.deepseek_ai.DshLibadwaita.desktop`、`/usr/share/icons/hicolor/scalable/apps/io.github.deepseek_ai.DshLibadwaita.svg` 和 `/usr/share/dbus-1/services/io.github.deepseek_ai.DshLibadwaita.service`。安装后有三种启动方式：

- 命令：`dsh-libadwaita`（桌面文件 `Exec=dsh-libadwaita`）。
- 桌面图标 `dsh` 或 `gtk-launch io.github.deepseek_ai.DshLibadwaita`。
- D-Bus 单实例激活（服务名 `io.github.deepseek_ai.DshLibadwaita`，服务文件 `Exec=/usr/bin/dsh-libadwaita --gapplication-service`）：

```sh
gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.StartServiceByName io.github.deepseek_ai.DshLibadwaita 0
```

安装包把 `dsh` 放到 `/usr/lib/dsh-libadwaita/bridge/node_modules/.bin/dsh`，但不安装 `/usr/bin/dsh`。未设置 `DSH_BIN` 时，supervisor 先按 `PATH` 查找可执行的 `dsh`，找不到再自动使用该安装路径；因此桌面图标、`gtk-launch` 与 D-Bus 激活无需手动 export。`DSH_BIN` 仍是显式覆盖；只有 PATH 和系统 bundle 都不可用时才回退到 `dsh` 并显示 Retry banner。

本机未执行 `pacman -U` 或系统安装；`desktop-file-validate packaging/io.github.deepseek_ai.DshLibadwaita.desktop` 退出码为 0（仅提示 `Categories` 含多个主类）。`gtk-launch` 与上面的 `gdbus ... StartServiceByName` 命令使用临时 XDG 数据目录、私有 D-Bus 会话和指向开发二进制的临时 service 文件验证过，二者都在总线上注册了 `io.github.deepseek_ai.DshLibadwaita`。

### 环境变量

| 变量 | 作用 |
|---|---|
| `DSH_BIN` | 显式覆盖 dsh 可执行文件。未设置时依次探测：PATH 中的可执行 `dsh` → `/usr/lib/dsh-libadwaita/bridge/node_modules/.bin/dsh` → 字符串 `dsh`（失败时显示 Retry）。以 `.js` 结尾时改为 `node <file>`。 |
| `DSH_BRIDGE_BUNDLE` | 覆盖受管 profile 的 bridge bundle 路径；未设置时优先使用存在的 `/usr/lib/dsh-libadwaita/bridge`，否则使用编译时的 `../bridge`。 |
| `DSH_HOME` | 不是 GUI 的用户覆盖项；supervisor 把 `$XDG_DATA_HOME/dsh-libadwaita`（未设置 `XDG_DATA_HOME` 时为 `$HOME/.local/share/dsh-libadwaita`）作为 `DSH_HOME` 注入 dsh 子进程，与 `~/.dsh` 隔离。 |
| `XDG_RUNTIME_DIR` | supervisor 在其下创建 `dsh-libadwaita/`（`0700`）和 `dsh-<app-pid>-<ns>.sock`（`0600`），并把该子目录作为 dsh 子进程的 `XDG_RUNTIME_DIR`。 |

### 首启行为

无 API key 不阻塞 dsh 启动与 `libadwaita/initialize` 握手；`libadwaita/credentials/status` 报告未配置时，GUI 才显示首启向导。保存时 `CredentialStore::set_key` 写入 Secret Service（同时保留内存副本），再通过 `libadwaita/credentials/inject` 注入 dsh 内存；选择 `Configure Later` 只关闭向导，不中断已建立的 bridge 连接。

### 故障排查

- 缺 dsh：检查 `DSH_BIN` 或 `PATH`；开发模式确认 `pnpm -C bridge install && pnpm -C bridge build` 已生成 `bridge/node_modules/.bin/dsh`；安装模式确认 `/usr/lib/dsh-libadwaita/bridge/node_modules/.bin/dsh` 存在，或显式设置 `DSH_BIN`。
- 无头/私有 D-Bus 会话：没有可激活的 `org.a11y.atspi.Registry` 时，GTK 输出 `Gtk-CRITICAL **: ... Unable to register the application: ... Could not activate remote peer 'org.a11y.atspi.Registry': unit failed`；这类会话用 `GTK_A11Y=none NO_AT_BRIDGE=1` 启动，普通桌面会话不受影响。
- 版本不匹配：`libadwaita/initialize` 要求 `PROTOCOL_VERSION = 1`，不一致时 supervisor 停止 dsh 并显示 `Protocol version mismatch` 与 Retry；确认 `@deepseek-ai/dsh` `0.2.2-rc.1` 与 `bridge/package.json` 的 `engines.dsh` 一致，并重建 bridge。
- profile 首启生成：`ensure_managed_profile` 在 `$DSH_HOME/profiles/libadwaita/` 写入 `package.json`、`cordis.patch.yml` 与 bridge 符号链接，patch 禁用 `hmr` 和本地凭据插件；不要到 `~/.dsh` 找配置，排查时改用独立的 `XDG_DATA_HOME`。
- 关闭窗口后台继续：有运行中的 session 时，关闭请求只隐藏窗口，并用 `ApplicationHoldGuard` 与后台通知维持进程；点击通知的 `app.activate` 可重新呈现窗口。
- 日志只在 stderr 且 stdout 保持纯净：supervisor 把 dsh 的 stdout 管道读取并计入 `stdout_purity_violations`，stderr 继承到 GUI 的 stderr；诊断看 stderr，stdout 应保持为空，API key 不进入命令行、日志或 stdout。
- 第二实例由 GApplication 单实例处理：`APP_ID = io.github.deepseek_ai.DshLibadwaita`，第二次 `activate` 只把已有窗口设为可见并 `present()`，不会重复调用 `bootstrap_supervisor()` 或创建第二个 dsh 子进程与 socket。

## 统一 CI 入口

以上检查由 `scripts/ci.sh` 统一执行：

```sh
pnpm -C bridge install
bash scripts/ci.sh
```

脚本依次执行 bridge 的 typecheck/build/test/lint、`cargo check`/`cargo test`（`app/Cargo.toml`）以及 `desktop-file-validate` 校验 `packaging/io.github.deepseek_ai.DshLibadwaita.desktop`；全部退出码为 0 时脚本成功，否则汇总失败的检查并返回非零。
