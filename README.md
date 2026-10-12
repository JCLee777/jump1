# Jump1

基于 Rust 和 GPUI 的轻量桌面启动器与工具箱，可快速打开应用、剪贴板历史和常用开发工具。

## 功能

- 搜索并打开已安装应用。
- 支持应用中文名称、英文别名、拼音全拼和首字母检索。
- 记录最近成功打开的应用，主页以图标和名称展示并可设置显示数量。
- 打开绝对路径或 `~/` 路径。
- 主页提供个人、文稿和下载文件夹快捷入口，可在“设置 → 主页”分别显示或隐藏工具插件与快捷操作。
- 打开网址或调用浏览器搜索。
- 支持键盘和鼠标操作。
- 使用 `Option + Space`（macOS）或 `Alt + Space` 呼出和隐藏面板。
- 设置页支持录制全局快捷键并绑定目标文本，可选择先复制前台应用的选中内容，再自动搜索并执行第一项结果。
- 面板失去焦点后自动隐藏，后台服务继续运行。
- macOS 安装版作为后台工具运行，不显示 Dock 图标，并可在设置中开启登录时启动。
- 跟随系统浅色或深色主题。
- 支持简体中文和英文界面。
- 使用项目自绘 SVG，并显示可读取的系统应用图标。
- 内置剪切板历史插件，支持文本、图片和文件，提供类型标记、时间与使用频率排序、搜索及一键粘贴。
- 内置 JSON 编辑器插件，进入时自动读取并格式化剪切板中的有效 JSON，同时支持实时校验、压缩、键排序和草稿恢复。
- 内置 YAML 编辑器插件，支持语法高亮、实时校验、格式化、键排序、剪切板导入和独立草稿恢复。
- 设置页支持全局主题、语言，以及插件贡献的设置项。
- 插件 manifest 支持版本、权限、页面/服务类型和生命周期状态。
- 内置翻译插件在打开时自动填入剪切板文本，并通过 OpenAI、DeepSeek 或自定义的 OpenAI 兼容 Chat Completions 接口翻译；根据原文自动在中文和英文之间选择翻译方向，结果可复制。服务商、模型、接口地址和 API Key 均保存在翻译插件的本地配置中，API Key 以明文存储。

## 运行

```bash
cargo run
```

性能构建：

```bash
cargo run --release
```

## macOS 安装包

在 macOS 上执行：

```bash
./scripts/package-macos.sh
```

脚本会构建 release 版本，生成并临时签名 `Jump1.app`，然后在 `dist/` 输出适合当前
CPU 架构的 DMG。打开 DMG，将 Jump1 拖入 Applications 即可安装。发布给其他用户时可通过
`CODESIGN_IDENTITY="Developer ID Application: ..."` 指定正式签名证书。默认临时签名使用
稳定的 bundle designated requirement，覆盖本地构建时不会因二进制哈希变化反复丢失
辅助功能授权。

macOS 13 及以上的安装版启动时默认显示面板，失去焦点后隐藏并常驻后台，且不显示 Dock 图标。使用全局快捷键可再次打开面板；
需要随系统登录启动时，在“设置 → 通用”中开启“登录时启动”。该开关使用 macOS 原生
登录项，系统要求确认时会打开对应的系统设置页面。

## 版本与更新

应用版本号以 `Cargo.toml` 为准，版本变更记录维护在 [CHANGELOG.md](CHANGELOG.md)。每个版本的更新说明都应包含简体中文和英文；应用内会根据当前界面语言显示对应内容。发布新版本时，先更新这两处并提交，再推送与版本号一致的 `vMAJOR.MINOR.PATCH` 标签（例如 `v0.2.0`）。GitHub Actions 会校验两种语言的说明，在 Apple Silicon macOS runner 上构建 ARM64 DMG，并将双语变更记录发布到 [GitHub Releases](https://github.com/JCLee777/jump1/releases)。

Apple Silicon Mac 用户可在“设置 → 通用 → 软件更新”检查新版本、查看最新更新说明，并在应用内下载 ARM64 安装包。下载完成后点击“安装并重启”，Jump1 会替换当前应用并重新打开；如果当前安装目录不可写，可从 GitHub Releases 手动更新。其他平台不会显示此更新入口。

## 结构

- `src/main.rs`：程序入口
- `src/lib.rs`：应用启动
- `src/app.rs`：应用壳、路由、输入和插件协调
- `src/icon.rs`：自绘图标
- `src/i18n.rs`：国际化
- `src/hotkey.rs`：跨平台全局快捷键
- `src/platform.rs`：系统适配
- `src/plugin/mod.rs`：插件注册、身份、生命周期和统一调度
- `src/plugin/protocol.rs`：插件与宿主共享的结果和动作协议
- `src/plugin/api.rs`：插件 trait、manifest、版本和权限声明
- `src/plugin/host_view.rs`：活动插件渲染和统一事件边界
- `src/plugin/launcher/`：主页插件、搜索模型和主页视图
- `src/plugin/clipboard/`：剪切板插件、持久化和插件视图
- `src/plugin/json/`：JSON 编辑器、校验、格式化和草稿持久化
- `src/plugin/yaml/`：YAML 编辑器、校验、格式化和草稿持久化
- `src/plugin/translate/`：大模型翻译页面和服务商配置
- `src/network.rs`：共享的 OpenAI 兼容 Chat Completions HTTP 客户端
- `src/plugin/settings/`：主题和语言设置插件
- `src/plugin/shortcuts/`：全局快捷方式与目标文本的独立持久化
- `src/plugin/system/`：主题和语言等系统级设置插件
- `src/state.rs`：主题和语言等全局偏好的持久化
- `src/plugin/storage.rs`：插件隔离目录和原子持久化能力
- `src/theme.rs`：主题与设计令牌
- `src/update.rs`：检查最新 GitHub Release 并解析更新说明
- `docs/architecture.md`：架构

## 状态

当前主页、剪切板、JSON/YAML 编辑器、翻译、快捷方式和设置已经按插件边界组织，并支持全局唤起。扩展插件执行和文件索引尚未实现。
