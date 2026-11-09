# MineRust — the Browser Client — Design

Date: 2026-11-09
Status: draft, awaiting operator approval
Project: **MineRust** — this repository — the WebAssembly (browser) client of
Horizon, built on the operator's fork `invisbleia/horizon-cracked`.

## Intent and why

Players should open a tab on the operator's site and play the full
single-player Horizon game (Rust Minecraft Java 26.3) right there — no
download, no install — and their worlds should persist between visits. This
is the browser step of the fork's "play easily, like TLauncher" identity.
Multiplayer is explicitly out of scope for this project.

## Goals

- The game boots in a browser tab: menus, world creation, playing, saving.
- Worlds, playerdata, and options save to the browser and survive reloads.
- Vanilla assets are served by the site and cached after a one-time download.
- Runs on current Chrome/Edge/Firefox/Safari with WebGPU; other browsers get a
  clear "can't run this" page, not a broken page.
- The desktop client keeps building and behaving exactly as today.

## Non-goals

- Multiplayer of any kind (LAN, dedicated, relay) — later project.
- The GPUI launcher on the web — the launcher stays a desktop program.
- Touch/mobile controls.
- Mods, cosmetics, telemetry ingest in the browser build.

## Anti-goals (failure even if it works)

- Forking gameplay logic: MineRust must reuse the game's own libraries, not a
  copy that drifts.
- Regressing the desktop build or its CI.

## Constraints

- Browsers cannot open raw TCP: fine while multiplayer is out of scope, but
  the design must not make the future relay harder.
- Threads on wasm need SharedArrayBuffer, i.e. cross-origin isolation
  (COOP/COEP headers). GitHub Pages cannot set headers; Cloudflare Pages can
  (`_headers` file). The MineRust site hosts on Cloudflare Pages;
  `coi-serviceworker` is the fallback shim for static hosts.
- wgpu 30.0.1 (the game's renderer, backend-switched per platform by
  features) gains a `webgpu` feature; winit 0.30.13 supports the web platform
  natively.
- `pollster::block_on` cannot run on the wasm main thread; any blocking call
  in the entry path needs a web-shaped equivalent.
- cpal 0.16 (the game's optional `audio` feature) ships a Web Audio backend
  on wasm.
- The Horizon repo never commits Mojang files: the MineRust site serves the
  prepared vanilla asset pack built by the operator's own machine
  (`horizon-prepare`); neither repository carries it.
- WebAssembly size/compile limits favor keeping the asset pack out of the
  wasm binary; it loads and caches separately.

## Approaches considered

1. **Real port — this MineRust repository, building on the game's libraries
   from the fork, with browser shims behind `cfg(target_family = "wasm")`.**
   The desktop client is untouched; MineRust owns the entry, the canvas loop,
   the OPFS file layer, the asset fetch/cache, and the WebGPU feature wiring.
   Trades off: real work on shims and build plumbing, but no drift and the
   only path that ends at "full game". **Chosen.**
2. **Stream the desktop game to the browser.** Fast to ship, but monthly GPU
   server cost, input latency, and it is not a real browser game. Rejected:
   against the "easy for players, no server bills" intent.
3. **Embed a different browser Minecraft (Eaglercraft-style) on the site.**
   Not Horizon, not this codebase, licence mess. Rejected.

## The parts decided (with reasons)

- **Repository layout:** MineRust is its own repository (this one) in its own
  folder. It holds the wasm entry crate (once the design is approved and the
  spike is done), this site's source, and the packaging. It depends on the
  Horizon fork's crates as git dependencies, pinned to an exact commit in
  `Cargo.lock` — the fork is public, so the pin is reproducible.
- **Where the shims live:** the platform differences (files, audio, threads,
  features) are edited inside the Horizon fork's crates behind
  `cfg(target_family = "wasm")`, and committed there. Desktop builds never
  link them. MineRust pins the fork commit that carries the shims it needs.
  (Reason: shims must live in the crates they adapt; a second copy of those
  crates inside MineRust would drift.)
- **Rendering:** a `webgpu` feature mirroring the fork's existing
  `vulkan`/`metal`/`dx12`/`gles` feature pattern; on wasm, the surface comes
  from the winit canvas. Whether the fork's vendored `third_party/wgpu-hal`
  patch (Windows Vulkan fixes) is inert on the web backend is a builder check.
- **Files and saves:** a small virtual-filesystem trait at the few places
  that touch the disk for saves, options, and logs; MineRust's web impl
  stores bytes in **OPFS** (a real, durable, per-origin filesystem). Save
  formats do not change: regions, NBT, level.dat land in OPFS paths shaped
  like game folders. Worlds persist per site origin; clearing site data
  clears them (the UI says so, once).
- **Assets — the operator chose public serving:** the site serves the
  prepared pack to every visitor, Eaglercraft-style: one-time download with
  a progress screen, cached in Cache Storage, read through the same VFS
  shim. The pack is built on the operator's machine (`horizon-prepare`),
  measured in slice 1, and uploaded by a small script (never git); the
  manifest pins the pack version to the wasm build. If the pack exceeds
  Vercel's static-file limits, it parks in Vercel Blob behind the same
  manifest.
- **Threads:** wasm atomics + threads behind a feature; the site serves
  COOP/COEP so SharedArrayBuffer works. The server thread stays a real
  thread; the streaming worker pool scales down on web. If a subsystem
  fights the threading model, the builder may run it on the main thread
  first and notes it in the plan.
- **Audio:** enable the game's existing `audio` feature with cpal's web
  backend.
- **Hosting: Vercel.** Custom headers in `vercel.json` provide COOP/COEP
  (threads) and `.wasm` MIME; a WebGPU-capability gate page covers
  unsupported browsers. The game runs locally for now (a dev server with the
  same headers); no permanent public address exists yet. Recorded warning:
  browser saves are bound to the site's origin, so worlds made while testing
  locally will not follow to the future public address — the final domain
  must be picked before public launch.
- **Order of work (each slice is shippable knowledge):**
  0. Spike, throwaway: compile the game's libraries for
     `wasm32-unknown-unknown` and light a canvas through wgpu/winit in a
     tab. No gameplay. Answers the build-plumbing questions before any shim
     work.
  1. Boot the real menus in the tab (fonts, UI drawing) via the shim layers.
  2. Create a world, stream chunks, walk around, break/place.
  3. Save/load worlds in OPFS; options too.
  4. Audio, full-game polish, performance pass, the capability gate page.

## Operator decisions (2026-11-09)

- Game files are served publicly to visitors (the "a" option).
- Hosting: Vercel; local testing first; the permanent address is decided
  before public launch, not now.
- Builder calls, in force unless vetoed: degraded-but-working visuals are
  fine at launch — no WebGPU shader rework, and shader packs stay
  desktop-only; the web menus hide controllers, the narrator, shader-pack
  settings, and the multiplayer/Realms entries; MineRust ships as a live
  site, not a hand-off bundle.

## Left to the builder

- Exact wgpu feature set and target flags for wasm; the atomics build
  pipeline.
- How much of the client's rendering crate compiles unchanged (expected:
  most).
- Worker pool sizing, rAF-driven tick pacing, and the asset-cache format.
- The spike (throwaway) also answers: does the gameplay library compile for
  wasm (it pulls `ureq`/`rustls`, `fs4` and friends that may need feature
  gates); is cpal 0.16's Web Audio host usable; do winit 0.30.13's web
  pointer-lock and IME cover mouse-look and chat; do the vendored
  `third_party/wgpu-hal` and `naga` patches compile for the wasm target.
- The spike's findings feed the plan; the spike itself is throwaway.
