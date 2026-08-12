# Definition of Done — Osman phases

Every **work package** (e.g. Phase 0.4, Phase 1A.1) is **not complete** until all three gates pass.

---

## Gate 1 — Design

| Step | Requirement |
|------|-------------|
| **Design doc** | `docs/design/phase-<id>-<slug>.md` exists before implementation |
| **Signoff** | Status set to **Approved** in doc header (Jon) OR **Updated** if scope changed mid-build |
| **UI changes** | Mock screenshot or annotated description in design doc |
| **Non-UI** | Data flow diagram or bullet behavior spec |

If implementation diverges from design → update design doc **before** marking done.

---

## Gate 2 — Tests

| Step | Requirement |
|------|-------------|
| **Unit tests** | Logic in new modules (`src/*.rs` `#[cfg(test)]`) |
| **UI tests** | Freya `launch_test` for new/changed screens (when applicable) |
| **Regression** | `cargo test` all green; no `#[ignore]` added without issue link |
| **Fixtures** | Parser/collect paths use `tests/fixtures/` when subprocesses involved |

Minimum test count guidance:

| Work type | Minimum |
|-----------|---------|
| New module | ≥2 tests (happy + edge) |
| UI banner / panel | ≥1 render test + message logic tests |
| Settings / prefs | round-trip serialize + default load |
| Chart change | pixel or layout test in harness |

---

## Gate 3 — Ship checklist

- [ ] No new `unwrap()` on user-facing paths without comment
- [ ] ROADMAP.md checkbox updated for work package
- [ ] Design doc status → **Shipped**
- [ ] README note if user-visible behavior changed

---

## Signoff block (copy into each design doc)

```markdown
## Signoff

| Role | Name | Status | Date |
|------|------|--------|------|
| Design | | ☐ Approved ☐ Updated | |
| Implementation | | ☐ Complete | |
| Tests | | ☐ Complete (`cargo test`) | |
```

---

## Phase index

| ID | Design doc | Status |
|----|------------|--------|
| 0.4 | [phase-0.4-data-health.md](../design/phase-0.4-data-health.md) | Shipped |
| 0.5 | [phase-0.5-onboarding.md](../design/phase-0.5-onboarding.md) | Shipped |
| 0.1 | [phase-0.1-clinical-icon.md](../design/phase-0.1-clinical-icon.md) | Shipped |
| 1A | _pending_ | — |
| 1B | _pending_ | — |

Update this table when starting a new package.
