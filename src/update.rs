//! GitHub Release lookup for supported macOS and Windows builds.

use std::cmp::Ordering;

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use std::{
    os::unix::fs::OpenOptionsExt,
    process::{Command, Stdio},
};

use crate::i18n::Locale;

#[derive(Clone, Debug)]
pub struct ReleaseEntry {
    pub version: String,
    pub notes: ReleaseNotes,
    pub release_url: String,
    pub download_url: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ReleaseNotes {
    zh_cn: Option<String>,
    en_us: Option<String>,
    fallback: String,
}

impl ReleaseNotes {
    fn from_markdown(markdown: &str) -> Self {
        #[derive(Clone, Copy)]
        enum Section {
            ZhCn,
            EnUs,
        }

        let mut result = Self::default();
        let mut section = None;
        let mut found_localized_section = false;

        for line in markdown.lines() {
            match line.trim() {
                "### 简体中文" => {
                    section = Some(Section::ZhCn);
                    found_localized_section = true;
                    continue;
                }
                "### English" => {
                    section = Some(Section::EnUs);
                    found_localized_section = true;
                    continue;
                }
                _ => {}
            }

            let Some(section) = section else { continue };
            let content = match section {
                Section::ZhCn => result.zh_cn.get_or_insert_with(String::new),
                Section::EnUs => result.en_us.get_or_insert_with(String::new),
            };
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
        }

        if found_localized_section {
            result.zh_cn = result
                .zh_cn
                .take()
                .map(|notes| notes.trim().to_owned())
                .filter(|notes| !notes.is_empty());
            result.en_us = result
                .en_us
                .take()
                .map(|notes| notes.trim().to_owned())
                .filter(|notes| !notes.is_empty());
        } else {
            result.fallback = markdown.trim().to_owned();
        }

        result
    }

    pub fn for_locale(&self, locale: Locale) -> &str {
        let (preferred, other) = match locale {
            Locale::ZhCn => (self.zh_cn.as_deref(), self.en_us.as_deref()),
            Locale::EnUs => (self.en_us.as_deref(), self.zh_cn.as_deref()),
        };
        preferred
            .filter(|notes| !notes.is_empty())
            .or_else(|| other.filter(|notes| !notes.is_empty()))
            .unwrap_or(&self.fallback)
    }
}

#[derive(Clone, Debug, Default)]
pub enum UpdateStatus {
    #[default]
    NotChecked,
    Checking,
    Ready(Vec<ReleaseEntry>),
    Failed(String),
    Downloading {
        releases: Vec<ReleaseEntry>,
        version: String,
    },
    Downloaded {
        releases: Vec<ReleaseEntry>,
        version: String,
        package_path: std::path::PathBuf,
    },
    Installing {
        releases: Vec<ReleaseEntry>,
        version: String,
    },
    DownloadFailed {
        releases: Vec<ReleaseEntry>,
        error: String,
    },
    InstallFailed {
        releases: Vec<ReleaseEntry>,
        version: String,
        package_path: std::path::PathBuf,
        error: String,
    },
}

impl UpdateStatus {
    pub fn releases(&self) -> Option<&[ReleaseEntry]> {
        match self {
            Self::Ready(releases)
            | Self::Downloading { releases, .. }
            | Self::Downloaded { releases, .. }
            | Self::Installing { releases, .. }
            | Self::DownloadFailed { releases, .. }
            | Self::InstallFailed { releases, .. } => Some(releases),
            Self::NotChecked | Self::Checking | Self::Failed(_) => None,
        }
    }

    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::Checking | Self::Downloading { .. } | Self::Installing { .. }
        )
    }
}

pub fn is_newer_version(candidate: &str, current: &str) -> bool {
    let Some(candidate) = version_parts(candidate) else {
        return false;
    };
    let Some(current) = version_parts(current) else {
        return false;
    };
    candidate.cmp(&current) == Ordering::Greater
}

fn version_parts(version: &str) -> Option<[u64; 3]> {
    let version = version.trim().strip_prefix('v').unwrap_or(version.trim());
    let mut parts = version.split('.');
    Some([
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ])
}

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
pub fn download_update(version: &str, download_url: &str) -> Result<PathBuf, String> {
    const MAX_DOWNLOAD_BYTES: u64 = 128 * 1024 * 1024;
    let version_number = version.strip_prefix('v').unwrap_or(version);

    if !is_stable_release_version(version_number) {
        return Err("更新版本号无效".to_owned());
    }
    let release_tag = if version.starts_with('v') {
        version.to_owned()
    } else {
        format!("v{version_number}")
    };
    let asset_name = platform_asset_name(version_number);
    let expected_asset_path =
        format!("/JCLee777/jump1/releases/download/{release_tag}/{asset_name}");
    let parsed_url =
        reqwest::Url::parse(download_url).map_err(|error| format!("更新下载地址无效：{error}"))?;
    if parsed_url.scheme() != "https"
        || parsed_url.host_str() != Some("github.com")
        || parsed_url.path() != expected_asset_path
        || parsed_url.query().is_some()
        || parsed_url.fragment().is_some()
    {
        return Err("更新下载地址不属于官方 Jump1 Release".to_owned());
    }

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(300))
        .user_agent(concat!("Jump1/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("无法初始化更新下载：{error}"))?;
    let mut response = client
        .get(download_url)
        .send()
        .map_err(|error| format!("更新下载失败：{error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("GitHub 下载返回状态码 {status}"));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_DOWNLOAD_BYTES)
    {
        return Err("更新安装包超过允许的大小".to_owned());
    }

    let update_dir = update_directory()?;
    fs::create_dir_all(&update_dir).map_err(|error| format!("无法创建更新目录：{error}"))?;
    let final_path = update_dir.join(asset_name);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("无法生成临时文件名：{error}"))?
        .as_nanos();
    let temporary_path = update_dir.join(format!(
        ".Jump1-{version_number}-{}-{nonce}.download",
        std::process::id()
    ));
    let mut open_options = OpenOptions::new();
    open_options.write(true).create_new(true);
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    open_options.mode(0o600);
    let mut file = open_options
        .open(&temporary_path)
        .map_err(|error| format!("无法创建更新文件：{error}"))?;

    let copy_result = (|| {
        let mut buffer = [0_u8; 64 * 1024];
        let mut downloaded = 0_u64;
        loop {
            let bytes_read = response
                .read(&mut buffer)
                .map_err(|error| format!("读取更新数据失败：{error}"))?;
            if bytes_read == 0 {
                break;
            }
            downloaded = downloaded.saturating_add(bytes_read as u64);
            if downloaded > MAX_DOWNLOAD_BYTES {
                return Err("更新安装包超过允许的大小".to_owned());
            }
            file.write_all(&buffer[..bytes_read])
                .map_err(|error| format!("保存更新数据失败：{error}"))?;
        }
        if downloaded == 0 {
            return Err("下载的更新安装包为空".to_owned());
        }
        file.sync_all()
            .map_err(|error| format!("无法保存完整的更新安装包：{error}"))?;
        Ok(())
    })();
    drop(file);
    if let Err(error) = copy_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    if final_path.exists() {
        if let Err(error) = fs::remove_file(&final_path) {
            let _ = fs::remove_file(&temporary_path);
            return Err(format!("无法替换旧更新文件：{error}"));
        }
    }
    if let Err(error) = fs::rename(&temporary_path, &final_path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(format!("无法完成更新文件保存：{error}"));
    }
    Ok(final_path)
}

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
fn update_directory() -> Result<PathBuf, String> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        let home = std::env::var_os("HOME").ok_or_else(|| "无法读取用户目录".to_owned())?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Jump1")
            .join("Updates"))
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| "无法读取本地应用数据目录".to_owned())?;
        Ok(PathBuf::from(local_app_data).join("Jump1").join("Updates"))
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn install_update_and_restart(package_path: &Path, version: &str) -> Result<(), String> {
    let version_number = version.strip_prefix('v').unwrap_or(version);
    if !is_stable_release_version(version_number) {
        return Err("更新版本号无效".to_owned());
    }
    let expected_dmg = platform_asset_name(version_number);
    if package_path.file_name().and_then(|name| name.to_str()) != Some(&expected_dmg)
        || !package_path.is_file()
    {
        return Err("找不到已下载的更新安装包".to_owned());
    }

    let executable = std::env::current_exe().map_err(|error| format!("无法定位应用：{error}"))?;
    let app_bundle = executable
        .ancestors()
        .nth(3)
        .filter(|path| path.extension().is_some_and(|extension| extension == "app"))
        .ok_or_else(|| "请先将 Jump1 安装到 Applications 文件夹".to_owned())?;
    let info_plist = app_bundle.join("Contents/Info.plist");
    if !info_plist.is_file() {
        return Err("当前应用安装位置无效".to_owned());
    }
    let parent = app_bundle
        .parent()
        .ok_or_else(|| "无法确定应用安装目录".to_owned())?;
    ensure_directory_writable(parent)?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("无法生成安装脚本名：{error}"))?
        .as_nanos();
    let script_path =
        std::env::temp_dir().join(format!("jump1-update-{}-{nonce}.sh", std::process::id()));
    let mut script = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o700)
        .open(&script_path)
        .map_err(|error| format!("无法准备更新安装器：{error}"))?;
    script
        .write_all(UPDATE_INSTALLER_SCRIPT.as_bytes())
        .and_then(|()| script.sync_all())
        .map_err(|error| format!("无法保存更新安装器：{error}"))?;
    drop(script);

    let mut installer = Command::new("/bin/sh");
    installer
        .arg(&script_path)
        .arg(std::process::id().to_string())
        .arg(package_path)
        .arg(app_bundle)
        .arg(version_number)
        .arg(&script_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Err(error) = installer.spawn() {
        let _ = fs::remove_file(&script_path);
        return Err(format!("无法启动更新安装器：{error}"));
    }
    Ok(())
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn ensure_directory_writable(directory: &Path) -> Result<(), String> {
    let probe = directory.join(format!(".jump1-update-probe-{}", std::process::id()));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&probe)
        .map_err(|error| {
            format!("Jump1 无法写入安装目录，请确认应用位于可写的 Applications 文件夹：{error}")
        })?;
    drop(file);
    fs::remove_file(probe).map_err(|error| format!("无法清理更新检查文件：{error}"))
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub fn install_update_and_restart(package_path: &Path, version: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let version_number = version.strip_prefix('v').unwrap_or(version);
    if !is_stable_release_version(version_number) {
        return Err("更新版本号无效".to_owned());
    }
    let expected_package = platform_asset_name(version_number);
    if package_path.file_name().and_then(|name| name.to_str()) != Some(&expected_package)
        || !package_path.is_file()
    {
        return Err("找不到已下载的更新安装包".to_owned());
    }

    let executable = std::env::current_exe().map_err(|error| format!("无法定位应用：{error}"))?;
    let update_dir = update_directory()?;
    fs::create_dir_all(&update_dir).map_err(|error| format!("无法创建更新目录：{error}"))?;
    let script_path = update_dir.join(format!("install-update-{}.ps1", std::process::id()));
    fs::write(&script_path, WINDOWS_UPDATE_INSTALLER_SCRIPT)
        .map_err(|error| format!("无法准备更新安装器：{error}"))?;

    let mut installer = std::process::Command::new("powershell.exe");
    installer
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&script_path)
        .arg(std::process::id().to_string())
        .arg(package_path)
        .arg(&executable)
        .arg(version_number)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if let Err(error) = installer.spawn() {
        let _ = fs::remove_file(&script_path);
        return Err(format!("无法启动更新安装器：{error}"));
    }
    Ok(())
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const WINDOWS_UPDATE_INSTALLER_SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'
$ProcessIdToWait = [int]$args[0]
$ArchivePath = [System.IO.Path]::GetFullPath($args[1])
$TargetPath = [System.IO.Path]::GetFullPath($args[2])
$ExpectedVersion = $args[3]
$ScriptPath = $MyInvocation.MyCommand.Path
$TargetDirectory = [System.IO.Path]::GetDirectoryName($TargetPath)
$Nonce = [guid]::NewGuid().ToString('N')
$StagedPath = Join-Path $TargetDirectory ".Jump1-$Nonce.exe"
$BackupPath = Join-Path $TargetDirectory ".Jump1-$Nonce.backup.exe"
$MovedOld = $false
$InstalledNew = $false

try {
    while (Get-Process -Id $ProcessIdToWait -ErrorAction SilentlyContinue) {
        Start-Sleep -Milliseconds 200
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $Archive = [System.IO.Compression.ZipFile]::OpenRead($ArchivePath)
    try {
        $AllowedNames = @('Jump1.exe', 'README-Windows.txt')
        if ($Archive.Entries.Count -ne 2) { throw 'The update archive has an unexpected file list.' }
        foreach ($Entry in $Archive.Entries) {
            if ($Entry.FullName -notin $AllowedNames -or $Entry.FullName.Contains('/') -or $Entry.FullName.Contains('\')) {
                throw 'The update archive contains an unexpected path.'
            }
        }
        $ExecutableEntry = $Archive.GetEntry('Jump1.exe')
        if ($null -eq $ExecutableEntry -or $ExecutableEntry.Length -le 0 -or $ExecutableEntry.Length -gt 128MB) {
            throw 'The update executable is missing or has an invalid size.'
        }
        $InputStream = $ExecutableEntry.Open()
        $OutputStream = [System.IO.File]::Create($StagedPath)
        try { $InputStream.CopyTo($OutputStream) }
        finally { $OutputStream.Dispose(); $InputStream.Dispose() }
    }
    finally { $Archive.Dispose() }

    $StagedVersion = (Get-Item -LiteralPath $StagedPath).VersionInfo.ProductVersion
    if ($StagedVersion -ne $ExpectedVersion) { throw "The update version does not match ($StagedVersion)." }

    Move-Item -LiteralPath $TargetPath -Destination $BackupPath
    $MovedOld = $true
    Move-Item -LiteralPath $StagedPath -Destination $TargetPath
    $InstalledNew = $true
    Start-Process -FilePath $TargetPath
    Remove-Item -LiteralPath $BackupPath -Force
    $MovedOld = $false
    Remove-Item -LiteralPath $ArchivePath -Force -ErrorAction SilentlyContinue
}
catch {
    if ($InstalledNew -and (Test-Path -LiteralPath $TargetPath)) {
        Remove-Item -LiteralPath $TargetPath -Force -ErrorAction SilentlyContinue
    }
    if ($MovedOld -and (Test-Path -LiteralPath $BackupPath)) {
        Move-Item -LiteralPath $BackupPath -Destination $TargetPath -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $StagedPath) {
        Remove-Item -LiteralPath $StagedPath -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $TargetPath) {
        Start-Process -FilePath $TargetPath -ErrorAction SilentlyContinue
    }
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.MessageBox]::Show(
        "Jump1 could not finish the update. The previous version was restored when possible.`r`n`r`n$($_.Exception.Message)",
        'Jump1 update',
        [System.Windows.Forms.MessageBoxButtons]::OK,
        [System.Windows.Forms.MessageBoxIcon]::Warning
    ) | Out-Null
}
finally {
    Remove-Item -LiteralPath $ScriptPath -Force -ErrorAction SilentlyContinue
}
"#;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const UPDATE_INSTALLER_SCRIPT: &str = r#"#!/bin/sh
set -eu

process_id="$1"
dmg_path="$2"
target_path="$3"
expected_version="$4"
script_path="$5"
parent_dir=${target_path%/*}
app_name=${target_path##*/}
mount_point=""
staging_root=""
backup_root=""
old_app_moved=0
installed_ok=0

cleanup() {
  result=$?
  if [ "$result" -ne 0 ] && [ "$installed_ok" -eq 0 ] && [ "$old_app_moved" -eq 1 ] && [ -e "$backup_root/$app_name" ]; then
    if [ -e "$target_path" ]; then /bin/rm -rf "$target_path" || true; fi
    if /bin/mv "$backup_root/$app_name" "$target_path"; then old_app_moved=0; fi
  fi
  if [ -n "$mount_point" ]; then
    /usr/bin/hdiutil detach -quiet "$mount_point" >/dev/null 2>&1 || true
    /bin/rm -rf "$mount_point" || true
  fi
  if [ -n "$staging_root" ]; then /bin/rm -rf "$staging_root" || true; fi
  if [ -n "$backup_root" ] && [ "$old_app_moved" -eq 0 ]; then /bin/rm -rf "$backup_root" || true; fi
  if [ "$result" -eq 0 ]; then /bin/rm -f "$dmg_path"; fi
  /bin/rm -f "$script_path"
  if [ "$result" -ne 0 ]; then
    if [ -d "$target_path" ]; then /usr/bin/open -a "$target_path" >/dev/null 2>&1 || true; fi
    /usr/bin/osascript -e 'display dialog "Jump1 could not finish the update. The previous app was restored when possible." buttons {"OK"} with icon caution' >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

while /bin/kill -0 "$process_id" 2>/dev/null; do /bin/sleep 0.2; done
mount_point=$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/jump1-update-mount.XXXXXX")
staging_root=$(/usr/bin/mktemp -d "$parent_dir/.jump1-update.XXXXXX")
backup_root=$(/usr/bin/mktemp -d "$parent_dir/.jump1-backup.XXXXXX")
/usr/bin/hdiutil attach -quiet -nobrowse -readonly -mountpoint "$mount_point" "$dmg_path"
source_app="$mount_point/Jump1.app"
staged_app="$staging_root/$app_name"
if [ ! -d "$source_app" ]; then echo "Jump1.app is missing from the update image" >&2; exit 1; fi
/usr/bin/ditto "$source_app" "$staged_app"
/usr/bin/codesign --verify --deep --strict "$staged_app"
bundle_id=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$staged_app/Contents/Info.plist")
bundle_version=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$staged_app/Contents/Info.plist")
if [ "$bundle_id" != "io.github.jclee777.jump1" ] || [ "$bundle_version" != "$expected_version" ]; then
  echo "Downloaded app identity or version does not match" >&2
  exit 1
fi
/usr/bin/hdiutil detach -quiet "$mount_point"
/bin/rm -rf "$mount_point"
mount_point=""
/bin/mv "$target_path" "$backup_root/$app_name"
old_app_moved=1
/bin/mv "$staged_app" "$target_path"
if ! /usr/bin/open -a "$target_path"; then echo "Could not relaunch Jump1" >&2; exit 1; fi
installed_ok=1
/bin/rm -rf "$backup_root" || true
backup_root=""
"#;

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
pub fn fetch_latest_release() -> Result<Vec<ReleaseEntry>, String> {
    let releases = GitHubReleaseClient::new()?.releases()?;

    Ok(releases
        .into_iter()
        .filter(|release| {
            !release
                .get("draft")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
                && !release
                    .get("prerelease")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true)
        })
        .filter_map(|release| {
            let version = release.get("tag_name")?.as_str()?.to_owned();
            let version_number = version.strip_prefix('v').unwrap_or(&version).to_owned();
            if !is_stable_release_version(&version_number) {
                return None;
            }
            let release_url = release.get("html_url")?.as_str()?.to_owned();
            let download_name = platform_asset_name(&version_number);
            let download_url = release
                .get("assets")?
                .as_array()?
                .iter()
                .find(|asset| {
                    asset.get("name").and_then(serde_json::Value::as_str) == Some(&download_name)
                })
                .and_then(|asset| asset.get("browser_download_url"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            Some(ReleaseEntry {
                version,
                notes: ReleaseNotes::from_markdown(
                    release
                        .get("body")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default(),
                ),
                release_url,
                download_url,
            })
        })
        .take(1)
        .collect())
}

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
struct GitHubReleaseClient {
    client: reqwest::blocking::Client,
}

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
impl GitHubReleaseClient {
    fn new() -> Result<Self, String> {
        use std::time::Duration;

        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent(concat!("Jump1/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| format!("无法初始化更新客户端：{error}"))?;
        Ok(Self { client })
    }

    fn releases(&self) -> Result<Vec<serde_json::Value>, String> {
        match self.api_releases() {
            Ok(releases) if !releases.is_empty() => Ok(releases),
            Ok(releases) => {
                #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
                {
                    self.atom_releases().or(Ok(releases))
                }
                #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
                {
                    Ok(releases)
                }
            }
            Err(api_error) => {
                #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
                {
                    self.atom_releases().map_err(|feed_error| {
                        format!("{api_error}；GitHub Releases feed 回退失败：{feed_error}")
                    })
                }
                #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
                {
                    Err(api_error)
                }
            }
        }
    }

    fn api_releases(&self) -> Result<Vec<serde_json::Value>, String> {
        const RELEASES_API: &str =
            "https://api.github.com/repos/JCLee777/jump1/releases?per_page=10";

        let response = self
            .client
            .get(RELEASES_API)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .map_err(|error| format!("GitHub 更新请求失败：{error}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("GitHub 返回状态码 {status}"));
        }
        serde_json::from_reader(response)
            .map_err(|error| format!("无法解析 GitHub 版本信息：{error}"))
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn atom_releases(&self) -> Result<Vec<serde_json::Value>, String> {
        const RELEASES_FEED: &str = "https://github.com/JCLee777/jump1/releases.atom";
        const RELEASE_PAGE_PREFIX: &str = "https://github.com/JCLee777/jump1/releases/tag/";

        let response = self
            .client
            .get(RELEASES_FEED)
            .header("Accept", "application/atom+xml")
            .send()
            .map_err(|error| format!("无法读取 Releases feed：{error}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("Releases feed 返回状态码 {status}"));
        }
        let feed = response
            .text()
            .map_err(|error| format!("无法读取 Releases feed 内容：{error}"))?;
        let document = roxmltree::Document::parse(&feed)
            .map_err(|error| format!("无法解析 Releases feed：{error}"))?;

        let releases = document
            .root_element()
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "entry")
            .filter_map(|entry| {
                let tag = entry
                    .children()
                    .find(|node| {
                        node.is_element()
                            && node.tag_name().name() == "link"
                            && node
                                .attribute("rel")
                                .is_none_or(|relation| relation == "alternate")
                    })
                    .and_then(|node| node.attribute("href"))?
                    .strip_prefix(RELEASE_PAGE_PREFIX)?;
                let version_number = tag.strip_prefix('v').unwrap_or(tag);
                if !is_stable_release_version(version_number) {
                    return None;
                }

                let body = entry
                    .children()
                    .find(|node| node.is_element() && node.tag_name().name() == "content")
                    .and_then(|node| node.text())
                    .map(github_html_to_markdown)
                    .unwrap_or_default();
                let download_name = platform_asset_name(version_number);

                Some(serde_json::json!({
                    "tag_name": tag,
                    "body": body,
                    "html_url": format!("{RELEASE_PAGE_PREFIX}{tag}"),
                    "draft": false,
                    "prerelease": false,
                    "assets": [{
                        "name": download_name,
                        "browser_download_url": format!(
                            "https://github.com/JCLee777/jump1/releases/download/{tag}/{download_name}"
                        )
                    }]
                }))
            })
            .take(1)
            .collect::<Vec<_>>();

        if releases.is_empty() {
            return Err("Releases feed 中没有可用的稳定版本".to_owned());
        }
        Ok(releases)
    }
}

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
fn is_stable_release_version(version: &str) -> bool {
    let parts = version.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
fn platform_asset_name(version: &str) -> String {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        format!("Jump1-{version}-aarch64.dmg")
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        format!("Jump1-{version}-x86_64-windows.zip")
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn github_html_to_markdown(html: &str) -> String {
    let mut markdown = String::new();
    let mut chars = html.chars().peekable();

    while let Some(character) = chars.next() {
        if character == '<' {
            let mut tag = String::new();
            let mut closed = false;
            for next in chars.by_ref() {
                if next == '>' {
                    closed = true;
                    break;
                }
                tag.push(next);
            }
            if !closed {
                markdown.push('<');
                markdown.push_str(&tag);
                break;
            }

            let tag = tag.trim().to_ascii_lowercase();
            let closing = tag.starts_with('/');
            let tag_name = tag
                .trim_start_matches('/')
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .trim_end_matches('/');
            match (tag_name, closing) {
                ("h1", false) => markdown.push_str("\n\n# "),
                ("h2", false) => markdown.push_str("\n\n## "),
                ("h3", false) => markdown.push_str("\n\n### "),
                ("h1" | "h2" | "h3" | "p", true) => markdown.push_str("\n\n"),
                ("p", false) => markdown.push_str("\n\n"),
                ("li", false) => markdown.push_str("\n- "),
                ("li", true) => markdown.push('\n'),
                ("br", _) => markdown.push('\n'),
                _ => {}
            }
        } else {
            markdown.push(character);
        }
    }

    let decoded = decode_html_entities(&markdown);
    decoded
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn decode_html_entities(text: &str) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut remaining = text;

    while let Some(ampersand) = remaining.find('&') {
        decoded.push_str(&remaining[..ampersand]);
        remaining = &remaining[ampersand..];

        let entity_end = remaining
            .find(';')
            .filter(|end| *end <= 16)
            .map(|end| (end, &remaining[1..end]));
        let entity = entity_end.and_then(|(end, entity)| {
            let value = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" | "#39" => Some('\''),
                "nbsp" => Some(' '),
                numeric if numeric.starts_with("#x") || numeric.starts_with("#X") => {
                    u32::from_str_radix(&numeric[2..], 16)
                        .ok()
                        .and_then(char::from_u32)
                }
                numeric if numeric.starts_with('#') => {
                    numeric[1..].parse().ok().and_then(char::from_u32)
                }
                _ => None,
            }?;
            Some((end, value))
        });

        if let Some((end, value)) = entity {
            decoded.push(value);
            remaining = &remaining[end + 1..];
        } else {
            decoded.push('&');
            remaining = &remaining[1..];
        }
    }

    decoded.push_str(remaining);
    decoded
}
