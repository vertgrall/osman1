# Osman by New Tower

> **Work in progress** — early native macOS network monitor built with Rust + [Freya](https://freyaui.dev). UI, menubar integration, and About branding are actively being shaped; expect rough edges.

**Osman** is a colorful live traffic monitor: adapters, processes, connections, traffic character scopes, alerts, and a menu-bar mini monitor — all without root or packet capture (OS interface counters via `sysinfo`).

---

## Screenshots

### About panel (New Tower branding)

Skia canvas splash card with Tower Village artwork, lockup pill, and brand mark — mirrors Mohawk’s About layout.

| Splash card (live render) | Brand mark |
|---|---|
| ![About splash card](docs/screenshots/about-splash-card.png) | ![New Tower brand mark](docs/screenshots/about-brand-mark.png) |

Tower Village source asset (shared with Mohawk):

![Tower Village hero](docs/screenshots/tower-village-hero.png)

---

## What exists today

| Area | Status |
|------|--------|
| **Overview** | Hero network chart, adapter table, sparklines |
| **Adapters / Processes / Connections** | Live tables, drill-down detail views |
| **Traffic Character** | Pattern scopes + timeline (in progress) |
| **Alerts** | Threshold engine + rules UI |
| **Menu bar** | Live RX/TX rates, tray popover, About / Quit |
| **About** | New Tower splash, lockup, brand mark; macOS App menu redirect |
| **Theme** | Light clinical palette (taupe / sage / orange) |
| **Tests** | 57 unit + UI + pixel regression tests (`cargo test`) |

---

## Recent focus (Aug 2026)

1. **About panel** — Replaced macOS generic blue-folder About with a Freya window using embedded Mohawk-parity PNGs (`SplashTowerVillage`, `NewTowerBrandMark`).
2. **macOS App menu hook** — `Osman → About` posts through Freya’s renderer dispatch (safe outside component scope; fixes prior panic).
3. **Regression tests** — Off-screen Skia pixel checks ensure splash/brand canvases actually draw, not just layout empty boxes.

---

## Run locally

```bash
cargo run
```

First compile pulls Freya/Skia and can take several minutes.

```bash
cargo test          # full suite
cargo test about    # About + branding tests only
```

Regenerate README screenshots after art changes:

```bash
EXPORT_README=1 cargo test about_test_harness::tests::export_readme_screenshots -- --ignored --exact
```

---

## Stack

- **GUI:** Freya 0.4 (Skia, winit, tray)
- **Traffic:** `sysinfo` network counters, `lsof` / nettop parsing for connections
- **Polling:** `async-io` timer (~1 Hz snapshot loop)
- **macOS menu:** `objc2` App menu About redirect

---

## Roadmap (in progress)

- [ ] Polish Traffic Character scopes and transitions
- [ ] Connection detail chart axis labels / scale tuning
- [ ] App icon + dock tile (New Tower shell branding)
- [ ] Release build + notarization path
- [ ] Mohawk feature parity checklist

---

## Brand assets

Embedded at build time from `resources/brand/` (same Tower Village + toolbar mark as Mohawk). See `about_assets.rs` for SHA/size parity tests against Mohawk source when that repo is present locally.

---

Designed and developed in Bellevue, WA by Jon McMillion for **New Tower**.
