# Phase 0.1 — Clinical Scope app icon

**Status:** Shipped  
**ROADMAP:** Phase 0.1  
**Chosen direction:** Mock **04 — Clinical Scope** (scope ring + sage/orange waveform)

---

## Decision

| Option | Verdict |
|--------|---------|
| 01 Orbit Pulse | Rejected |
| 02 Dual Wave | Rejected |
| 03 Tower Beacon | Rejected |
| **04 Clinical Scope** | **Selected** — matches Osman “clinical monitor” UI language |
| 05 O Stream | Rejected |

Master asset: `resources/icon/OsmanAppIcon-1024.png` (from `osman-icon-mock-04-clinical-scope.png`).

---

## Delivery

| Asset | Path |
|-------|------|
| Master 1024 | `resources/icon/OsmanAppIcon-1024.png` |
| App icon set | `resources/icon/AppIcon.appiconset/` |
| Dock `.icns` | `resources/icon/AppIcon.icns` (built by `build.rs`) |
| Menubar 22px | `resources/icon/MenubarIcon-22.png` → `icon_assets.rs` |
| Window 128px | `resources/icon/WindowIcon-128.png` → Freya `WindowConfig` |
| Bundle | `scripts/build-release.sh` → `dist/Osman.app` |
| Plist | `resources/Info.plist` — `com.newtower.osman`, “Osman by NT” |

Regenerate PNG sizes: `./scripts/generate-app-icons.sh`  
Rebuild bundle: `./scripts/build-release.sh`

---

## Signoff

| Role | Name | Status | Date |
|------|------|--------|------|
| Design | Jon | ☑ Approved (Clinical Scope) | Aug 2026 |
| Implementation | Agent | ☑ Complete | Aug 2026 |
| Tests | | ☑ Complete (78 passed) | Aug 2026 |
