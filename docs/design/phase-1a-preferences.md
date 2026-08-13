# Phase 1A.1 — Preferences model

**Status:** Shipped  
**ROADMAP:** Phase 1A.1  
**Owner:** Aug 2026

---

## Goal

Persist user choices across restarts in `~/Library/Application Support/Osman/preferences.json`.

---

## Schema (v1)

```json
{
  "poll_interval_ms": 1000,
  "default_section": "overview",
  "onboarding_done": false,
  "menubar_only": false
}
```

| Field | Default | Notes |
|-------|---------|-------|
| `poll_interval_ms` | `1000` | Allowed: 500, 1000, 2000 |
| `default_section` | `"overview"` | Sidebar section on launch |
| `onboarding_done` | `false` | Replaces `onboarding_done` flag file |
| `menubar_only` | `false` | Reserved for 1A.2e |

---

## Migration

On load, if legacy `onboarding_done` file exists → set `onboarding_done: true`, save JSON, delete legacy file.

---

## Wiring (1A.1)

- `main()` → `preferences::init()` before `launch()`
- Poll loop uses `poll_interval()` from prefs
- `app_section` initial state from `default_section()`
- Onboarding dismiss → `set_onboarding_done()` + save

Settings UI for editing (1A.2) comes next.

---

## Tests

| Test | |
|------|--|
| `default_preferences` | |
| `round_trip_json` | |
| `missing_file_loads_defaults` | |
| `migrate_legacy_onboarding_flag` | |
| `poll_interval_clamps_to_allowed_values` | |

---

## Signoff

| Role | Name | Status | Date |
|------|------|--------|------|
| Design | Jon | ☑ Updated | Aug 2026 |
| Implementation | Agent | ☑ Complete | Aug 2026 |
| Tests | | ☑ Complete (83 passed) | Aug 2026 |
