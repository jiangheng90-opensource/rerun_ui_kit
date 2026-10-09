# Porting Guide

How `rerun_ui_kit` is produced from upstream [`re_ui`](https://github.com/rerun-io/rerun/tree/main/crates/viewer/re_ui),
and how to re-port when a new upstream version is needed.

## 1. Port snapshot

| | |
|---|---|
| Upstream repo | `rerun-io/rerun` |
| Upstream pin | tag `0.30.2` |
| Primary source | `crates/viewer/re_ui` |
| Secondary sources | `crates/utils/re_format` (→ `src/format/`), `re_log_types` + `re_types_core` time types (→ `src/time_types.rs`), `re_log::LogMsg` (→ `src/notifications.rs`) |
| Port commit | `6fe0756` "feat: initial vendored port of re_ui 0.30.2 without the rerun data stack" |

**Strategy: verbatim vendoring + minimal surgery.** Files are copied file-for-file
with identical names and module structure. More than 95% of the lines are
byte-identical to upstream; the entire port is ~490 changed lines, all of which
fall into the small set of mechanical transformations listed below.
**The diff against upstream *is* the port ledger** — keep it small and boring.

## 2. File mapping

| This crate | Upstream |
|---|---|
| `src/*.rs`, `src/egui_ext/`, `src/list_item/` | `crates/viewer/re_ui/src/…` (1:1) |
| `src/format/{mod,duration,plural,time}.rs` | `crates/utils/re_format/src/{lib,duration,plural,time}.rs` |
| `src/time_types.rs` | hand-minimized port of `re_log_types` + `re_types_core::datatypes` |
| `data/` | `crates/viewer/re_ui/data/` verbatim, **except** `logo_dark_mode.png` / `logo_light_mode.png` (rerun brand assets, removed) |
| `examples/rerun_ui_kit_example/` | `crates/viewer/re_ui/examples/re_ui_example/` (adapted: `re_quota_channel::send_crossbeam` → `crossbeam` `try_send`, `re_ui::` → `rerun_ui_kit::`) |

## 3. Dependency substitution table

Every upstream `re_*` dependency is either substituted by a public crate with a
compatible API, vendored as a shim module, or deleted.

| Upstream | Replacement | How |
|---|---|---|
| `re_log` | `log` | path substitution: `re_log::info!` → `log::info!`, `re_log::debug_assert!` → `debug_assert!` |
| `re_mutex` | `parking_lot` | path substitution: `use re_mutex::Mutex` → `use parking_lot::Mutex` |
| `re_tracing` | — | delete every `re_tracing::profile_function!();` line |
| `re_format` | `src/format/` | vendored near-verbatim; dropped `format_f16` and other unused items |
| `re_log_types` / `re_types_core` | `src/time_types.rs` | minimized rewrite (see §5.2) |
| `re_entity_db`, `re_log_types` (paths) | — | deleted: `syntax_highlighting.rs` impls for rerun data types (§6.2) |
| `re_analytics` | — | deleted: `analytics` feature + `re_analytics::record(...)` call site in `ui_ext.rs` |
| `eframe` (lib) | `egui` | path substitution `eframe::emath/epaint` → `egui::…`; one API break (§6.1) |
| `getrandom` | — | `filter_widget.rs` session id: time + atomic counter instead of RNG |
| `ahash` | `std` | `ahash::HashMap` → `std::collections::HashMap` in `filter_widget.rs` |
| `notify` | — | deleted `#[cfg(hot_reload_design_tokens)]` branches (dev-only feature) |
| `objc2-app-kit`, `raw-window-handle` | — | deleted with macOS traffic-light measuring (§6.1) |
| `re_build_tools` (build dep) | — | no `build.rs` needed |
| `re_quota_channel` (dev) | `crossbeam` (already upstream) | example: `re_quota_channel::send_crossbeam(...)` → plain `try_send(...)` |
| `rand` (dev) | — | dropped; not needed by the ported code |
| `egui_kittest` (dev) | kept | snapshot tests retained but `#[ignore]`d (§7) |

Kept as-is (public crates): `anyhow`, `egui`, `egui_commonmark`, `egui_extras`,
`itertools`, `jiff`, `num-traits`, `parking_lot`, `ron`, `serde`, `smallvec`,
`strum`, `strum_macros`, `sublime_fuzzy`, `url`, `web-time`.

## 4. Mechanical transformation rules

These are the only edit classes applied to vendored files. A re-port is mostly
re-applying R1–R6 to a fresh copy.

- **R1 — path substitution** (per §3): `re_log::` → `log::`,
  `re_format::` → `crate::format::`, `re_log_types::…` → `crate::time_types::…`,
  `eframe::emath/epaint` → `egui::…`.
- **R2 — delete cfg/feature blocks**: `#[cfg(feature = "analytics")]`,
  `#[cfg(feature = "testing")] pub mod testing`, all
  `#[cfg(hot_reload_design_tokens)]` branches (keep the `OnceLock` static-loading
  branch), all `#[cfg(target_os = "macos")]` traffic-light code.
- **R3 — delete single lines**: `re_tracing::profile_function!();`,
  the two `egui_ctx.include_bytes("bytes://logo_…", …)` calls in
  `apply_style_and_install_loaders`.
- **R4 — lint attribute downgrade**: `#[expect(clippy::…)]` → `#[allow(clippy::…)]`.
  Upstream's strict workspace clippy config is not vendored, so `expect` would
  warn as unfulfilled. (`design_tokens.rs`, `ui_ext.rs`, `command_palette.rs`)
- **R5 — doctest rename**: `re_ui::…` → `rerun_ui_kit::…` in doc examples.
- **R6 — explicit f32 literals** *(post-port fixup, commit `971b7e0`)*:
  `1.0` → `1.0_f32` in `Stroke::new` etc. for rustc ≥1.97
  (`float_literal_f32_fallback`). Re-apply if new upstream code hits the lint.

## 5. Shim modules

### 5.1 `src/format/` — vendored `re_format` subset

Near-verbatim copy of `re_format`'s `lib.rs` (→ `mod.rs`), `duration.rs`,
`plural.rs`, `time.rs`. Only unused items were dropped (`format_f16`, …).
Treat as vendored code: re-copy wholesale on re-port.

### 5.2 `src/time_types.rs` — minimized rewrite (the only semantic port)

Ports from `re_log_types` / `re_types_core` **only the API surface `re_ui`
actually uses**. Key simplifications vs upstream:

- `TimeInt` is a plain `i64` newtype instead of `NonMinI64`-based;
  `MIN = i64::MIN + 1` preserves upstream's "no `i64::MIN`" semantics.
  The `STATIC` sentinel is gone.
- All arrow / serde / `TryFromIntError` machinery removed.
- `Timestamp::now()` uses `web_time` (wasm-safe).

Kept API surface (check this list against upstream changes on re-port):
`TimeInt` (`MIN/MAX/ZERO`, `new_temporal`, `from_secs`, `saturated_temporal_i64`,
`as_i64`, `as_f64`), `TimeRange`, `TimeRangeBoundary` (`AT_CURSOR`,
`start/end_boundary_time`), `AbsoluteTimeRange` (`EMPTY`, `EVERYTHING`, `new`,
`min/max`, `contains`, `from_relative_time_range`), `TimeType` (`format*`),
`Duration`, `Timestamp`, `TimestampFormat`(`Kind`).

Consumers of this shim — re-check these files first on every re-port:
`time.rs`, `time_drag_value.rs`, `relative_time_range.rs`, `notifications.rs`.

### 5.3 `notifications::LogMsg` — local copy of `re_log::LogMsg`

A 3-field struct (`level: log::Level`, `target`, `msg`) replacing
`re_log::LogMsg` in the public `NotificationUi::add_log` API.

## 6. Intentional deviations (cannot be re-applied mechanically)

These change behavior or public API. Each needs a conscious decision on re-port.

### 6.1 Break #1 — `top_bar_style` drops `eframe::Frame` (macOS traffic lights)

Upstream measures the native macOS traffic-light (close/minimize/maximize)
button area so the top bar can share space with it in full-size windows:

```
eframe::Frame.window_handle() → raw_window_handle::AppKit
  → objc2_app_kit NSWindow.standardWindowButton(...).frame()
  → WindowChromeMetrics { traffic_lights_size }  (fallback: vec2(64.0, 24.0))
  → ÷ egui zoom_factor → TopBarStyle { height, indent }
```

The port cuts the chain at the entry point: the `_frame: &eframe::Frame`
parameter is removed and the function returns fixed tokens-based values with
`indent: 0.0`:

```rust
fn top_bar_style(&self, _style_like_web: bool) -> TopBarStyle
```

Consequences:

- Deleted `src/egui_ext/mac_traffic_light_sizes.rs` (the crate's only `unsafe`
  code — enables `#![forbid(unsafe_code)]`), plus the `objc2-app-kit` and
  `raw-window-handle` macOS dependencies.
- `fullsize_content` / `native_window_bar` / `CUSTOM_WINDOW_DECORATIONS` in
  `lib.rs` are kept (they only need `egui::os::OperatingSystem`), but nothing
  in this crate consumes traffic-light metrics anymore.
- Cost shifted to consumers: an app that wants rerun-style full-size window
  chrome on macOS must reserve the ~78pt left indent itself. Acceptable for
  bevy/winit use cases.

On re-port: do **not** overwrite `context_ext.rs` blindly — re-apply this
parameter removal by hand if upstream changed the function.

### 6.2 Break #2 — syntax highlighting for rerun data types removed

All `SyntaxHighlighting` impls for `EntityPath(Part)`, `Instance(Path)`,
`ComponentPath`, `ComponentDescriptor`, `ComponentIdentifier`, `ComponentType`,
`ArchetypeName`, and the `InstanceInBrackets` helper were deleted from
`syntax_highlighting.rs` — they are orphan impls on rerun's data model and pull
in `re_entity_db` / `re_log_types`. The generic builder and primitive impls
(`f32`…`usize`, `bool`, via `crate::format`) are kept.

### 6.3 Other behavioral deltas

- `filter_widget.rs`: session id from `web_time` nanos ⊕ atomic counter
  (documented in-code) instead of `getrandom`.
- `hot_reload_design_tokens.rs`: only the non-hot-reload path remains; the file
  is kept under its upstream name so `lib.rs` structure matches.
- Brand logos (`logo_dark/light_mode.png`) and their `include_bytes`
  registration removed.
- Local feature on top of the port (commit `9a55e83`): `modal.rs` slim title
  bar, non-selectable title. **Not** an upstream mapping — do not "fix" it back
  during re-ports.

## 7. Tests

- `egui_kittest` snapshot tests are kept in place but marked
  `#[ignore = "snapshot reference images were not vendored from the rerun repo"]`
  (`alert.rs`, `combo_item.rs`, `relative_time_range.rs`) — the reference PNGs
  live in the rerun repo and are not published with the crate. Keeping the test
  bodies preserves the file structure for future re-ports.
- Upstream `src/testing.rs` (feature `testing`, pulls `egui_kittest` +
  `re_analytics/testing`) was dropped entirely.

## 8. Re-port procedure (next upstream version)

1. Fetch the new upstream tree: `crates/viewer/re_ui/{src,data,examples}` and
   `crates/utils/re_format/src` at the target tag.
2. Diff upstream old-tag → new-tag first; files unchanged upstream need no work.
3. Overwrite the verbatim files (anything with an empty diff in this port:
   `command.rs`, `icons.rs`, `button.rs`, `menu.rs`, `drag_and_drop.rs`,
   `text_edit.rs`, `section_collapsing_header.rs`, `egui_ext/*` (except
   `mod.rs`), `data/icons/`, …) with the new upstream versions.
4. For every other file, re-apply rules **R1–R6** (§4).
5. Hand-check the touch-points of the shims and breaks:
   - `time.rs`, `time_drag_value.rs`, `relative_time_range.rs` against §5.2;
   - `syntax_highlighting.rs` against §6.2;
   - `context_ext.rs` against §6.1;
   - `lib.rs`: re-delete logos / hot-reload / `testing` module / `profile_function!`.
6. Re-sync `src/format/` wholesale from `re_format`; delete newly-unused items
   only if they fail to compile.
7. Update the version pin in `Cargo.toml` (`description`), `README.md`, and the
   `lib.rs` doc header. Bump egui/eframe/egui_extras/egui_kittest/egui_tiles to
   the versions upstream's workspace uses at that tag.
8. Verify: `cargo test`, `cargo run --example rerun_ui_kit_example`,
   `cargo check --target wasm32-unknown-unknown`, `cargo clippy`.

## 9. License & attribution

Same license as upstream: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
The bundled Inter font is OFL-1.1 (`data/OFL.txt`). Every shim/ported file
carries a header comment citing its upstream crate, the pinned version, and the
license — keep these headers accurate when bumping the pin.
