# 01-TaskPackage-S10.A-Chimera-Iceclaw-Control-Plane-Slice-v1-2026-04-06

## 任务目标

将跨项目总资料包中的 S10.A `ironelf / chimera-iceclaw` 最小 durable control-plane slice 同步回 `ironelf` 子项目文档区，避免后续项目内维护断链。

## 上游来源

上游资料包：

- `/Users/sourcefire/X-lab/docs/stack-architecture/2026-04-s10-review-v1/13-task-pack-chimera-iceclaw.md`

## 本轮目标摘要

本轮 `ironelf` 侧目标不是做完整平台重构，而是补出最小可联调的 control-plane ownership：

1. 接收来自 `chimera-core` 的最小 `TaskIntent`
2. 生成 durable task identity
3. 返回最小 `TaskReceipt`
4. 定义内部 `DispatchRequest`
5. 定义最小 `ExecutionResult`
6. 记录最小事件流以支撑后续扩展

## 范围

### In scope

- control-plane ingress
- task acceptance
- durable receipt / event ledger
- 最小持久化
- handler / auth / store 接线
- focused tests

### Out of scope

- 完整审批系统
- 完整任务调度系统
- GUI 层
- Claude Code 或 Cherry Studio 集成
- 大规模 repo 重构

## 本轮预期交付

- `ironelf` 中最小 control-plane slice 落地
- 子项目侧补档
- 代码抽检与 focused cargo tests
- 协议边界问题登记

## 本轮特别说明

检查时发现：

- `ironelf` 当前接收路由为 `/api/control/tasks/accept`
- `chimera-core` 默认 dispatch path 仍为 `/api/controlplane/task-intents`

这意味着：

- `ironelf` 单侧实现与测试已基本成立
- 但跨仓默认配置仍未天然对齐
- 若不通过配置覆盖或路由兼容层处理，真实 handoff 仍可能被 404/协议不匹配卡住
