# 06-Checks-S10.B-Rename-Base-Alignment-v1-2026-04-07

## 检查目标

对 S10.B 第一波 rename / base-alignment 做六项交付确认：

1. rename inventory 是否落库
2. naming matrix 是否清晰
3. 改动是否限制在低风险对齐层
4. build / script 静态校验是否通过
5. 运维切换步骤是否明确
6. rollback 步骤是否明确

## 1. 交付结果总览

本轮产出：

- rename inventory：
  - `docs/Issue-Checks/2026-04/05-Rename-Inventory-S10.B-Chimera-Iceclaw-v1-2026-04-07.md`
- checks / close-out：
  - `docs/Issue-Checks/2026-04/06-Checks-S10.B-Rename-Base-Alignment-v1-2026-04-07.md`
- 第一波实现：
  - 对外 canonical naming 前移到 `chimera-iceclaw`
  - 保留 `ironclaw` 的 service / restart / env / binary 兼容入口

## 2. 改动文件列表

本轮核心修改文件：

- `Cargo.toml`
- `README.md`
- `README.zh-CN.md`
- `deploy/env.example`
- `deploy/ironclaw.service`
- `deploy/chimera-iceclaw.service`
- `deploy/restart.sh`
- `deploy/setup.sh`
- `deploy/macos-service.sh`
- `docs/Issue-Checks/2026-04/00-INDEX-2026-04.md`
- `docs/Issue-Checks/2026-04/05-Rename-Inventory-S10.B-Chimera-Iceclaw-v1-2026-04-07.md`
- `docs/Issue-Checks/2026-04/06-Checks-S10.B-Rename-Base-Alignment-v1-2026-04-07.md`

## 3. 实现边界确认

本轮已完成：

1. `chimera-iceclaw` 成为 README 顶部与 deploy/service 的 canonical 名称
2. `Cargo.toml` 的 `homepage` / `repository` 已对齐到新目标仓
3. 新增 `deploy/chimera-iceclaw.service`，作为 canonical systemd unit
4. 保留 `deploy/ironclaw.service` 作为 legacy compatibility unit
5. `deploy/setup.sh` 同时安装 canonical 与 legacy restart alias
6. `deploy/restart.sh` 支持自动选择 `chimera-iceclaw` 或 `ironclaw`
7. `deploy/macos-service.sh` 支持 canonical label 展示，但继续复用旧 runtime state 文件名

本轮明确未做：

1. 不改 Cargo package 名 `ironclaw`
2. 不改 workspace crate 名与 crate 目录
3. 不改 CLI 命令 `ironclaw`
4. 不改 `~/.ironclaw` 默认目录
5. 不改 worker image、runtime labels、sandbox labels
6. 不做 CI / tests / release 全量 rename

判断：

- 范围聚焦在 metadata / docs / deploy / ops 兼容层
- 未触碰当前 `chimera-core <-> ironelf` 联调关键运行面
- 符合“先 inventory 和 naming matrix，再做最小落地”的任务要求

## 4. Build / Test / Static Checks

### 执行命令

```bash
cd /Users/sourcefire/X-lab/ironelf
cargo build
cargo test control_plane --features libsql -- --nocapture
bash -n deploy/macos-service.sh
bash -n deploy/restart.sh
bash -n deploy/setup.sh
bash deploy/macos-service.sh status
```

### 结果

- `cargo build`：通过
- `cargo test control_plane --features libsql -- --nocapture`：通过
  - `8 passed`
  - `0 failed`
- `bash -n deploy/macos-service.sh`：通过
- `bash -n deploy/restart.sh`：通过
- `bash -n deploy/setup.sh`：通过
- `bash deploy/macos-service.sh status`：通过
  - 本机服务仍处于 `running`
  - 当前二进制仍为 `/Volumes/ChimeraData/build-cache/rust-targets/ironelf/debug/ironclaw`
  - 当前日志路径仍为 `~/.ironclaw/logs/ironclaw.current.log`

说明：

- 本轮没有触碰 Rust package / binary / runtime label，因此 focused runtime regression 仍以现有 control-plane 测试和本机服务状态核对为主。
- `macos-service.sh` 已完成 canonical service label 对齐，但默认 state 文件路径仍保持 legacy 命名，符合本轮兼容策略。

## 5. 运维切换步骤

### 生产切换建议

1. 保留旧目录与旧 unit：
   - `/opt/ironclaw`
   - `ironclaw.service`
2. 准备新 canonical 目录：
   - `/opt/chimera-iceclaw`
3. 将 `.env` 放到以下任一路径：
   - `/opt/chimera-iceclaw/.env`
   - `/opt/ironclaw/.env`
4. 安装并启用 canonical unit：
   - `sudo systemctl enable chimera-iceclaw`
   - `sudo systemctl start chimera-iceclaw`
5. 验证：
   - `sudo systemctl status chimera-iceclaw`
   - `docker logs chimera-iceclaw`
6. 运维重启优先使用：
   - `sudo chimera-iceclaw-restart --with-proxy`

### 开发目录切换建议

1. 不原地改名当前 `/Users/sourcefire/X-lab/ironelf`
2. 新建 canonical 工作目录：
   - `/Users/sourcefire/X-lab/chimera-iceclaw`
3. 保留旧目录作为回滚基线
4. 待远程仓和本地 smoke 稳定后，再切开发主入口

## 6. Rollback 步骤

### 服务回滚

1. 停止 canonical unit：
   - `sudo systemctl stop chimera-iceclaw`
2. 启动 legacy unit：
   - `sudo systemctl start ironclaw`
3. 使用 legacy restart alias：
   - `sudo ironclaw-restart --service ironclaw --with-proxy`

### 目录回滚

1. 保持 `/Users/sourcefire/X-lab/ironelf` 不动
2. 如果 `/Users/sourcefire/X-lab/chimera-iceclaw` smoke 失败，直接回旧目录继续开发
3. 不在本轮删除旧目录、旧 unit、旧 restart alias、旧 env 路径

### Remote 回滚

1. 当前 `origin` 继续保留 `ironelf`
2. 新 remote 建议先作为附加 remote，而不是直接替换
3. 若新仓协作异常，继续以旧 remote 推进当前联调主线

## 7. 已知风险

1. `Cargo package` / `crate` / `binary` / `~/.ironclaw` 尚未 rename，当前仍是双命名过渡状态。
2. canonical service 已切到 `chimera-iceclaw`，但默认 Docker image registry path 仍保持旧 `ironclaw` 命名，后续需要独立切换。
3. README 多语言与 CI / tests / release 仍存在大量 `ironclaw` hardcode，本轮仅完成第一波对齐。

## 8. 结论

本轮 S10.B 第一波 rename / base-alignment 已满足最低交付：

1. rename inventory 已完成
2. naming matrix 已完成
3. 第一波最小实现已落在 metadata / docs / deploy / ops compatibility 层
4. 当前设计保留旧 service、旧命令别名、旧目录与旧 env 路径，支持快速回滚
5. 下一波再进入 package / binary / config dir / runtime labels 的统一 rename
