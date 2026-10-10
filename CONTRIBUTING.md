# 贡献指南

感谢你对 Jump1 的关注。

## 提交前检查

```bash
cargo fmt --check
cargo check
```

## 提交规范

- 一个提交只解决一个主题。
- 提交标题使用祈使句，推荐使用 `feat:`、`fix:`、`refactor:`、`docs:` 等前缀。
- 用户可见行为、模块边界或运行方式变化时，同时更新 README 或 `docs/`。
- 不要提交 `target/`、编辑器配置、密钥或本地应用数据。

## 代码边界

请先阅读 [架构说明](docs/architecture.md) 和 [仓库指南](AGENTS.md)，保持插件状态、
平台适配与 UI 宿主职责分离。
