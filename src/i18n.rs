//! Small, typed localization boundary for product-facing text.

use crate::plugin::protocol::{ResultDetail, ResultKind, ResultTitle};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Locale {
    ZhCn,
    EnUs,
}

impl Locale {
    pub fn from_environment() -> Self {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .find_map(|name| std::env::var(name).ok())
            .map(|value| {
                if value.to_ascii_lowercase().starts_with("zh") {
                    Self::ZhCn
                } else {
                    Self::EnUs
                }
            })
            .unwrap_or(Self::ZhCn)
    }
}

#[derive(Clone)]
pub struct I18n {
    locale: Locale,
}

impl I18n {
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }

    pub fn locale(&self) -> Locale {
        self.locale
    }

    pub fn text(&self, message: Message) -> &'static str {
        match (self.locale, message) {
            (Locale::ZhCn, Message::SearchPlaceholder) => "搜索应用、输入路径或网页关键词…",
            (Locale::ZhCn, Message::EmptyResults) => "没有匹配的项目",
            (Locale::ZhCn, Message::OpenSuccess) => "已交给系统打开",
            (Locale::ZhCn, Message::KeyboardHint) => {
                "↑ ↓ ← → 选择   ·   ↵ 打开   ·   Esc 清空或隐藏"
            }
            (Locale::ZhCn, Message::ClipboardName) => "剪切板",
            (Locale::ZhCn, Message::ClipboardDescription) => {
                "搜索文本、图片和文件，直接粘贴到原位置"
            }
            (Locale::ZhCn, Message::ClipboardSearchPlaceholder) => "搜索剪切板历史…",
            (Locale::ZhCn, Message::ClipboardEmpty) => "复制文本、图片或文件后会显示在这里",
            (Locale::ZhCn, Message::ClipboardCopied) => "已复制到剪切板",
            (Locale::ZhCn, Message::ClipboardKeyboardHint) => {
                "↑ ↓ 选择   ·   → 预览   ·   ↵ 粘贴   ·   Tab 切换排序   ·   Esc 返回"
            }
            (Locale::ZhCn, Message::ClipboardPreviewKeyboardHint) => {
                "↑ ↓ 切换   ·   ← / Esc 关闭预览   ·   ↵ 粘贴"
            }
            (Locale::ZhCn, Message::ClipboardPreviewTruncated) => "仅显示前 16000 个字符",
            (Locale::ZhCn, Message::ClipboardPreviewMoreFiles) => "其余文件未显示",
            (Locale::ZhCn, Message::ClipboardSortRecent) => "按时间",
            (Locale::ZhCn, Message::ClipboardSortFrequent) => "按使用次数",
            (Locale::ZhCn, Message::ClipboardClear) => "清空历史",
            (Locale::ZhCn, Message::ClipboardCleared) => "剪切板历史已清空",
            (Locale::ZhCn, Message::ClipboardHistoryUnlimited) => "无限",
            (Locale::ZhCn, Message::JsonName) => "JSON 编辑器",
            (Locale::ZhCn, Message::JsonDescription) => "校验、格式化、压缩和对比 JSON 数据",
            (Locale::ZhCn, Message::JsonFormat) => "格式化",
            (Locale::ZhCn, Message::JsonMinify) => "压缩",
            (Locale::ZhCn, Message::JsonSortKeys) => "排序键",
            (Locale::ZhCn, Message::JsonImportClipboard) => "从剪切板导入",
            (Locale::ZhCn, Message::JsonCopy) => "复制",
            (Locale::ZhCn, Message::JsonClear) => "清空",
            (Locale::ZhCn, Message::CompareOpen) => "左右对比",
            (Locale::ZhCn, Message::CompareClose) => "退出对比",
            (Locale::ZhCn, Message::CompareLeft) => "原文",
            (Locale::ZhCn, Message::CompareRight) => "对比内容",
            (Locale::ZhCn, Message::CompareEmpty) => "在右侧输入内容开始对比",
            (Locale::ZhCn, Message::CompareEqual) => "结构一致",
            (Locale::ZhCn, Message::CompareFormattingDifferent) => "结构一致，文本有差异",
            (Locale::ZhCn, Message::CompareDifferent) => "结构不同",
            (Locale::ZhCn, Message::CompareInvalid) => "请先修正两侧语法",
            (Locale::ZhCn, Message::JsonEmpty) => "输入或粘贴 JSON 数据",
            (Locale::ZhCn, Message::JsonKeyboardHint) => "支持粘贴   ·   Esc 返回",
            (Locale::ZhCn, Message::JsonIndent) => "JSON 缩进",
            (Locale::ZhCn, Message::JsonIndentDescription) => "格式化时使用的空格数量",
            (Locale::ZhCn, Message::YamlName) => "YAML 编辑器",
            (Locale::ZhCn, Message::YamlDescription) => "校验、格式化、整理和对比 YAML 数据",
            (Locale::ZhCn, Message::YamlFormat) => "格式化",
            (Locale::ZhCn, Message::YamlSortKeys) => "排序键",
            (Locale::ZhCn, Message::YamlImportClipboard) => "从剪切板导入",
            (Locale::ZhCn, Message::YamlCopy) => "复制",
            (Locale::ZhCn, Message::YamlClear) => "清空",
            (Locale::ZhCn, Message::YamlEmpty) => "输入或粘贴 YAML 数据",
            (Locale::ZhCn, Message::YamlKeyboardHint) => "支持粘贴   ·   Esc 返回",
            (Locale::ZhCn, Message::YamlIndent) => "YAML 缩进",
            (Locale::ZhCn, Message::YamlIndentDescription) => "编辑时使用的空格数量",
            (Locale::ZhCn, Message::TranslateName) => "翻译",
            (Locale::ZhCn, Message::TranslateDescription) => "使用大模型翻译文本",
            (Locale::ZhCn, Message::TranslateSourceLabel) => "原文",
            (Locale::ZhCn, Message::TranslateInputPlaceholder) => {
                "输入文本，或从剪切板导入要翻译的内容"
            }
            (Locale::ZhCn, Message::TranslateAutoDirection) => "自动识别方向",
            (Locale::ZhCn, Message::TranslateChineseToEnglish) => "中文 → English",
            (Locale::ZhCn, Message::TranslateEnglishToChinese) => "English → 中文",
            (Locale::ZhCn, Message::TranslateStart) => "翻译",
            (Locale::ZhCn, Message::TranslateWorking) => "正在翻译…",
            (Locale::ZhCn, Message::TranslateOutputLabel) => "译文",
            (Locale::ZhCn, Message::TranslateOutputEmpty) => "翻译结果会显示在这里",
            (Locale::ZhCn, Message::TranslateCopy) => "复制译文",
            (Locale::ZhCn, Message::TranslateCopied) => "已复制译文",
            (Locale::ZhCn, Message::TranslateEmptyInput) => "请先输入要翻译的内容",
            (Locale::ZhCn, Message::TranslateTooLong) => "内容不能超过 32 KB",
            (Locale::ZhCn, Message::TranslateApiKeyRequired) => "请先在设置中保存 API Key",
            (Locale::ZhCn, Message::TranslateSaveFailed) => "无法保存翻译配置",
            (Locale::ZhCn, Message::TranslateSettingsDescription) => {
                "选择服务商和模型，保存 API Key 并测试连接"
            }
            (Locale::ZhCn, Message::TranslateProvider) => "服务商",
            (Locale::ZhCn, Message::TranslateOpenAI) => "OpenAI",
            (Locale::ZhCn, Message::TranslateDeepSeek) => "DeepSeek",
            (Locale::ZhCn, Message::TranslateCustom) => "自定义",
            (Locale::ZhCn, Message::TranslateEndpoint) => "Chat Completions 接口地址",
            (Locale::ZhCn, Message::TranslateEndpointPlaceholder) => "https://…/chat/completions",
            (Locale::ZhCn, Message::TranslateModel) => "模型 ID",
            (Locale::ZhCn, Message::TranslateModelPlaceholder) => "例如 gpt-5-mini",
            (Locale::ZhCn, Message::TranslateApiKey) => "API Key",
            (Locale::ZhCn, Message::TranslateApiKeyPlaceholder) => "sk-…",
            (Locale::ZhCn, Message::TranslateApiKeySaved) => "API Key 已保存在本地设置文件",
            (Locale::ZhCn, Message::TranslateSaveAndTest) => "保存并测试",
            (Locale::ZhCn, Message::TranslateTesting) => "正在测试连接…",
            (Locale::ZhCn, Message::TranslateConnected) => "连接成功，配置已保存",
            (Locale::ZhCn, Message::TranslatePrivacyNote) => {
                "原文会发送到所选服务商；Jump1 不保存翻译内容，API Key 以明文保存在本地设置文件中。"
            }
            (Locale::ZhCn, Message::SettingsName) => "设置",
            (Locale::ZhCn, Message::SettingsDescription) => "主题、语言、面板大小与启动行为",
            (Locale::ZhCn, Message::SettingsGeneral) => "通用",
            (Locale::ZhCn, Message::SettingsSystem) => "系统",
            (Locale::ZhCn, Message::SettingsPlugins) => "插件",
            (Locale::ZhCn, Message::SettingsLauncher) => "主页",
            (Locale::ZhCn, Message::SettingsLauncherDescription) => "应用搜索与最近使用",
            (Locale::ZhCn, Message::SettingsGeneralDescription) => {
                "主题、显示语言、面板大小与启动行为"
            }
            (Locale::ZhCn, Message::SettingsPluginsDescription) => "各插件的独立选项",
            (Locale::ZhCn, Message::SettingsTheme) => "主题",
            (Locale::ZhCn, Message::SettingsAccentColor) => "强调色",
            (Locale::ZhCn, Message::SettingsLanguage) => "语言",
            (Locale::ZhCn, Message::SettingsPanelSize) => "面板初始大小",
            (Locale::ZhCn, Message::PanelSizeSmall) => "小",
            (Locale::ZhCn, Message::PanelSizeMedium) => "中",
            (Locale::ZhCn, Message::PanelSizeLarge) => "大",
            (Locale::ZhCn, Message::PanelSizeCustom) => "自定义",
            (Locale::ZhCn, Message::SettingsPanelSizeCustomDimensions) => "自定义宽高",
            (Locale::ZhCn, Message::SettingsPanelSizeDescription) => {
                "单位为像素，可直接输入或使用步进按钮调整"
            }
            (Locale::ZhCn, Message::SettingsPanelWidth) => "宽",
            (Locale::ZhCn, Message::SettingsPanelHeight) => "高",
            (Locale::ZhCn, Message::SettingsLaunchAtLogin) => "登录时启动",
            (Locale::ZhCn, Message::SettingsLaunchAtLoginDescription) => {
                "登录系统后在后台运行，可直接使用全局快捷键"
            }
            (Locale::ZhCn, Message::LaunchAtLoginEnabled) => "已开启登录时启动",
            (Locale::ZhCn, Message::LaunchAtLoginDisabled) => "已关闭登录时启动",
            (Locale::ZhCn, Message::ThemeSystem) => "跟随系统",
            (Locale::ZhCn, Message::ThemeLight) => "浅色",
            (Locale::ZhCn, Message::ThemeDark) => "深色",
            (Locale::ZhCn, Message::AccentColorDefault) => "默认",
            (Locale::ZhCn, Message::AccentColorBlue) => "蓝色",
            (Locale::ZhCn, Message::AccentColorPurple) => "紫色",
            (Locale::ZhCn, Message::AccentColorPink) => "粉色",
            (Locale::ZhCn, Message::AccentColorOrange) => "橙色",
            (Locale::ZhCn, Message::AccentColorGreen) => "绿色",
            (Locale::ZhCn, Message::AccentColorTeal) => "青色",
            (Locale::ZhCn, Message::LanguageChinese) => "简体中文",
            (Locale::ZhCn, Message::LanguageEnglish) => "English",
            (Locale::ZhCn, Message::SettingsKeyboardHint) => "Esc 返回",
            (Locale::ZhCn, Message::SettingsUpdatesTitle) => "软件更新",
            (Locale::ZhCn, Message::SettingsCurrentVersion) => "当前版本",
            (Locale::ZhCn, Message::SettingsCheckForUpdates) => "检查更新",
            (Locale::ZhCn, Message::SettingsCheckingUpdates) => "正在检查…",
            (Locale::ZhCn, Message::SettingsNeverChecked) => "尚未检查更新",
            (Locale::ZhCn, Message::SettingsCheckFailed) => "检查失败",
            (Locale::ZhCn, Message::SettingsNoReleases) => "GitHub 上还没有已发布的版本",
            (Locale::ZhCn, Message::SettingsUpToDate) => "当前已是最新版本",
            (Locale::ZhCn, Message::SettingsUpdateAvailable) => "发现新版本",
            (Locale::ZhCn, Message::SettingsDownloadUpdate) => "下载更新",
            (Locale::ZhCn, Message::SettingsDownloadingUpdate) => "正在下载更新…",
            (Locale::ZhCn, Message::SettingsUpdateDownloaded) => "更新已下载",
            (Locale::ZhCn, Message::SettingsInstallUpdate) => "安装并重启",
            (Locale::ZhCn, Message::SettingsInstallingUpdate) => "正在安装，应用即将重启…",
            (Locale::ZhCn, Message::SettingsDownloadFailed) => "下载失败",
            (Locale::ZhCn, Message::SettingsInstallFailed) => "安装失败",
            (Locale::ZhCn, Message::SettingsReleaseNotes) => "更新日志",
            (Locale::ZhCn, Message::SettingsOpenRelease) => "在 GitHub 查看",
            (Locale::ZhCn, Message::ClipboardHistoryLimit) => "历史记录数量",
            (Locale::ZhCn, Message::ClipboardHistoryLimitDescription) => "保存的剪切板条目数量",
            (Locale::ZhCn, Message::RecentApplications) => "最近使用",
            (Locale::ZhCn, Message::LauncherRecentLimit) => "最近使用数量",
            (Locale::ZhCn, Message::LauncherRecentLimitDescription) => "主页显示的最近应用数量",
            (Locale::ZhCn, Message::ShortcutBindings) => "快捷方式",
            (Locale::ZhCn, Message::ShortcutBindingsDescription) => {
                "录制全局快捷键，填写目标文本，并可在执行前复制当前选中内容"
            }
            (Locale::ZhCn, Message::ShortcutKeyColumn) => "快捷键",
            (Locale::ZhCn, Message::ShortcutTargetColumn) => "目标文本",
            (Locale::ZhCn, Message::ShortcutAutoCopyColumn) => "自动复制",
            (Locale::ZhCn, Message::ShortcutBindingsEmpty) => "还没有快捷方式",
            (Locale::ZhCn, Message::ShortcutBindingAdd) => "+ 添加快捷方式",
            (Locale::ZhCn, Message::ShortcutRecording) => "请按快捷键…",
            (Locale::ZhCn, Message::ShortcutRecord) => "点击录制",
            (Locale::ZhCn, Message::ShortcutTargetPlaceholder) => "例如：微信、剪贴板、翻译",
            (Locale::ZhCn, Message::ShortcutConflict) => "冲突",
            (Locale::EnUs, Message::SearchPlaceholder) => {
                "Search apps, enter a path, or search the web…"
            }
            (Locale::EnUs, Message::EmptyResults) => "No matching items",
            (Locale::EnUs, Message::OpenSuccess) => "Sent to the system",
            (Locale::EnUs, Message::KeyboardHint) => {
                "↑ ↓ ← → Select   ·   ↵ Open   ·   Esc Clear or Hide"
            }
            (Locale::EnUs, Message::ClipboardName) => "Clipboard",
            (Locale::EnUs, Message::ClipboardDescription) => {
                "Search text, images, and files, then paste them back"
            }
            (Locale::EnUs, Message::ClipboardSearchPlaceholder) => "Search clipboard history…",
            (Locale::EnUs, Message::ClipboardEmpty) => "Copied text, images, and files appear here",
            (Locale::EnUs, Message::ClipboardCopied) => "Copied to clipboard",
            (Locale::EnUs, Message::ClipboardKeyboardHint) => {
                "↑ ↓ Select   ·   → Preview   ·   ↵ Paste   ·   Tab Sort   ·   Esc Back"
            }
            (Locale::EnUs, Message::ClipboardPreviewKeyboardHint) => {
                "↑ ↓ Switch   ·   ← / Esc Close preview   ·   ↵ Paste"
            }
            (Locale::EnUs, Message::ClipboardPreviewTruncated) => {
                "Showing the first 16000 characters"
            }
            (Locale::EnUs, Message::ClipboardPreviewMoreFiles) => "More files not shown",
            (Locale::EnUs, Message::ClipboardSortRecent) => "Recent",
            (Locale::EnUs, Message::ClipboardSortFrequent) => "Most used",
            (Locale::EnUs, Message::ClipboardClear) => "Clear history",
            (Locale::EnUs, Message::ClipboardCleared) => "Clipboard history cleared",
            (Locale::EnUs, Message::ClipboardHistoryUnlimited) => "Unlimited",
            (Locale::EnUs, Message::JsonName) => "JSON Editor",
            (Locale::EnUs, Message::JsonDescription) => {
                "Validate, format, minify, and compare JSON data"
            }
            (Locale::EnUs, Message::JsonFormat) => "Format",
            (Locale::EnUs, Message::JsonMinify) => "Minify",
            (Locale::EnUs, Message::JsonSortKeys) => "Sort keys",
            (Locale::EnUs, Message::JsonImportClipboard) => "Import clipboard",
            (Locale::EnUs, Message::JsonCopy) => "Copy",
            (Locale::EnUs, Message::JsonClear) => "Clear",
            (Locale::EnUs, Message::CompareOpen) => "Compare",
            (Locale::EnUs, Message::CompareClose) => "Close compare",
            (Locale::EnUs, Message::CompareLeft) => "Original",
            (Locale::EnUs, Message::CompareRight) => "Compare with",
            (Locale::EnUs, Message::CompareEmpty) => "Enter content on the right to compare",
            (Locale::EnUs, Message::CompareEqual) => "Same structure",
            (Locale::EnUs, Message::CompareFormattingDifferent) => "Same structure, different text",
            (Locale::EnUs, Message::CompareDifferent) => "Different structure",
            (Locale::EnUs, Message::CompareInvalid) => "Fix syntax on both sides first",
            (Locale::EnUs, Message::JsonEmpty) => "Enter or paste JSON data",
            (Locale::EnUs, Message::JsonKeyboardHint) => "Paste supported   ·   Esc Back",
            (Locale::EnUs, Message::JsonIndent) => "JSON indentation",
            (Locale::EnUs, Message::JsonIndentDescription) => {
                "Number of spaces used while formatting"
            }
            (Locale::EnUs, Message::YamlName) => "YAML Editor",
            (Locale::EnUs, Message::YamlDescription) => {
                "Validate, format, organize, and compare YAML data"
            }
            (Locale::EnUs, Message::YamlFormat) => "Format",
            (Locale::EnUs, Message::YamlSortKeys) => "Sort keys",
            (Locale::EnUs, Message::YamlImportClipboard) => "Import clipboard",
            (Locale::EnUs, Message::YamlCopy) => "Copy",
            (Locale::EnUs, Message::YamlClear) => "Clear",
            (Locale::EnUs, Message::YamlEmpty) => "Enter or paste YAML data",
            (Locale::EnUs, Message::YamlKeyboardHint) => "Paste supported   ·   Esc Back",
            (Locale::EnUs, Message::YamlIndent) => "YAML indentation",
            (Locale::EnUs, Message::YamlIndentDescription) => "Number of spaces used while editing",
            (Locale::EnUs, Message::TranslateName) => "Translate",
            (Locale::EnUs, Message::TranslateDescription) => "Translate text with an LLM",
            (Locale::EnUs, Message::TranslateSourceLabel) => "Source",
            (Locale::EnUs, Message::TranslateInputPlaceholder) => {
                "Enter text or import it from the clipboard"
            }
            (Locale::EnUs, Message::TranslateAutoDirection) => "Auto-detect direction",
            (Locale::EnUs, Message::TranslateChineseToEnglish) => "Chinese → English",
            (Locale::EnUs, Message::TranslateEnglishToChinese) => "English → Chinese",
            (Locale::EnUs, Message::TranslateStart) => "Translate",
            (Locale::EnUs, Message::TranslateWorking) => "Translating…",
            (Locale::EnUs, Message::TranslateOutputLabel) => "Translation",
            (Locale::EnUs, Message::TranslateOutputEmpty) => "Your translation will appear here",
            (Locale::EnUs, Message::TranslateCopy) => "Copy translation",
            (Locale::EnUs, Message::TranslateCopied) => "Translation copied",
            (Locale::EnUs, Message::TranslateEmptyInput) => "Enter text to translate first",
            (Locale::EnUs, Message::TranslateTooLong) => "Text must be 32 KB or less",
            (Locale::EnUs, Message::TranslateApiKeyRequired) => "Save an API key in Settings first",
            (Locale::EnUs, Message::TranslateSaveFailed) => "Could not save translation settings",
            (Locale::EnUs, Message::TranslateSettingsDescription) => {
                "Choose a provider and model, then save and test your API key"
            }
            (Locale::EnUs, Message::TranslateProvider) => "Provider",
            (Locale::EnUs, Message::TranslateOpenAI) => "OpenAI",
            (Locale::EnUs, Message::TranslateDeepSeek) => "DeepSeek",
            (Locale::EnUs, Message::TranslateCustom) => "Custom",
            (Locale::EnUs, Message::TranslateEndpoint) => "Chat Completions endpoint",
            (Locale::EnUs, Message::TranslateEndpointPlaceholder) => "https://…/chat/completions",
            (Locale::EnUs, Message::TranslateModel) => "Model ID",
            (Locale::EnUs, Message::TranslateModelPlaceholder) => "For example, gpt-5-mini",
            (Locale::EnUs, Message::TranslateApiKey) => "API Key",
            (Locale::EnUs, Message::TranslateApiKeyPlaceholder) => "sk-…",
            (Locale::EnUs, Message::TranslateApiKeySaved) => "API Key saved in local settings",
            (Locale::EnUs, Message::TranslateSaveAndTest) => "Save and test",
            (Locale::EnUs, Message::TranslateTesting) => "Testing connection…",
            (Locale::EnUs, Message::TranslateConnected) => "Connected; settings saved",
            (Locale::EnUs, Message::TranslatePrivacyNote) => {
                "Text is sent to your selected provider. Jump1 does not save translations; your API Key is stored as plain text in a local settings file."
            }
            (Locale::EnUs, Message::SettingsName) => "Settings",
            (Locale::EnUs, Message::SettingsDescription) => {
                "Theme, language, panel size, and startup behavior"
            }
            (Locale::EnUs, Message::SettingsGeneral) => "General",
            (Locale::EnUs, Message::SettingsSystem) => "System",
            (Locale::EnUs, Message::SettingsPlugins) => "Plugins",
            (Locale::EnUs, Message::SettingsLauncher) => "Home",
            (Locale::EnUs, Message::SettingsLauncherDescription) => {
                "Application search and recents"
            }
            (Locale::EnUs, Message::SettingsGeneralDescription) => {
                "Theme, display language, panel size, and startup behavior"
            }
            (Locale::EnUs, Message::SettingsPluginsDescription) => "Options owned by each plugin",
            (Locale::EnUs, Message::SettingsTheme) => "Theme",
            (Locale::EnUs, Message::SettingsAccentColor) => "Accent color",
            (Locale::EnUs, Message::SettingsLanguage) => "Language",
            (Locale::EnUs, Message::SettingsPanelSize) => "Initial panel size",
            (Locale::EnUs, Message::PanelSizeSmall) => "Small",
            (Locale::EnUs, Message::PanelSizeMedium) => "Medium",
            (Locale::EnUs, Message::PanelSizeLarge) => "Large",
            (Locale::EnUs, Message::PanelSizeCustom) => "Custom",
            (Locale::EnUs, Message::SettingsPanelSizeCustomDimensions) => "Custom dimensions",
            (Locale::EnUs, Message::SettingsPanelSizeDescription) => {
                "Size in pixels; type a value or use the stepper buttons"
            }
            (Locale::EnUs, Message::SettingsPanelWidth) => "W",
            (Locale::EnUs, Message::SettingsPanelHeight) => "H",
            (Locale::EnUs, Message::SettingsLaunchAtLogin) => "Launch at login",
            (Locale::EnUs, Message::SettingsLaunchAtLoginDescription) => {
                "Run in the background after login so global shortcuts are ready"
            }
            (Locale::EnUs, Message::LaunchAtLoginEnabled) => "Launch at login enabled",
            (Locale::EnUs, Message::LaunchAtLoginDisabled) => "Launch at login disabled",
            (Locale::EnUs, Message::ThemeSystem) => "System",
            (Locale::EnUs, Message::ThemeLight) => "Light",
            (Locale::EnUs, Message::ThemeDark) => "Dark",
            (Locale::EnUs, Message::AccentColorDefault) => "Default",
            (Locale::EnUs, Message::AccentColorBlue) => "Blue",
            (Locale::EnUs, Message::AccentColorPurple) => "Purple",
            (Locale::EnUs, Message::AccentColorPink) => "Pink",
            (Locale::EnUs, Message::AccentColorOrange) => "Orange",
            (Locale::EnUs, Message::AccentColorGreen) => "Green",
            (Locale::EnUs, Message::AccentColorTeal) => "Teal",
            (Locale::EnUs, Message::LanguageChinese) => "简体中文",
            (Locale::EnUs, Message::LanguageEnglish) => "English",
            (Locale::EnUs, Message::SettingsKeyboardHint) => "Esc Back",
            (Locale::EnUs, Message::SettingsUpdatesTitle) => "Software Updates",
            (Locale::EnUs, Message::SettingsCurrentVersion) => "Current version",
            (Locale::EnUs, Message::SettingsCheckForUpdates) => "Check for updates",
            (Locale::EnUs, Message::SettingsCheckingUpdates) => "Checking…",
            (Locale::EnUs, Message::SettingsNeverChecked) => "Updates have not been checked",
            (Locale::EnUs, Message::SettingsCheckFailed) => "Update check failed",
            (Locale::EnUs, Message::SettingsNoReleases) => "No published releases on GitHub yet",
            (Locale::EnUs, Message::SettingsUpToDate) => "You are up to date",
            (Locale::EnUs, Message::SettingsUpdateAvailable) => "Update available",
            (Locale::EnUs, Message::SettingsDownloadUpdate) => "Download update",
            (Locale::EnUs, Message::SettingsDownloadingUpdate) => "Downloading update…",
            (Locale::EnUs, Message::SettingsUpdateDownloaded) => "Update downloaded",
            (Locale::EnUs, Message::SettingsInstallUpdate) => "Install and Relaunch",
            (Locale::EnUs, Message::SettingsInstallingUpdate) => "Installing; app will relaunch…",
            (Locale::EnUs, Message::SettingsDownloadFailed) => "Download failed",
            (Locale::EnUs, Message::SettingsInstallFailed) => "Installation failed",
            (Locale::EnUs, Message::SettingsReleaseNotes) => "Release notes",
            (Locale::EnUs, Message::SettingsOpenRelease) => "View on GitHub",
            (Locale::EnUs, Message::ClipboardHistoryLimit) => "History limit",
            (Locale::EnUs, Message::ClipboardHistoryLimitDescription) => {
                "Number of clipboard entries to keep"
            }
            (Locale::EnUs, Message::RecentApplications) => "Recently Used",
            (Locale::EnUs, Message::LauncherRecentLimit) => "Recent items",
            (Locale::EnUs, Message::LauncherRecentLimitDescription) => {
                "Number of recent applications shown on the home page"
            }
            (Locale::EnUs, Message::ShortcutBindings) => "Shortcuts",
            (Locale::EnUs, Message::ShortcutBindingsDescription) => {
                "Record a shortcut, enter its target, and optionally copy selected text first"
            }
            (Locale::EnUs, Message::ShortcutKeyColumn) => "Shortcut",
            (Locale::EnUs, Message::ShortcutTargetColumn) => "Target text",
            (Locale::EnUs, Message::ShortcutAutoCopyColumn) => "Auto copy",
            (Locale::EnUs, Message::ShortcutBindingsEmpty) => "No shortcuts yet",
            (Locale::EnUs, Message::ShortcutBindingAdd) => "+ Add shortcut",
            (Locale::EnUs, Message::ShortcutRecording) => "Press shortcut…",
            (Locale::EnUs, Message::ShortcutRecord) => "Click to record",
            (Locale::EnUs, Message::ShortcutTargetPlaceholder) => {
                "For example: WeChat, Clipboard, Translate"
            }
            (Locale::EnUs, Message::ShortcutConflict) => "Conflict",
        }
    }

    pub fn open_error(&self, error: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("打开失败：{error}"),
            Locale::EnUs => format!("Could not open: {error}"),
        }
    }

    pub fn launch_at_login_error(&self, error: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("无法更新登录项：{error}"),
            Locale::EnUs => format!("Could not update login item: {error}"),
        }
    }

    pub fn result_summary(&self, results: usize, apps: usize) -> String {
        match self.locale {
            Locale::ZhCn => format!("{results} 个结果 · {apps} 个已发现应用"),
            Locale::EnUs => format!("{results} results · {apps} apps discovered"),
        }
    }

    pub fn result_kind(&self, kind: ResultKind) -> &'static str {
        match (self.locale, kind) {
            (Locale::ZhCn, ResultKind::Application) => "应用",
            (Locale::ZhCn, ResultKind::RecentApplication) => "最近使用",
            (Locale::ZhCn, ResultKind::Path) => "文件或文件夹",
            (Locale::ZhCn, ResultKind::Plugin) => "插件",
            (Locale::ZhCn, ResultKind::QuickAction) => "快捷操作",
            (Locale::ZhCn, ResultKind::Web) => "网络",
            (Locale::EnUs, ResultKind::Application) => "Application",
            (Locale::EnUs, ResultKind::RecentApplication) => "Recent",
            (Locale::EnUs, ResultKind::Path) => "File or folder",
            (Locale::EnUs, ResultKind::Plugin) => "Plugin",
            (Locale::EnUs, ResultKind::QuickAction) => "Quick action",
            (Locale::EnUs, ResultKind::Web) => "Web",
        }
    }

    pub fn result_title(&self, title: &ResultTitle) -> String {
        match (self.locale, title) {
            (_, ResultTitle::Application(name)) => name.clone(),
            (Locale::ZhCn, ResultTitle::ClipboardPlugin) => "剪切板".into(),
            (Locale::ZhCn, ResultTitle::JsonPlugin) => "JSON 编辑器".into(),
            (Locale::ZhCn, ResultTitle::YamlPlugin) => "YAML 编辑器".into(),
            (Locale::ZhCn, ResultTitle::TranslatePlugin) => "翻译".into(),
            (Locale::ZhCn, ResultTitle::SettingsPlugin) => "设置".into(),
            (Locale::ZhCn, ResultTitle::OpenHome) => "打开个人文件夹".into(),
            (Locale::ZhCn, ResultTitle::OpenDownloads) => "打开下载文件夹".into(),
            (Locale::ZhCn, ResultTitle::OpenPath(name)) => format!("打开 {name}"),
            (Locale::ZhCn, ResultTitle::OpenUrl) => "打开网址".into(),
            (Locale::ZhCn, ResultTitle::SearchWeb(query)) => {
                format!("在浏览器搜索「{query}」")
            }
            (Locale::EnUs, ResultTitle::OpenHome) => "Open home folder".into(),
            (Locale::EnUs, ResultTitle::ClipboardPlugin) => "Clipboard".into(),
            (Locale::EnUs, ResultTitle::JsonPlugin) => "JSON Editor".into(),
            (Locale::EnUs, ResultTitle::YamlPlugin) => "YAML Editor".into(),
            (Locale::EnUs, ResultTitle::TranslatePlugin) => "Translate".into(),
            (Locale::EnUs, ResultTitle::SettingsPlugin) => "Settings".into(),
            (Locale::EnUs, ResultTitle::OpenDownloads) => "Open Downloads".into(),
            (Locale::EnUs, ResultTitle::OpenPath(name)) => format!("Open {name}"),
            (Locale::EnUs, ResultTitle::OpenUrl) => "Open URL".into(),
            (Locale::EnUs, ResultTitle::SearchWeb(query)) => {
                format!("Search the web for “{query}”")
            }
        }
    }

    pub fn result_detail(&self, detail: &ResultDetail) -> String {
        match detail {
            ResultDetail::Literal(value) => value.clone(),
            ResultDetail::ClipboardPlugin => self.text(Message::ClipboardDescription).into(),
            ResultDetail::JsonPlugin => self.text(Message::JsonDescription).into(),
            ResultDetail::YamlPlugin => self.text(Message::YamlDescription).into(),
            ResultDetail::TranslatePlugin => self.text(Message::TranslateDescription).into(),
            ResultDetail::SettingsPlugin => self.text(Message::SettingsDescription).into(),
        }
    }

    pub fn json_valid_summary(&self, lines: usize, bytes: usize) -> String {
        match self.locale {
            Locale::ZhCn => format!("JSON 格式正确 · {lines} 行 · {}", format_bytes(bytes)),
            Locale::EnUs => format!("Valid JSON · {lines} lines · {}", format_bytes(bytes)),
        }
    }

    pub fn json_error(&self, line: usize, column: usize, message: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("第 {line} 行，第 {column} 列：JSON 语法错误"),
            Locale::EnUs => format!("Line {line}, column {column}: {message}"),
        }
    }

    pub fn yaml_valid_summary(&self, lines: usize, bytes: usize) -> String {
        match self.locale {
            Locale::ZhCn => format!("YAML 格式正确 · {lines} 行 · {}", format_bytes(bytes)),
            Locale::EnUs => format!("Valid YAML · {lines} lines · {}", format_bytes(bytes)),
        }
    }

    pub fn yaml_error(&self, line: usize, column: usize, message: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("第 {line} 行，第 {column} 列：YAML 语法错误"),
            Locale::EnUs => format!("Line {line}, column {column}: {message}"),
        }
    }

    pub fn clipboard_summary(&self, visible: usize, total: usize) -> String {
        match self.locale {
            Locale::ZhCn => format!("{visible} 条结果 · 共保存 {total} 条"),
            Locale::EnUs => format!("{visible} results · {total} saved"),
        }
    }

    pub fn clipboard_entry_detail(&self, bytes: usize, captured_at: u64, use_count: u64) -> String {
        let date = chrono::DateTime::from_timestamp(captured_at as i64, 0)
            .map(|date| {
                date.with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| "-".into());
        match self.locale {
            Locale::ZhCn => format!("{} · {date} · 使用 {use_count} 次", format_bytes(bytes)),
            Locale::EnUs => format!("{} · {date} · used {use_count} times", format_bytes(bytes)),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Message {
    SearchPlaceholder,
    EmptyResults,
    OpenSuccess,
    KeyboardHint,
    ClipboardName,
    ClipboardDescription,
    ClipboardSearchPlaceholder,
    ClipboardEmpty,
    ClipboardCopied,
    ClipboardKeyboardHint,
    ClipboardPreviewKeyboardHint,
    ClipboardPreviewTruncated,
    ClipboardPreviewMoreFiles,
    ClipboardSortRecent,
    ClipboardSortFrequent,
    ClipboardClear,
    ClipboardCleared,
    ClipboardHistoryUnlimited,
    JsonName,
    JsonDescription,
    JsonFormat,
    JsonMinify,
    JsonSortKeys,
    JsonImportClipboard,
    JsonCopy,
    JsonClear,
    CompareOpen,
    CompareClose,
    CompareLeft,
    CompareRight,
    CompareEmpty,
    CompareEqual,
    CompareFormattingDifferent,
    CompareDifferent,
    CompareInvalid,
    JsonEmpty,
    JsonKeyboardHint,
    JsonIndent,
    JsonIndentDescription,
    YamlName,
    YamlDescription,
    YamlFormat,
    YamlSortKeys,
    YamlImportClipboard,
    YamlCopy,
    YamlClear,
    YamlEmpty,
    YamlKeyboardHint,
    YamlIndent,
    YamlIndentDescription,
    TranslateName,
    TranslateDescription,
    TranslateSourceLabel,
    TranslateInputPlaceholder,
    TranslateAutoDirection,
    TranslateChineseToEnglish,
    TranslateEnglishToChinese,
    TranslateStart,
    TranslateWorking,
    TranslateOutputLabel,
    TranslateOutputEmpty,
    TranslateCopy,
    TranslateCopied,
    TranslateEmptyInput,
    TranslateTooLong,
    TranslateApiKeyRequired,
    TranslateSaveFailed,
    TranslateSettingsDescription,
    TranslateProvider,
    TranslateOpenAI,
    TranslateDeepSeek,
    TranslateCustom,
    TranslateEndpoint,
    TranslateEndpointPlaceholder,
    TranslateModel,
    TranslateModelPlaceholder,
    TranslateApiKey,
    TranslateApiKeyPlaceholder,
    TranslateApiKeySaved,
    TranslateSaveAndTest,
    TranslateTesting,
    TranslateConnected,
    TranslatePrivacyNote,
    SettingsName,
    SettingsDescription,
    SettingsGeneral,
    SettingsSystem,
    SettingsPlugins,
    SettingsLauncher,
    SettingsLauncherDescription,
    SettingsGeneralDescription,
    SettingsPluginsDescription,
    SettingsTheme,
    SettingsAccentColor,
    SettingsLanguage,
    SettingsPanelSize,
    PanelSizeSmall,
    PanelSizeMedium,
    PanelSizeLarge,
    PanelSizeCustom,
    SettingsPanelSizeCustomDimensions,
    SettingsPanelSizeDescription,
    SettingsPanelWidth,
    SettingsPanelHeight,
    SettingsLaunchAtLogin,
    SettingsLaunchAtLoginDescription,
    LaunchAtLoginEnabled,
    LaunchAtLoginDisabled,
    ThemeSystem,
    ThemeLight,
    ThemeDark,
    AccentColorDefault,
    AccentColorBlue,
    AccentColorPurple,
    AccentColorPink,
    AccentColorOrange,
    AccentColorGreen,
    AccentColorTeal,
    LanguageChinese,
    LanguageEnglish,
    SettingsKeyboardHint,
    SettingsUpdatesTitle,
    SettingsCurrentVersion,
    SettingsCheckForUpdates,
    SettingsCheckingUpdates,
    SettingsNeverChecked,
    SettingsCheckFailed,
    SettingsNoReleases,
    SettingsUpToDate,
    SettingsUpdateAvailable,
    SettingsDownloadUpdate,
    SettingsDownloadingUpdate,
    SettingsUpdateDownloaded,
    SettingsInstallUpdate,
    SettingsInstallingUpdate,
    SettingsDownloadFailed,
    SettingsInstallFailed,
    SettingsReleaseNotes,
    SettingsOpenRelease,
    ClipboardHistoryLimit,
    ClipboardHistoryLimitDescription,
    RecentApplications,
    LauncherRecentLimit,
    LauncherRecentLimitDescription,
    ShortcutBindings,
    ShortcutBindingsDescription,
    ShortcutKeyColumn,
    ShortcutTargetColumn,
    ShortcutAutoCopyColumn,
    ShortcutBindingsEmpty,
    ShortcutBindingAdd,
    ShortcutRecording,
    ShortcutRecord,
    ShortcutTargetPlaceholder,
    ShortcutConflict,
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    }
}
