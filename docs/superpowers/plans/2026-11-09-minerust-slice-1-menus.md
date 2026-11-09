# MineRust Slice 1 — Real Menus in the Browser Tab — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Boot the real Horizon client (menus, fonts, UI) inside a browser tab, built on the fork's own crates with shims committed there, served from MineRust.

**Architecture:** Two repos. The fork `invisbleia/horizon-cracked` (local clone: `E:\horizon crack`) gains `cfg(target_family = "wasm")` shims behind a new `web` feature — VFS trait, main-thread execution, WebGPU backend, async boot. MineRust (this repo, `E:\MineRust`) holds the wasm entry crate, the build pipeline, the pack fetch/cache layer, and the site. Desktop paths are untouched: every shim is cfg-gated and CI-verified.

**Tech Stack:** Rust → wasm32-unknown-unknown, wgpu 30.0.1 (`webgpu` feature), winit 0.30.13, wasm-bindgen 0.2.130 + wasm-bindgen-futures 0.4.80, Cache Storage + in-memory archive for the pack, PowerShell dev server (`serve.ps1`), Vercel later.

**Spec:** `docs/design.md` in this repo (operator decisions and builder calls recorded there).

## Global Constraints

These hold for every task, in both repos:

- **Versions (spike-proven):** wasm-bindgen `0.2.130` (lockstep with `wasm-bindgen-cli 0.2.130`, prebuilt binary at `E:\MineRust\.tools\wasm-bindgen-0.2.130-x86_64-pc-windows-msvc\wasm-bindgen.exe`), wasm-bindgen-futures `0.4.80` (the 0.2 line is a fossil that breaks `js-sys`), wgpu `30.0.1`, winit `0.30.13`. Rust edition 2024, rust-version 1.95.
- **Spike-proven wgpu 30 wasm API:** `RequestAdapterOptions { power_preference, force_fallback_adapter, compatible_surface: Some(&surface), apply_limit_buckets: false }`; `SurfaceConfiguration` needs `color_space: wgpu::SurfaceColorSpace::Auto`; `get_current_texture()` returns the `CurrentSurfaceTexture` enum (`Success`/`Suboptimal` carry the texture); present with `queue.present(frame)`; canvas comes via `winit::platform::web::WindowAttributesExtWebSys::with_canvas`; `run_app(&mut app)` never returns on web (winit throws a self-labeled "not actually an error" exception for control flow) — no code may rely on it returning.
- **Desktop invariance:** every fork change is `cfg(target_family = "wasm")` (or feature-gated so default desktop builds are byte-identical in behavior). Verification is the fork's CI on push — **no local `cargo test` runs** (operator rule); local checks are `cargo check --target wasm32-unknown-unknown` only.
- **Attribution:** every commit in both repos ends with the trailer `Co-Authored-By: GLM-5.3-Flash <noreply@zhipu.ai>`.
- **Fork pin:** MineRust depends on the fork by git dependency pinned to an exact commit in `Cargo.lock`; each fork task ends by pushing and recording the new pin in the MineRust task that follows. Local iteration may add an uncommitted `[patch]` to path deps.
- **No game files in git:** the prepared pack (`horizon-prepare` output) never enters either repo; MineRust's site serves it from an upload folder.
- **Operator decisions in force:** public asset serving; Vercel later, local-first; degraded visuals acceptable; web menus hide controllers, narrator, shader packs, and multiplayer/Realms; no multiplayer.
- **Threading:** slice 1 is single-threaded on the main thread — `std::thread` spawns in boot paths are shimmed inline behind cfg(wasm). Atomics/web-workers for the server thread and streaming pool are a later slice's plan.

## Review Focus

Failure modes the tasks' tests don't already pin, most likely first:

1. **Desktop regression from a shim** — a fork edit leaks out of cfg(wasm) and the desktop client changes behavior. Expect: fork CI stays green on every push; any red run blocks the next MineRust pin.
2. **Boot on a browser with no WebGPU** — the page must print a clear console message ("WebGPU required — use Chrome/Edge 113+"), never a wasm panic with no context.
3. **First load with a cold cache** — pack fetch shows progress and survives a mid-download reload; a partial Cache Storage entry must never be treated as complete (manifest hash checked before use).
4. **Code after the event loop** — anything placed after `run_app` on web silently never runs; cleanup and save-on-exit hooks must hang off winit events or `beforeunload`, not the return path.
5. **Pack/wasm version skew** — a stale cached pack with a new wasm build must be detected via the manifest version and re-fetched, not render garbage.

---

### Task 1: Fork — WebGPU backend plumbing for wasm

**Files:**
- Modify: `crates/horizon-client-render/Cargo.toml` (wgpu dep: add `[target.'cfg(target_family = "wasm")'.dependencies]` with `features = ["webgpu"]`)
- Modify: `crates/horizon-client-render/src/gfx.rs:1916-1937` (`backend_order`)
- Modify: `crates/horizon-client-render/src/gfx.rs` (`open_device` at 2080, `device_search` at 2123 — env-overrides are no-ops on wasm)

**Interfaces:**
- Produces: on wasm, `Renderer::new` resolves an adapter with `Backends::BROWSER` and creates surfaces from the winit canvas. Signature unchanged: `pub async fn new(window: Arc<Window>, offscreen: bool, ..., api: GraphicsApi, ...) -> Result<Self>` (`gfx.rs:2524`).

- [ ] **Step 1:** In `gfx.rs`, make `backend_order` return `vec![wgpu::Backends::BROWSER]` first thing under `#[cfg(target_family = "wasm")]`, keeping the native body untouched.
- [ ] **Step 2:** Add the wasm-only wgpu dep block to `horizon-client-render/Cargo.toml` (same version, `default-features = false`, features `["std", "parking_lot", "webgpu", "wgsl"]`; the spike proved this exact set compiles).
- [ ] **Step 3:** Guard `HORIZON_GRAPHICS_API`/`WGPU_BACKEND` env parsing behind `#[cfg(not(target_family = "wasm"))]` (env vars don't exist on web).
- [ ] **Step 4:** Verify: `cargo check -p horizon-client-render --target wasm32-unknown-unknown` succeeds; push fork; **fork CI green** (desktop unchanged).
- [ ] **Step 5:** Commit (fork): `wasm: WebGPU backend plumbing behind cfg(target_family = "wasm")`.

### Task 2: Fork — gate desktop-only dependencies out of wasm builds

**Files:**
- Modify: `crates/horizon-client/Cargo.toml` (target-gate `ureq` (line 80), `fs4` (83); `gilrs`/`horizon-dualsense`/`horizon-friends` are already optional — the `web` feature set excludes them)
- Modify: `crates/horizon-client-lib/Cargo.toml` (target-gate `ureq` (87), `fs4` (90), `mio` (72) to `cfg(not(target_family = "wasm"))`)
- Modify: their call sites — session join / heartbeat HTTP (`ureq`), `save.rs:148` `SessionLock::acquire` + disk-space estimate (`fs4`), `lan/io_pool.rs` (`mio`)
- Create: `crates/horizon-client/src/client_lib.rs` additions behind `#[cfg(target_family = "wasm")]`: a no-op `SessionLock::acquire` (web: single tab, no flock) and a stub lan IO pool

**Interfaces:**
- Produces: `SessionLock::acquire(&self) -> ...` returns the same shape as desktop but always succeeds on wasm; `--web` feature set on horizon-client: `default = ["audio"]`-equivalent minus `controllers`/`discord`/`friends`/`heartbeat`.
- Consumes: nothing from later tasks; unblocks Task 5's wasm check of the full client.

- [ ] **Step 1:** Target-gate the three deps in both Cargo.tomls exactly as listed above.
- [ ] **Step 2:** At each call site, add the cfg twin: wasm variant keeps signatures, drops the operation (HTTP: `Err(UsageError::...)`-style result or feature-absent path the code already has; SessionLock: success; disk space: `u64::MAX`).
- [ ] **Step 3:** Verify: `cargo check -p horizon-client --target wasm32-unknown-unknown --no-default-features` progresses past dependency resolution (later tasks finish making it compile); desktop `cargo check -p horizon-client` unaffected; push; **fork CI green**.
- [ ] **Step 4:** Commit (fork): `wasm: gate ureq/fs4/mio and their call sites behind cfg(not(wasm))`.

### Task 3: Fork — the VFS trait at pack and options read points

**Files:**
- Create: `crates/horizon-core/src/fs.rs` — `pub trait Vfs: Send + Sync` with `fn read(&self, path: &str) -> std::io::Result<Vec<u8>>`, `fn exists(&self, path: &str) -> bool`, `fn write(&self, path: &str, bytes: &[u8]) -> std::io::Result<()>`; `pub fn fs() -> &'static dyn Vfs`; `pub fn set_fs(v: &'static dyn Vfs)` (a `OnceLock`; default `DesktopFs` over `std::fs`)
- Modify: `crates/horizon-core/src/lib.rs` (add `pub mod fs;`)
- Modify: `crates/horizon-client-render/src/assets.rs:186` (`Assets::load(root: &Path)` reads through `horizon_core::fs::fs()`)
- Modify: `crates/horizon-client/src/game/options.rs:93-99` (`options_path`/`Options::read` — read/write through the Vfs)
- Modify: `crates/horizon-client-render/src/settings.rs:340` and `:678` (`Settings::load`/`save`)

**Interfaces:**
- Produces: `horizon_core::fs::{Vfs, fs, set_fs, DesktopFs}` — paths are game-shaped (`".local/horizon-client/26.3/facts.json"`, `"options.txt"`), desktop `DesktopFs` resolves them exactly as today (current-dir-relative), so MineRust's `WebFs` (Task 7) can map them onto its archive + OPFS.
- Consumes: nothing; this is the seam the whole web port hangs on.

- [ ] **Step 1:** Write `fs.rs` with the trait, the `OnceLock`, and `DesktopFs` (30 lines, `std::fs` behind it).
- [ ] **Step 2:** Swap the listed read/write call sites to `horizon_core::fs::fs()` — read/write calls keep their existing signatures; `PathBuf` inputs become `String` game paths at the boundary.
- [ ] **Step 3:** Verify: `cargo check --workspace --target wasm32-unknown-unknown -p horizon-core -p horizon-client-render` and push; **fork CI green** (the desktop tests that read options/pack files prove `DesktopFs` behaves identically).
- [ ] **Step 4:** Commit (fork): `wasm: VFS seam in horizon-core, desktop routes through std::fs`.

### Task 4: Fork — main-thread execution shims for boot threads

**Files:**
- Modify: `crates/horizon-threads/src/jobs.rs:201-224` (`Pool::new` — wasm: zero background workers, `spawn` runs the job inline)
- Modify: `crates/horizon-threads/src/lib.rs:368` (`configure_allocator` — verify inert on wasm; mimalloc is C and must not be selected)
- Modify: `crates/horizon-client/src/game/startup_menu.rs:366` (`spawn_loader` "resource-loader" thread — wasm: call the body inline before the wait loop)
- Modify: `crates/horizon-client/src/game.rs:17998-18006` (scoped `"load-{name}"` workers — wasm: sequential inline loop, same results channel)
- Modify: `crates/horizon-client/src/game.rs:18518-18524` (`write_prepared_files` thread "Prepared data file" — wasm: skip; the pack is served, never written)

**Interfaces:**
- Produces: on wasm, all boot resource loading completes synchronously inside the first `App::Loading` frame budget; `horizon_threads::Pool::new(threads, name)` is callable on wasm with `threads` ignored.
- Consumes: nothing.

- [ ] **Step 1:** Add `#[cfg(target_family = "wasm")]` inline-run twins at each listed spawn site; the wasm branch must produce the same data structures the desktop branch hands to the loading screen (no behavior visible to callers changes).
- [ ] **Step 2:** Confirm `configure_allocator` never selects mimalloc on wasm (add the cfg arm returning early if needed).
- [ ] **Step 3:** Verify: `cargo check -p horizon-client --target wasm32-unknown-unknown` — the client lib now compiles further; push; **fork CI green**.
- [ ] **Step 4:** Commit (fork): `wasm: inline main-thread execution for boot threads`.

### Task 5: Fork — the web entry and async renderer boot

**Files:**
- Create: `crates/horizon-client/src/web.rs` — `pub fn window_attrs()` (winit attributes + `with_canvas` from `document.getElementById("game")`), behind `#[cfg(target_family = "wasm")]`
- Modify: `crates/horizon-client/src/game.rs:19268-19304` (`run_loading`) and `game\loading_window.rs:26-130` (`build_renderer`/`loading_renderer`) — wasm variant: replace `pollster::block_on(Renderer::new(...))` with `wasm_bindgen_futures::spawn_local` that builds the renderer and flips an `Option<Renderer>` the `App::Loading` redraw path consumes; frames before readiness draw the loading background only
- Modify: `crates/horizon-client/src/game.rs:18852` (window creation — wasm uses `web::window_attrs()`)
- Modify: `crates/horizon-client/src/lib.rs` (add `pub mod web;` and `pub fn main_web()` → the same path `entry::main_with(Mods)` takes, with mods empty)
- Modify: `crates/horizon-client/Cargo.toml` — add `wasm-bindgen`, `wasm-bindgen-futures` to a wasm target dep block

**Interfaces:**
- Produces: `horizon_client::main_web()` — callable from `#[wasm_bindgen(start)]`; expects a canvas `id="game"`; never returns (event loop). Desktop `entry::main()` untouched.
- Consumes: Task 1 (BROWSER backend), Task 3 (`set_fs` is called here before any read — the entry accepts a `&'static dyn Vfs` argument: `pub fn main_web(vfs: &'static dyn horizon_core::fs::Vfs)`).

- [ ] **Step 1:** Add `main_web(vfs)` in `lib.rs`: `set_fs(vfs)`, then the `run_with()` body's wasm twin (options read → `run_loading` → `EventLoop::new()?.run_app(&mut app)`).
- [ ] **Step 2:** Make the Loading-state renderer construction async on wasm (`spawn_local`, `Option<Renderer>` handed to redraw) — desktop keeps `pollster::block_on` verbatim.
- [ ] **Step 3:** Verify: `cargo check -p horizon-client --target wasm32-unknown-unknown` **passes clean** (the full client lib now compiles for wasm); push; **fork CI green**; record the fork commit SHA.
- [ ] **Step 4:** Commit (fork): `wasm: main_web entry, async renderer boot, canvas window attrs`.

### Task 6: MineRust — the real client crate replaces the spike

**Files:**
- Create: `client/` (replaces `spike/`; delete the spike crate) — `Cargo.toml` with git deps: `horizon-client = { git = "https://github.com/invisbleia/horizon-cracked", rev = "<Task 5 SHA>", default-features = false }` (no `audio` — cpal's web backend is unverified and belongs to slice 4), `horizon-core` (same rev), `wasm-bindgen 0.2`, `wasm-bindgen-futures 0.4`, `console_error_panic_hook`
- Create: `client/src/lib.rs` — `#[wasm_bindgen(start)] pub async fn run()`: panic hook, then `horizon_client::main_web(&WEB_FS)` (main_web calls `set_fs` itself; the Vfs impl arrives in Task 7 — this task ships a temporary read-only in-memory Vfs so the entry compiles)
- Create: `client/build.ps1` — `cargo build --target wasm32-unknown-unknown --release` then `.tools\wasm-bindgen-0.2.130-x86_64-pc-windows-msvc\wasm-bindgen.exe --target web --out-dir web/pkg client/target/.../client.wasm`; if getrandom ≥ 0.3 is in the tree, add `RUSTFLAGS = '--cfg getrandom_backend="wasm_js"'` (verify with `cargo tree -i getrandom`; if it resolves 0.2 instead, enable its `js` feature on the dep instead)
- Modify: `web/index.html` — canvas `id="game"` full-viewport, `type="module"` init from `./pkg/client.js`, a `<noscript>`/fallback div

**Interfaces:**
- Consumes: fork `main_web(vfs)` and `horizon_core::fs::{Vfs, set_fs}` at the Task 5 pin SHA.
- Produces: `http://127.0.0.1:8765` shows the game's loading screen state (renderer up, pack reads failing gracefully into the loading screen until Task 7 feeds it).

- [ ] **Step 1:** Delete `spike/`, create `client/` with the manifest, entry, and `build.ps1`; commit the fork rev pin.
- [ ] **Step 2:** Verify: `.\client\build.ps1` produces `client/web/pkg/{client.js, client_bg.wasm}`; reload the tab — console shows the game's own logging banner and no panic (missing-pack path is exercised in Task 7).
- [ ] **Step 3:** Commit (MineRust): `client: real wasm entry on fork @ <sha>`.

### Task 7: MineRust — pack fetch, cache, and the WebFs

**Files:**
- Create: `pack/build-pack.ps1` — runs `cargo run -p horizon-prepare -- --prepare --root <E:\horizon crack>` once, zips `.local/horizon-client/26.3` to `site-public/pack-<OUTPUT_VERSION>.zip` (zip's file list + each entry's SHA-1 into `site-public/pack-manifest.json` with a `version` field), and prints the upload folder. Never committed: `site-public/` is gitignored.
- Create: `client/src/webfs.rs` — `pub struct WebFs` implementing `horizon_core::fs::Vfs`: boot fetches `pack-manifest.json`, then the pack zip once into Cache Storage (keyed by manifest `version`; hash-checked), holds the archive in memory, serves `read/exists` from it; game-shaped paths map onto archive entries (`"facts.json"` → `26.3/facts.json` prefix inside the zip); `write` queues and flushes to OPFS (`options.txt` et al.), `read` prefers the OPFS-written copy over the pack copy.
- Modify: `client/src/lib.rs` — construct `static WEB_FS: WebFs` and pass it to `main_web`

**Interfaces:**
- Consumes: Task 3's `Vfs` (exact trait); Task 6's entry.
- Produces: `WebFs` — the only Vfs implementation MineRust ships; slice 3 (world saves) reuses it unchanged.

- [ ] **Step 1:** Run `pack/build-pack.ps1`; record pack size (it drives the progress screen wording).
- [ ] **Step 2:** Implement `WebFs` fetch/cache/hash-check with a progress callback wired to the loading screen's existing progress reporting.
- [ ] **Step 3:** Verify in the tab: menus draw with the pack loaded (fonts render, panorama/backdrop shows); second load with cold network (dev tools offline + reload) still boots from Cache Storage; `Review Focus` item 3's mid-download reload never poisons the cache (hash check path).
- [ ] **Step 4:** Commit (MineRust): `client: WebFs — pack fetch, Cache Storage, OPFS write-back`.

### Task 8: Slice proof — menus, with the operator's hidden entries actually hidden

**Files:**
- Modify: `crates/horizon-client/src/title.rs` (fork) — `Title::new(saves, resources)`/`draw_with_items` (757/2505): on wasm, the menu item list omits Multiplayer/Realms and the options pages omit controllers/narrator/shader-pack entries (`screen_draw.rs::Draw` items built under `#[cfg(not(target_family = "wasm"))]`)
- Modify: `client/web/index.html` (MineRust) — the fallback div doubles as the capability gate: if `navigator.gpu` is missing, show "WebGPU required" instead of initializing

**Interfaces:**
- Consumes: Tasks 5–7 (booted menus).

- [ ] **Step 1:** Fork: hide the four entry classes on wasm (controllers, narrator, shader packs, multiplayer/Realms); push; **fork CI green**; record SHA; bump the pin in `client/Cargo.toml` (MineRust commit).
- [ ] **Step 2:** MineRust: capability gate before `init()`.
- [ ] **Step 3:** Verify in the tab: title screen draws; the four hidden classes are absent from the menus; clicking Options opens the real options screen; no console errors beyond the known winit control-flow exception. Screenshot as the slice record.
- [ ] **Step 4:** Commit (fork): `wasm: web menus hide controllers, narrator, shader packs, multiplayer`; Commit (MineRust): `client: capability gate; pin fork @ <sha>`.

## Later slices (their own plans, informed by this one)

- **Slice 2:** world creation + chunk streaming + break/place — atomics/web-workers pipeline, the integrated server thread, input/pointer-lock shims.
- **Slice 3:** world save/load through `WebFs` (regions, NBT, level.dat in OPFS) — reuses Task 7's impl.
- **Slice 4:** cpal Web Audio, Vercel deploy (`vercel.json` COOP/COEP + MIME), pack upload script, performance pass, the full gate page.
