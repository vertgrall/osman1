# Phase 1E.1–1E.2 — Collect fixtures & ingest tests

**Status:** Approved  
**ROADMAP:** Phase 1E.1, 1E.2  
**Owner:** Aug 2026

---

## Goal

Parser tests must not spawn `nettop` / `lsof`. Check in **synthetic** samples that match real macOS column layout, then prove they become a `TrafficSnapshot`.

Think of it like a recipe card vs cooking live: the fixture is the card (known ingredients); the ingest function is the kitchen. We never call DoorDash (`Command`) in unit tests.

---

## Data flow

```
tests/fixtures/*.txt          src/detail.rs
┌─────────────────────┐       ┌──────────────────────────┐
│ nettop_tcp_sample   │──►    │ ingest_nettop_text       │
│ nettop_udp_sample   │──►    │                          │──► TrafficSnapshot
│ lsof_sample         │──►    │ ingest_lsof_text         │    { connections, processes }
└─────────────────────┘       └──────────────────────────┘
        no subprocesses                    no Networks lookup in tests
```

Production still runs `nettop -m tcp|udp -L 1 -n -x` and `lsof -n -P -i`, then calls the same ingest functions.

---

## Fixture rules

| Rule | Why |
|------|-----|
| Synthetic IPs / names only | No real user hostnames, usernames, or LAN addresses |
| Match live column layout | TCP/UDP CSV: `time, key, interface, state, bytes_in, bytes_out, …` |
| lsof: 9+ whitespace fields | `parse_lsof_line` uses `parts[0]=COMMAND`, `parts[1]=PID`, `parts[8..]=NAME` |
| COMMAND is one token | lsof truncates; spaces would split the line wrong |
| Listeners vs established | lsof ingest **only** keeps `SocketRole::Listener` |

---

## Ingest behavior (spec)

```
lsof line
   │
   ├─ parse COMMAND + PID + NAME
   ├─ parse_lsof_name → role
   ├─ skip if not Listener
   └─ skip if (pid, local_port, transport) already in snapshot
```

Interface in tests is injected (`en0` / `lo0`) so ingest does not call `sysinfo::Networks`.

---

## Tests

| Test | Asserts |
|------|---------|
| `ingest_nettop_fixture_populates_connections` | Existing TCP sample → 2 Chrome sockets |
| `ingest_udp_fixture_populates_udp_flows` | UDP sample → UDP rows + process bytes |
| `ingest_lsof_fixture_adds_listeners_only` | LISTEN kept; ESTABLISHED skipped; v4/v6 same port deduped |
| `fixtures_merge_into_traffic_snapshot` | TCP + UDP + lsof → one snapshot, health row counts |

---

## Out of scope (later 1E)

- 1E.3 adapter friendly names (SCNetwork)
- 1E.4 beta feedback link in Settings

---

## Signoff

| Role | Name | Status | Date |
|------|------|--------|------|
| Design | Jon | ☐ Approved ☐ Updated | |
| Implementation | | ☐ Complete | |
| Tests | | ☐ Complete (`cargo test`) | |
