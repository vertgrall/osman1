# Phase 0.4 — Data health & error UX

**Status:** Shipped  
**ROADMAP:** Phase 0.4  
**Owner:** Implementation Aug 2026

---

## Problem

When `nettop` / `lsof` fail or adapters are empty, Osman shows silent empty UI:

- Hero chart blank
- "Waiting for adapter samples…" with no explanation
- User assumes app is broken

---

## Design

### Health model

New module `data_health.rs`:

```
TrafficSnapshot.collect()
        │
        ├── nettop tcp/udp subprocess
        ├── lsof subprocess
        └──► DataHealth { nettop_tcp_ok, nettop_udp_ok, lsof_ok, row counts }
```

Separate **adapter empty** messaging uses `NetworkSnapshot.sample_tick` + interface count.

### User-visible messages (priority order)

| Condition | Message |
|-----------|---------|
| `sample_tick == 0` && no adapters | "Waiting for first adapter sample…" |
| `sample_tick > 0` && no adapters | "No active network adapters detected (loopback is hidden)." |
| nettop failed entirely | "Connection details unavailable. Install Xcode Command Line Tools: `xcode-select --install`" |
| nettop ok, lsof failed | "Listener list may be incomplete (lsof unavailable)." |
| All ok | _(no banner)_ |

### UI — Overview banner

```
┌─────────────────────────────────────────────────────────────┐
│ ⚠ Connection details unavailable…          [muted taupe bar] │
├─────────────────────────────────────────────────────────────┤
│ Network activity (hero chart)                                │
│ Adapter table                                                │
└─────────────────────────────────────────────────────────────┘
```

- Banner: full width, `palette.panel` background, `palette.muted` text, 12px padding, 11–12px font
- Only on **Overview** (not every section) to avoid noise
- Adapter table empty state uses **same copy** as health (not generic "Waiting…")

### Non-goals (0.4)

- Modal onboarding (Phase 0.5)
- Retry buttons
- Logging to disk

---

## Files

| File | Change |
|------|--------|
| `src/data_health.rs` | **New** — `DataHealth`, message logic, tests |
| `src/detail.rs` | Track collect success; attach `health` to `TrafficSnapshot` |
| `src/overview_ui.rs` | Empty adapter row uses `DataHealth` copy |
| `src/main.rs` | Overview health banner |
| `tests/fixtures/nettop_tcp_sample.txt` | Fixture for ingest test |

---

## Tests (required for done)

| Test | File |
|------|------|
| `banner_none_when_healthy` | `data_health.rs` |
| `banner_nettop_failed` | `data_health.rs` |
| `banner_no_adapters_after_tick` | `data_health.rs` |
| `banner_waiting_first_sample` | `data_health.rs` |
| `ingest_nettop_fixture_populates_connections` | `detail.rs` |
| `overview_shows_health_banner_when_degraded` | `overview_ui.rs` or `data_health.rs` UI test |

---

## Signoff

| Role | Name | Status | Date |
|------|------|--------|------|
| Design | Jon | ☑ Approved | Aug 2026 |
| Implementation | Agent | ☑ Complete | Aug 2026 |
| Tests | | ☑ Complete (69 passed, `cargo test`) | Aug 2026 |

**Design note:** Banner is Overview-only; sage/orange palette unchanged.
