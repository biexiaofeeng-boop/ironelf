# 10-Dev-Prod-Topology-S10.B-v1-2026-04-07

## 目标

明确 `chimera-iceclaw` / `chimera-iceclaw-dev` / `ironelf` 三套目录在当前阶段的职责边界。

## 推荐拓扑

### 1. 生产 / 预发布基线

- 目录：
  - `/Users/sourcefire/X-lab/chimera-iceclaw`
- 角色：
  - canonical 项目目录
  - 预发布 / 生产基线
  - 对外项目名与新 remote 基线

### 2. 开发工作区

- 目录：
  - `/Users/sourcefire/X-lab/chimera-iceclaw-dev`
- 角色：
  - 日常开发目录
  - 分支试验与提交目录
  - 与生产基线物理分离，避免误操作

### 3. 历史回退基线

- 目录：
  - `/Users/sourcefire/X-lab/ironelf`
- 角色：
  - 历史兼容目录
  - 回退参照
  - 与旧 remote / 旧上下文保持兼容

## 为什么这更适合 Rust

Rust 的运行环境主要依赖：

1. 编译出来的二进制
2. `.env` / 数据库 / 日志 / service
3. 容器、网络、系统服务

而不是强依赖源码目录名本身。

所以更合理的隔离方式是：

- 让“生产基线目录”和“开发工作区目录”分开
- 而不是继续追求把仓内所有 `ironclaw` 命名都改掉

## 当前脚本

### 开发目录同步脚本

- `deploy/create-dev-workspace.sh`

用途：

- 创建或同步 `/Users/sourcefire/X-lab/chimera-iceclaw-dev`
- 保持 `origin = chimera-iceclaw`
- 保持 `ironelf-legacy` 回退 remote

### 预发布检查脚本

- `deploy/pre_release_check.sh`

用途：

- 检查当前目录的 git 基线
- 检查构建
- 检查本机服务状态
- 检查 gateway / runtime / models 三个 HTTP 接口

## 建议使用方式

### 日常开发

```bash
cd /Users/sourcefire/X-lab/chimera-iceclaw
bash deploy/create-dev-workspace.sh
cd /Users/sourcefire/X-lab/chimera-iceclaw-dev
```

### 预发布检查

```bash
cd /Users/sourcefire/X-lab/chimera-iceclaw
bash deploy/pre_release_check.sh
```

### 回退

```bash
cd /Users/sourcefire/X-lab/ironelf
```
