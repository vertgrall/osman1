# Phase 0.5 — Onboarding & privacy

**Status:** Shipped  
**ROADMAP:** Phase 0.5  
**Owner:** Aug 2026

---

## Problem

New users don't know what Osman reads, what it skips, or why subprocess tools matter. Without a first-run screen, privacy expectations are unclear.

---

## Design

### First-run sheet (modal)

Shown once per Mac until dismissed. Demo / screenshot mode skips it.

```
┌──────────────────────────────────────────────────────────────┐
│ ░░░░░░░░░░░░░ scrim (taupe 55% alpha) ░░░░░░░░░░░░░░░░░░░░░ │
│                                                              │
│     ┌────────────────────────────────────────────┐         │
│     │ Welcome to Osman                              │         │
│     │ Network traffic monitor for macOS · New Tower │         │
│     │                                               │         │
│     │ What Osman reads                              │         │
│     │ · Adapter rates (sysinfo)                     │         │
│     │ · Processes & connections (nettop / lsof)   │         │
│     │ · Menubar live rates                          │         │
│     │                                               │         │
│     │ What Osman does not do                        │         │
│     │ · No packet capture (PCAP) or payload inspect │         │
│     │ · No root or kernel extensions                │         │
│     │ · No upload — data stays on this Mac          │         │
│     │                                               │         │
│     │ [ Get started ]  Settings   About             │         │
│     └────────────────────────────────────────────┘         │
└──────────────────────────────────────────────────────────────┘
```

- **Panel:** `palette.panel`, 12px radius, 1px border, max width ~480px, 20px padding
- **Scrim:** absolute full-window overlay, blocks interaction with app beneath
- **Get started:** primary — dismiss + persist flag
- **Settings:** navigate sidebar to Settings, dismiss + persist
- **About:** open About window (`menubar::request_about_window`), dismiss + persist

### Persistence (0.5c)

Flag file: `~/Library/Application Support/Osman/onboarding_done`  
Phase 1A migrates to `preferences.json` → `onboarding_done`.

---

## Files

| File | Change |
|------|--------|
| `src/onboarding.rs` | **New** — store, copy, overlay UI, tests |
| `src/main.rs` | Show overlay on first launch; wire callbacks |
| `docs/design/phase-0.5-onboarding.md` | This doc |

---

## Tests (required for done)

| Test | File |
|------|------|
| `mark_seen_creates_flag_file` | `onboarding.rs` |
| `has_seen_false_when_missing` | `onboarding.rs` |
| `onboarding_sheet_renders_welcome_copy` | `onboarding.rs` |
| `get_started_dismisses_overlay` | `onboarding.rs` |

---

## Signoff

| Role | Name | Status | Date |
|------|------|--------|------|
| Design | Jon | ☑ Approved | Aug 2026 |
| Implementation | Agent | ☑ Complete | Aug 2026 |
| Tests | | ☑ Complete (73 passed, `cargo test`) | Aug 2026 |
