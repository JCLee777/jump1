# Changelog

All notable changes to Jump1 are recorded here. Each published version is also
available on the [GitHub Releases page](https://github.com/JCLee777/jump1/releases).

## [Unreleased]

## [0.1.13] - 2026-10-13

### 简体中文

- 修复 Windows 托盘图标左右键事件识别；面板已显示时再次点击不改变窗口状态。
- 为 Windows 可执行文件嵌入应用图标，增加便携 ZIP 打包，并在 GitHub Release 中构建 Windows 版本。
- 增加 Windows 用户级安装程序，创建开始菜单入口并支持卸载；卸载时清除登录启动项，GitHub Release 同时提供安装程序和便携 ZIP。
- Windows 设置页支持检查、下载和经用户确认后安装更新。
- Windows 启动时使用系统互斥锁保证单例；“登录时启动”会在登录后于后台启动并注册全局快捷键。

### English

- Fixed Windows notification area click handling; clicking again while the panel is visible leaves it unchanged.
- Embedded the app icon in the Windows executable, added portable ZIP packaging, and added Windows builds to GitHub Releases.
- Added a per-user Windows installer with a Start Menu shortcut and uninstaller that removes the login startup entry; GitHub Releases now provide both the installer and portable ZIP.
- Added Windows update checks, downloads, and installation after explicit user confirmation.
- Enforced a single running Windows instance and implemented “Run at login” to start in the background with global shortcuts ready.

## [0.1.12] - 2026-10-12

### 简体中文

- 主页新增文稿文件夹快捷入口，并将个人、下载、文稿入口名称简化。
- 设置 → 主页可分别控制内置插件和文件夹快捷操作是否显示在主页默认列表中。
- 强调色选择改为仅显示色块，移除颜色文字提示。

### English

- Added a Documents folder shortcut to the home page and shortened the Home, Downloads, and Documents labels.
- Added separate settings to show or hide built-in plugins and folder actions in the home page's default list.
- Simplified accent color selection to swatches without text labels.

## [0.1.11] - 2026-10-12

### 简体中文

- 修复多显示器下启动器仍显示在默认屏幕的问题；面板启动或快捷键唤起时，会在鼠标所在屏幕居中显示。

### English

- Fixed the launcher appearing on the default display in multi-display setups. The panel now opens centered on the display containing the pointer at startup and when invoked by its hotkey.

## [0.1.10] - 2026-10-12

### 简体中文

- 修复深色主题下灰色强调色的翻译按钮文字对比度，调整设置页“保存并测试”和翻译页“翻译”按钮的文字颜色。

### English

- Improved text contrast for translation buttons with the gray accent in dark mode, including “Save and test” in settings and “Translate” on the translation page.

## [0.1.9] - 2026-10-12

### 简体中文

- 修复 macOS 系统应用名称索引，支持使用中文名称、拼音全拼和首字母搜索活动监视器等应用。
- macOS 会读取应用包中的编译本地化名称，搜索框也明确提示支持中文和拼音。

### English

- Fixed localized name indexing for macOS system apps, enabling searches by Chinese name, full pinyin, and initials.
- Read compiled localized names from app bundles and clarified Chinese and pinyin support in the launcher search hint.

## [0.1.8] - 2026-10-11

### 简体中文

- 打开翻译插件时自动填入剪切板文本，并移除手动读取剪切板按钮。
- 修复译文内容撑宽输入区的问题，原文和译文输入框现在保持等宽。

### English

- Automatically fills the translation page with clipboard text when opened and removes the manual clipboard button.
- Prevented translated content from expanding its field, keeping the source and translation inputs the same width.

## [0.1.7] - 2026-10-11

### 简体中文

- 新增大模型翻译插件，支持中英文自动翻译方向、剪贴板导入和复制译文。
- 支持配置 OpenAI、DeepSeek 和自定义兼容接口，并可在设置中保存和测试连接。
- 优化翻译页面布局，原文和译文输入区对齐并随面板高度伸展，翻译操作位于两栏中间。

### English

- Added an AI translation plugin with automatic Chinese and English direction, clipboard import, and copyable results.
- Added OpenAI, DeepSeek, and custom compatible endpoint settings, with API key saving and connection testing.
- Refined the translation layout with aligned, resizable source and result fields and a centered translate action.

## [0.1.6] - 2026-10-10

### 简体中文

- 优化剪贴板图片预览速度与缓存管理，离开剪贴板页面后释放图片缓存。
- 调整设置页布局与主题对比度。

### English

- Improved clipboard image preview speed and cache management, releasing image caches when leaving the clipboard page.
- Refined settings layout and theme contrast.

## [0.1.5] - 2026-10-10

### 简体中文

- 简化软件更新页面，聚焦当前版本、更新状态和新版本说明。
- 移除冗余介绍、安装提示和历史版本列表。

### English

- Simplified the software update page to focus on the current version, update status, and new release notes.
- Removed redundant descriptions, installation hints, and release history.

## [0.1.4] - 2026-10-10

### 简体中文

- 软件更新支持在应用内下载；确认安装后自动替换应用并重新启动。

### English

- Software updates can now be downloaded in the app, then installed after confirmation with an automatic relaunch.

## [0.1.3] - 2026-10-10

### 简体中文

- 启动器首页支持使用左右方向键切换选中项目。

### English

- The launcher home screen now supports moving the selection with the left and right arrow keys.

## [0.1.2] - 2026-10-10

### 简体中文

- GitHub API 限流或不可用时，自动回退到公开 Releases feed 检查更新。

### English

- Update checks now fall back to the public Releases feed when the GitHub API is rate-limited or unavailable.

## [0.1.0] - 2026-10-10

### 简体中文

- Jump1 首个公开版本，提供轻量桌面启动器与工具箱。
- 支持应用搜索、剪切板历史、JSON 和 YAML 编辑器、快捷方式与设置。
- 提供 Apple Silicon macOS 安装包，以及基于 GitHub 的更新检查、更新日志和版本记录。

### English

- Initial public release of Jump1, a lightweight desktop launcher and utility toolkit.
- Application search, clipboard history, JSON and YAML editors, shortcuts, and settings.
- Apple Silicon macOS package and GitHub-based update checks, release notes, and version history.

## [0.1.1] - 2026-10-10

### 简体中文

- 更新日志提供简体中文和英文，并在应用内按当前界面语言显示。
- 发布流程会检查中英文说明均已填写。

### English

- Release notes are available in Simplified Chinese and English, and the app displays the matching language.
- The release workflow verifies that both translations are present.
