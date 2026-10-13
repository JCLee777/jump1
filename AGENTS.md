# Repository guide

## Product

Jump1 是轻量桌面启动器与工具箱。命令面板是入口，应用启动是内置功能之一。插件 API 尚未确定。

## Code map

- `src/main.rs`: keep as a thin binary entry.
- `src/lib.rs`: initialize GPUI and create the main window.
- `src/app.rs`: own UI state, input handling, and rendering.
- `src/plugin/launcher/search.rs`: define launcher search data and pure matching logic.
- `src/icon.rs`: own the built-in SVG catalog and icon rendering helpers.
- `src/i18n.rs`: own locales and all product-facing text.
- `src/platform.rs`: contain OS-specific discovery and process launching.
- `src/plugin/`: contain built-in plugin identities and isolated implementations.
- `src/plugin/protocol.rs`: contain shared plugin-host result and action types.
- `src/state.rs`: contain persisted application-wide preferences only.
- `src/theme.rs`: own application metrics and resolve semantic design tokens.
- `src/update.rs`: query the latest supported GitHub Release and parse its notes.
- `build.rs` and `assets/app-icon.ico`: embed the Windows executable icon and version resource.
- `installer/windows.nsi`: build the per-user Windows setup program with Start Menu and uninstall entries.
- `.github/workflows/release.yml`: validate tags and build/publish macOS ARM64 and Windows x86_64 releases.
- `CHANGELOG.md`: record user-facing changes for each version.
- `docs/architecture.md`: record module boundaries and extension decisions.

## Boundaries

- Keep system-specific `cfg` branches in `platform.rs`.
- Keep `plugin/launcher/search.rs` independent of GPUI and subprocess execution.
- Keep localized text out of command and platform data.
- Keep built-in icons as project-owned SVG files under `assets/icons`; do not
  introduce a second bundled icon library.
- Resolve native application icons in `platform.rs` and preserve a built-in SVG
  fallback for missing or invalid images.
- Keep plugin storage, limits, and pure filtering inside each plugin module.
- Keep cross-plugin preferences in `state.rs`; do not move plugin data there.
- Treat built-in plugin types as implementation details until the external plugin
  contract is defined.
- Use semantic colors, spacing, radius, and typography from `theme.rs`; do not add
  presentation constants to feature views.
- Do not add product logic to `main.rs` or bootstrap logic to `app.rs`.
- Do not present internal command types as a stable plugin API.
- Update documentation when module ownership or implemented status changes.

## Release policy

- `Cargo.toml` is the source of truth for the application version. Use stable
  `MAJOR.MINOR.PATCH` versions and keep `Cargo.lock` in sync.
- Every release must have a matching `CHANGELOG.md` section headed
  `## [MAJOR.MINOR.PATCH]` with user-facing release notes. Do not publish a
  version without that entry.
- Each release note section must include non-empty `### 简体中文` and
  `### English` sections. The release workflow validates both; the in-app
  updater displays the section matching the selected app locale and falls back
  to the available translation or original notes.
- Publish through GitHub Releases only. Push a `vMAJOR.MINOR.PATCH` tag that
  matches `Cargo.toml`; `.github/workflows/release.yml` validates the tag and
  changelog before publishing. Release notes come from that changelog section.
- Treat a published release and its tag as immutable. Correct a published build
  with a new patch version rather than moving its tag or replacing its asset.
- Supported release targets are Apple Silicon macOS (`aarch64-apple-darwin`) and
  Windows x86_64 MSVC (`x86_64-pc-windows-msvc`). Keep `Jump1-MAJOR.MINOR.PATCH-aarch64.dmg`,
  `Jump1-MAJOR.MINOR.PATCH-x86_64-windows.zip`, and
  `Jump1-MAJOR.MINOR.PATCH-x86_64-windows-setup.exe` aligned between the packaging
  scripts and release workflow. The in-app updater continues to use only the
  versioned Windows ZIP, whose contents are `Jump1.exe` plus `README-Windows.txt`;
  the setup EXE installs per-user, adds Start Menu and uninstall entries, and is
  intended for initial setup or manual upgrades. After an in-app update, refresh
  the registered version when the setup program's per-user uninstall entry exists.
  `build.rs` embeds the app icon and executable version metadata from the project
  icon and Cargo version.
- The in-app updater reads the public `JCLee777/jump1` GitHub Releases feed and
  downloads the matching platform package. The user must select Install and
  Relaunch before the updater replaces the current app. Restrict downloads to
  the matching official release asset, validate the staged app identity and
  version, and retain the GitHub release page as a manual fallback. On Windows,
  validate the ZIP file list and staged executable version before replacement;
  restore the previous executable if replacement fails. Keep target gating and
  download naming aligned with published assets, show localized notes, and
  never install or relaunch silently.
- Update this policy, workflow, and user-facing release documentation together
  whenever adding another release platform or update feed.

## Commands

```bash
cargo fmt --check
cargo check
cargo run --release
```

Run formatting and compilation checks before handing off code changes. Cross-platform code must compile conceptually behind target-specific `cfg`; only claim runtime verification for tested systems.
