# 给 ironelf Rust Dev 的启动提示词（R1）

你在分支 `codex/r1-runtime-bridge-v1` 上工作。

目标：把 `ironelf` 实现为 `chimera-core` 的安全 runtime executor，不替代 `chimera-core` 的协作控制层。

硬约束：

1. 先协议，后 API，再状态机，最后验收。
2. 只做 runtime plane，不接管对话协作层。
3. 所有桥接返回必须结构化，不能只返回自由文本。
4. phase 1 必须 fail-open，不能要求 `chimera-core` 全量改造后才能工作。
5. 保持高风险任务优先接入，低风险快任务不强制走 runtime。

执行顺序：

1. T01-T04：协议与版本兼容
2. T05-T08：bridge API 与健康检查
3. T09-T12：状态机、超时、取消、失败收口
4. T13-T16：准入策略、验收回填、交接总结

交付物：

1. bridge API 代码与 schema 对应实现
2. checks 回填证据
3. commit hash 与 changed files
4. known risks 与 integration notes
