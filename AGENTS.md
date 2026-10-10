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
- `src/update.rs`: query and parse the supported GitHub Release feed.
- `.github/workflows/release.yml`: validate tags and build/publish macOS ARM64 releases.
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
- Publish through GitHub Releases only. Push a `vMAJOR.MINOR.PATCH` tag that
  matches `Cargo.toml`; `.github/workflows/release.yml` validates the tag and
  changelog before publishing. Release notes come from that changelog section.
- The supported release target is Apple Silicon macOS (`aarch64-apple-darwin`).
  Keep the release asset name `Jump1-MAJOR.MINOR.PATCH-aarch64.dmg` aligned
  between `scripts/package-macos.sh`, the release workflow, and `src/update.rs`.
- The in-app updater reads the public `JCLee777/jump1` GitHub Releases feed and
  opens the matching DMG for manual installation. Keep its target gating,
  download naming, and displayed release history aligned with the published
  assets; updates are not installed silently.
- Do not add other release platforms or update feeds without updating this
  policy, the workflow, and the user-facing release documentation together.

## Commands

```bash
cargo fmt --check
cargo check
cargo run --release
```

Run formatting and compilation checks before handing off code changes. Cross-platform code must compile conceptually behind target-specific `cfg`; only claim runtime verification for tested systems.
