# 上游 `@deepseek-ai/dsh` 升级流程

本文记录 bridge 对上游 dsh 版本的锁定、升级步骤，以及在当前锁定版本上的实跑验证结果。

## 1. 当前锁定状态

| 位置 | 值 |
| --- | --- |
| `bridge/package.json` `engines.dsh` | `0.2.2-rc.1` |
| `bridge/package.json` `dependencies`（`@deepseek-ai/dsh` 及各 `dsh-*` 运行时包） | 全部精确固定为 `0.2.2-rc.1` |
| `bridge/pnpm-lock.yaml` | 由上述清单生成 |
| `README.md` / `docs/bridge-protocol.md` | `0.2.2-rc.1` |
| `app/src/supervisor.rs` 中生成的 profile | `@deepseek-ai/dsh-base: 0.2.2-rc.1` |

查询上游最新版本：

```sh
npm view @deepseek-ai/dsh version
npm view @deepseek-ai/dsh dist-tags --json
```

2026-11-06 实测：

```text
0.2.2-rc.1
{ "latest": "0.2.2-rc.1", "alpha": "0.2.2-alpha.3", "next": "0.2.2-rc.1" }
```

即当前没有比锁定版本更新的发布，本轮不执行版本 bump；下节流程已按锁定版本实跑验证。

## 2. 升级步骤

1. **确认上游版本**：执行上节的 npm 查询；只有 `latest`（或明确选定的 dist-tag）与当前锁不一致时才升级。
2. **更新声明**：
   - `bridge/package.json`：`engines.dsh`、`dependencies` 中 `@deepseek-ai/dsh` 与所有 `@deepseek-ai/dsh-*` 同步到同一精确版本；
   - `README.md` 与 `docs/bridge-protocol.md` 的锁定版本说明；
   - `app/src/supervisor.rs` 生成的 profile 中 `@deepseek-ai/dsh-base` 版本（若上游版本随之变化）。
3. **重装依赖并刷新锁文件**：

   ```sh
   pnpm -C bridge install --lockfile-only
   pnpm -C bridge install
   ```

4. **比对 `--dump-config` 行 id**（关键步骤）：在隔离的 `DSH_HOME` 中生成受管 profile，运行：

   ```sh
   DSH_HOME=<tmp>/dsh-home \
   XDG_DATA_HOME=<tmp>/data \
   XDG_RUNTIME_DIR=<tmp>/runtime \
   bridge/node_modules/.bin/dsh --profile libadwaita --dump-config
   ```

   确认三件事：
   - `hmr` 行仍为 `disabled: true`（模块 `@deepseek-ai/dsh-hmr`）；
   - `credentials` 行仍为 `disabled: true`（模块 `@deepseek-ai/dsh-credentials-local`）；
   - `llm-deepseek` 行仍存在（模块 `@deepseek-ai/dsh-llm-deepseek-api-key`），供 `baseURL` 覆盖使用。

   若上游重命名了这些行，必须同步更新 `app/src/supervisor.rs` 的 `generate_cordis_patch_yml`、`docs/bridge-protocol.md` 与设计文档中的行 id。
5. **运行完整测试**：

   ```sh
   pnpm -C bridge typecheck
   pnpm -C bridge build
   pnpm -C bridge test
   pnpm -C bridge lint
   cargo test --manifest-path app/Cargo.toml
   bash scripts/ci.sh
   ```

6. **提交**：`⬆️ chore(deps): bump @deepseek-ai/dsh to <version>`，只暂存版本、锁文件与受影响文档。若因上游破坏性变更导致测试失败，不得强行 bump；撤回临时改动、保持仓库锁定版本，并记录确切断点与下一步。

## 3. 在锁定版本 `0.2.2-rc.1` 上的实跑记录（2026-11-06）

工作目录：仓库根目录；隔离 profile：`/tmp/dsh-upgrade-check/dsh-home`。

实际执行的命令与结果：

```sh
# 1. 版本查询
npm view @deepseek-ai/dsh version        # 0.2.2-rc.1
npm view @deepseek-ai/dsh dist-tags --json
# {"latest":"0.2.2-rc.1","alpha":"0.2.2-alpha.3","next":"0.2.2-rc.1"}
# -> 与锁定版本一致，无需 bump

# 2. 行 id / patch 组合比对
bridge/node_modules/.bin/dsh --profile libadwaita --dump-config
# exit=0，stderr 为空，输出 386 行
```

`--dump-config` 观察到的关键行（行号取自本次输出）：

```text
# == @deepseek-ai/dsh-base, patched by <profile>/cordis.patch.yml
- id: hmr
  name: '@deepseek-ai/dsh-hmr'
  disabled: true
  config:
    root: []
...
# == @deepseek-ai/dsh-base, patched by <profile>/cordis.patch.yml
- id: credentials
  name: '@deepseek-ai/dsh-credentials-local'
  disabled: true
...
- id: llm-deepseek
  name: '@deepseek-ai/dsh-llm-deepseek-api-key'
# == @dsh-libadwaita/bridge
- id: tool-ask-user
- id: file-reference-local
- id: workspace
- id: message-feedback
- id: bridge-credentials
- id: bridge
```

结论：行 id `hmr`、`credentials`、`llm-deepseek` 与设计文档记录一致，未被上游重命名；profile patch 正确覆盖 `hmr` 与 `credentials`；bridge bundle 的六行 patch 正常追加在 `dsh-base` 之后。

测试结果：

| 命令 | 结果 |
| --- | --- |
| `pnpm -C bridge typecheck` | PASS |
| `pnpm -C bridge build` | PASS |
| `pnpm -C bridge test` | 37 tests，37 pass，0 fail |
| `pnpm -C bridge lint` | PASS |
| `cargo test --manifest-path app/Cargo.toml` | 61 pass，0 fail |
| `bash scripts/ci.sh` | 全部 7 项检查通过 |

因此升级流程在当前锁定版本上可执行、可复现；后续上游发版时按第 2 节执行即可，重点检查 `--dump-config` 的三个行 id 与 bridge patch 追加位置。
