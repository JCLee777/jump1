# Jump1

基于 Rust 和 GPUI 的轻量桌面启动器与工具箱，可快速打开应用、剪贴板历史和常用开发工具。

## 功能

- 搜索并打开已安装应用。
- Windows 搜索也包含控制面板、任务管理器、设置和常用系统管理工具。
- 支持应用中文名称、英文别名、拼音全拼和首字母检索。
- 记录最近成功打开的应用，主页以图标和名称展示并可设置显示数量。
- 打开绝对路径或 `~/` 路径。
- 主页提供个人、文稿和下载文件夹快捷入口，可在“设置 → 主页”分别显示或隐藏工具插件与快捷操作。
- 打开网址或调用浏览器搜索。
- 支持键盘和鼠标操作。
- 使用 `Option + Space`（macOS）或 `Alt + Space` 呼出和隐藏面板。
- 设置页支持录制全局快捷键并绑定目标文本，可选择先复制前台应用的选中内容，再自动搜索并执行第一项结果。
- 面板失去焦点后自动隐藏，后台服务继续运行。
- Windows 使用单实例运行；“设置 → 通用 → 登录时启动”可让应用在用户登录后于后台启动，并注册全局快捷键。
- macOS 安装版作为后台工具运行，不显示 Dock 图标，并可在设置中开启登录时启动。
- 跟随系统浅色或深色主题。
- 支持简体中文和英文界面。
- 使用项目自绘 SVG，并显示可读取的系统应用图标。
- 内置剪切板历史插件，支持文本、图片和文件，提供类型标记、时间与使用频率排序、搜索、一键粘贴、收藏和 Ctrl+D 置顶便签；便签背景跟随主题色，清空历史时保留收藏。
- 内置 JSON 编辑器插件，进入时自动读取并格式化剪切板中的有效 JSON，同时支持实时校验、压缩、键排序和草稿恢复。
- 内置 YAML 编辑器插件，支持语法高亮、实时校验、格式化、键排序、剪切板导入和独立草稿恢复。
- 内置 Markdown 编辑器插件，支持语法高亮、实时分栏预览、剪切板导入和独立草稿恢复。
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

开发时查看帧耗时：

```bash
cargo run --features dev-profiler
```

按 `Control + Shift + P` 在帧耗时、详细统计和关闭浮层之间切换。浮层显示 GPUI 帧绘制耗时及统计分布，不是持续刷新的 FPS 数字；功能只在开发构建中启用。

## macOS 安装包

在 macOS 上执行：

```bash
./scripts/package-macos.sh
```

脚本会构建 release 版本，生成并临时签名 `Jump1.app`，然后在 `dist/` 输出适合当前
CPU 架构的 DMG。跨架构构建前先用 `rustup target add <target>` 安装目标，再通过
`CARGO_BUILD_TARGET=x86_64-apple-darwin` 或 `CARGO_BUILD_TARGET=aarch64-apple-darwin` 指定
Intel 或 Apple Silicon 架构。打开 DMG，将 Jump1 拖入 Applications 即可安装。发布给其他用户时可通过
`CODESIGN_IDENTITY="Developer ID Application: ..."` 指定正式签名证书。默认临时签名使用
稳定的 bundle designated requirement，覆盖本地构建时不会因二进制哈希变化反复丢失
辅助功能授权。

## Windows 安装包和便携包

在 Windows PowerShell 中执行：

```powershell
./scripts/package-windows.ps1
```

脚本会构建 release 版本，在 `dist/` 生成安装程序
`Jump1-<版本>-x86_64-windows-setup.exe` 和便携包 `Jump1-<版本>-x86_64-windows.zip`。
安装程序默认安装到当前用户目录，创建开始菜单入口并支持卸载，卸载时会移除 Jump1 的登录启动项；便携包解压后运行
`Jump1.exe` 即可。需要安装 NSIS 并确保 `makensis.exe` 可用，才能同时生成两种包；只需便携包时可运行
`./scripts/package-windows.ps1 -PortableOnly`。应用图标和版本信息会嵌入可执行文件。

安装版默认位置允许当前用户写入，因此应用内更新可以自动替换文件。便携版也应放在当前用户可写的目录中；
两种包的应用内更新都使用便携 ZIP，安装程序可用于首次安装或手动升级。

macOS 13 及以上的安装版启动时默认显示面板，失去焦点后隐藏并常驻后台，且不显示 Dock 图标。使用全局快捷键可再次打开面板；
需要随系统登录启动时，在“设置 → 通用”中开启“登录时启动”。该开关使用 macOS 原生
登录项，系统要求确认时会打开对应的系统设置页面。

Windows 的“登录时启动”开关会为当前用户注册启动项。登录后 Jump1 在后台启动，面板保持隐藏，托盘图标和全局快捷键仍可用；同一 Windows 会话中只允许运行一个 Jump1 实例。

## 版本与更新

应用版本号以 `Cargo.toml` 为准，版本变更记录维护在 [CHANGELOG.md](CHANGELOG.md)。每个版本的更新说明都应包含简体中文和英文；应用内会根据当前界面语言显示对应内容。发布新版本时，先更新这两处并提交，再推送与版本号一致的 `vMAJOR.MINOR.PATCH` 标签（例如 `v0.2.0`）。GitHub Actions 会校验两种语言的说明，在 Apple Silicon 和 Intel macOS runner 上分别构建 ARM64 与 x86_64 DMG、在 Windows runner 上构建 x86_64 安装程序和便携 ZIP，并将四个安装包和双语变更记录发布到 [GitHub Releases](https://github.com/JCLee777/jump1/releases)。

Apple Silicon Mac、Intel Mac 和 x86_64 Windows 用户可在“设置 → 通用 → 软件更新”检查新版本、查看最新更新说明，并在应用内下载对应架构的安装包。下载完成后点击“安装并重启”，Jump1 会在用户确认后替换当前应用并重新打开；如果安装目录不可写，可从 GitHub Releases 手动更新。其他平台不会显示此更新入口。

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
- `src/plugin/markdown/`：Markdown 编辑器、实时预览和草稿持久化
- `src/plugin/translate/`：大模型翻译页面和服务商配置
- `src/network.rs`：GPUI 资源加载和 OpenAI 兼容 Chat Completions 的共享 HTTP 客户端
- `src/plugin/settings/`：主题和语言设置插件
- `src/plugin/shortcuts/`：全局快捷方式与目标文本的独立持久化
- `src/plugin/system/`：主题和语言等系统级设置插件
- `src/state.rs`：主题和语言等全局偏好的持久化
- `src/plugin/storage.rs`：插件隔离目录和原子持久化能力
- `src/theme.rs`：主题与设计令牌
- `src/update.rs`：检查最新 GitHub Release 并解析更新说明
- `docs/architecture.md`：架构

## 状态

当前主页、剪切板、JSON/YAML/Markdown 编辑器、翻译、快捷方式和设置已经按插件边界组织，并支持全局唤起。扩展插件执行和文件索引尚未实现。
