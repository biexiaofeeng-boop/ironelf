# 05-Rename-Inventory-S10.B-Chimera-Iceclaw-v1-2026-04-07

## 目标

为 `ironelf / ironclaw -> chimera-iceclaw` 建立第一版 rename inventory 与 naming matrix，约束本轮只做低风险 base-alignment，不做全仓硬 rename。

## 1. 当前基线判断

当前工作区与联调基线：

- 开发目录仍为 `/Users/sourcefire/X-lab/ironelf`
- 当前 git remote 仍为 `origin = git@github.com:biexiaofeeng-boop/ironelf.git`
- 新 canonical 远程目标为 `https://github.com/biexiaofeeng-boop/chimera-iceclaw`
- 当前 `chimera-core <-> ironelf` 联调主线必须保持不破

## 2. Rename Inventory

### 2.1 高密度热点分类

| 分类 | 典型位置 | 当前状态 | 风险判断 |
|---|---|---|---|
| Cargo package / workspace crate | `Cargo.toml`, `crates/ironclaw_common`, `crates/ironclaw_safety` | 包名、crate 名、workspace member 仍为 `ironclaw*` | 高风险，本轮后移 |
| 二进制 / CLI 命令 | `src/cli/mod.rs`, `src/main.rs`, `src/worker/*` | 可执行名与帮助文本仍为 `ironclaw` | 高风险，本轮保留兼容 |
| 默认配置目录 / 本地状态 | `~/.ironclaw`, `src/bootstrap`, `src/settings.rs`, `src/setup/*` | 配置、DB、日志、skills、channels 全部挂在 `.ironclaw` | 高风险，本轮后移 |
| Runtime / sandbox 标识 | `ironclaw-worker`, `ironclaw.job_id`, Docker label / container name | 运行时标签和镜像名仍是 `ironclaw*` | 中高风险，本轮只做 inventory，不改运行面 |
| Deploy / service / env | `deploy/*.service`, `deploy/setup.sh`, `deploy/restart.sh`, `deploy/macos-service.sh`, `deploy/env.example` | 适合做第一波 canonical 名称前移 | 低到中风险，本轮优先 |
| README / docs / metadata | `README*`, `docs/Issue-Checks/2026-04/*`, `Cargo.toml` homepage/repository | 对外命名仍以 `IronClaw` 为主 | 低风险，本轮优先 |
| CI / release / tests | `.github/workflows/*`, `tests/e2e/*` | 存在大量 `ironclaw` hardcode | 中高风险，本轮后移 |

### 2.2 粗粒度分布

基于本轮 inventory 搜索：

- `src/` 内存在约 `154` 个命名热点文件
- `tests/` 内存在约 `90` 个命名热点文件
- `docs/` 内存在约 `33` 个命名热点文件
- `deploy/` 下 `6` 个文件全部涉及命名切换

结论：

1. 全仓一次性 rename 风险过高，不适合与当前联调主线并行推进。
2. 第一波应优先处理 metadata、README、deploy/service、运维命令别名。
3. Rust package、binary、config dir、runtime label 必须延后，避免破坏现有启动和 worker 收口。

## 3. Naming Matrix

| 项目面 | Canonical Name Now | Compatibility Keep | Phase 2 Postpone |
|---|---|---|---|
| 对外项目名 | `chimera-iceclaw` | 文档中说明兼容 `ironelf / ironclaw` 过渡期 | 完成全量多语言文档收口 |
| Git 元数据 | `chimera-iceclaw` repository/homepage | 当前 `origin` 仍可保持 `ironelf` 直到远端切换 | 最终把 canonical remote 变成默认 push 目标 |
| deploy / service 名称 | `chimera-iceclaw.service`, `chimera-iceclaw-restart` | 保留 `ironclaw.service`, `ironclaw-restart` | 删除旧 unit / alias |
| deploy env 路径 | 优先 `/opt/chimera-iceclaw/.env` | 继续兼容 `/opt/ironclaw/.env` | 最终只保留新路径 |
| macOS 本地服务标识 | 对外显示 `chimera-iceclaw` | 运行文件仍沿用 `ironclaw.pid` / `ironclaw.current.log` | 迁移到新 state 名称 |
| Cargo homepage / repository | 指向 `chimera-iceclaw` | 包名仍是 `ironclaw` | 将 package / crate / binary 真正 rename |
| Rust package / crate / binary | 不改 | `ironclaw` | Phase 2 统一切换 |
| `~/.ironclaw` 配置目录 | 不改 | `~/.ironclaw` | Phase 2 设计迁移脚本与软链策略 |
| Docker image / runtime labels | 不改主默认值 | `ironclaw-worker`, `ironclaw.job_id` | Phase 2 随运行面统一切换 |
| CI / test hardcode | 不改 | 保持当前 | Phase 2 批量重构 |

## 4. 本轮第一波实现范围

本轮直接落地：

1. `Cargo.toml` 的 `homepage` / `repository`
2. README 顶部 canonical naming 说明
3. `deploy/chimera-iceclaw.service` 新 canonical unit
4. `deploy/ironclaw.service` 明确为 legacy compatibility unit
5. `deploy/setup.sh` / `deploy/restart.sh` / `deploy/macos-service.sh` 的兼容启动策略
6. `deploy/env.example` 的 canonical agent naming

本轮明确不动：

1. `name = "ironclaw"`
2. workspace crate 目录名
3. CLI 子命令名 `ironclaw`
4. `~/.ironclaw` 默认目录
5. runtime label / container label / worker image 名
6. 大批测试、CI、发布脚本 rename

## 5. 目录与 remote 策略建议

### 本地目录

- canonical 目标目录：
  - `/Users/sourcefire/X-lab/chimera-iceclaw`
- 过渡期保留目录：
  - `/Users/sourcefire/X-lab/ironelf`

建议：

1. 先在新目录做独立 clone 或 worktree。
2. 不要立即改掉现有 `ironelf` 工作目录。
3. 生产切换稳定后，再决定是否引入 `chimera-iceclaw-dev`。

### Git remote

建议切换顺序：

1. 先保留当前 `origin` 指向 `ironelf`
2. 增加新 remote，例如：
   - `git remote add chimera-iceclaw git@github.com:biexiaofeeng-boop/chimera-iceclaw.git`
3. 完成跨仓同步验证后，再决定是否把新 remote 升级为 canonical push 目标

## 6. 结论

本轮 S10.B 第一阶段应定义为：

- `chimera-iceclaw` 成为文档、service、运维命令和元数据的 canonical 名称
- `ironclaw` 暂时保留为 Rust package / binary / config-runtime compatibility 名称
- `ironelf` 目录与旧 remote 暂时保留为过渡期开发与回滚基线
