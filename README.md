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
```

bridge 的测试将在阶段 2 按 TDD 引入；当前不提供 `test` 脚本。
