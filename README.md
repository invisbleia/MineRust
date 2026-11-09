# MineRust

**The full game in your browser tab.** MineRust is the WebAssembly port of
[Horizon](https://github.com/horizon-rust/horizon) (a Rust rewrite of
Minecraft: Java Edition 26.3), built on the
[horizon-cracked](https://github.com/invisbleia/horizon-cracked) fork.

Single-player first: menus, world creation, playing, and worlds that save in
your browser. No download, no install — open the page and play.

## Status

**Design phase.** The design document lives in
[docs/design.md](docs/design.md). No code yet.

## How it fits together

- **MineRust** (this repo) — the browser entry, the website, and packaging.
- **[horizon-cracked](https://github.com/invisbleia/horizon-cracked)** — the
  game's libraries and the desktop client. MineRust builds on them, pinned to
  a commit; the browser shims inside them live there.

## Roadmap

0. Spike: the renderer boots in a tab (throwaway).
1. Real menus in the tab.
2. Create a world, walk around, break and place blocks.
3. Save and load worlds in the browser.
4. Sound, polish, performance, and the "your browser can't run this" page.
