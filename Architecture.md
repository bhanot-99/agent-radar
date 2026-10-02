# Architecture.md — Agent-Radar

> This file summarizes and operationalizes `overview.md` (the authoritative architecture spec) into a concrete folder structure and tech stack. For full diagrams, flowcharts, and the corrections log, read `overview.md` directly — this file does not duplicate the Mermaid diagrams, only references them by section.

---

## 1. App Flow (summary)

```
Linux Kernel (inotify + procfs, both unprivileged)
        │
        ▼
inotify Watcher (recursive emulation, epoll-driven)   — overview.md §3.1, §5.1
        │ RawFsEvent
        ▼
Process Correlator (PID → AI-agent ancestor, socket vs disk fd)  — §3.2, §5.2
        │ EnrichedEvent
        ▼
Semantic Classifier (debounce, rename-cookie pairing, ordered rule table) — §3.3, §5.1
        │ ActivityCategory
        ▼
TUI State Store → Ratatui Render Pipeline (event-driven, cell-diffed) — §3.4, §5.3
        │ ANSI escapes (changed cells only)
        ▼
Terminal (tmux pane / split)
```

Full system-context and component diagrams: `overview.md` §2.1–2.2.
Full flowcharts for watcher init, event attribution, classification, and render loop: `overview.md` §3.1–3.4.
Class/data model diagrams: `overview.md` §4.1–4.2.

**Single-threaded, synchronous design.** There is no async runtime and no cross-thread channel in v1 — the `AppRunner` loop owns one `epoll_wait` call multiplexing the inotify fd, two signal self-pipe fds (resize, shutdown), and an intermittently-armed `timerfd`. This is a deliberate constraint (see `rules.md`), not an oversight.

---

## 2. Folder & File Structure (v1)

```
agent-radar/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── install.sh                      # one-line curl installer (overview.md §7)
├── packaging/
│   ├── aur/PKGBUILD
│   ├── deb/                        # cargo-deb config
│   └── rpm/                        # cargo-generate-rpm config
├── src/
│   ├── main.rs                     # entrypoint: flock, init, hand off to AppRunner
│   ├── app_runner.rs               # AppRunner: owns epoll_fd, timerfd, terminal; run_loop/shutdown
│   ├── events.rs                   # TelemetryEvent, ActivityCategory, FileOp (overview.md §4.2, §6)
│   ├── watcher/
│   │   ├── mod.rs                  # EventSource trait (poll_fd, drain)
│   │   └── inotify_tier.rs         # InotifyWatcher: the only v1 EventSource impl; RawFsEvent
│   ├── correlator/
│   │   ├── mod.rs                  # ProcessCorrelator: pid_cache (LRU, 512, 30s TTL), agent_patterns
│   │   └── procfs.rs               # /proc/<pid>/cmdline, /ppid, /fd/ reading helpers
│   ├── classifier/
│   │   ├── mod.rs                  # SemanticClassifier: debounce map, rate_trackers, classify()
│   │   └── rules.rs                # Ordered Rule table (path-context predicates, overview.md §3.3)
│   ├── tui/
│   │   ├── mod.rs                  # TuiState, FpsTracker, SysStats
│   │   ├── widgets/
│   │   │   ├── header_bar.rs
│   │   │   ├── hero_stream.rs      # HeroCardType rendering + animation frame advance
│   │   │   └── log_feed_table.rs
│   │   └── theme.rs                # palette + style constants (see design.md)
│   └── signals.rs                  # SIGWINCH/SIGTERM/SIGINT self-pipe setup
└── tests/
    ├── watcher_tests.rs
    ├── classifier_tests.rs
    └── correlator_tests.rs
```

Notes:
- `watcher/mod.rs` defines the `EventSource` trait specifically so a v2 privileged tier (`fanotify_tier.rs`, `ebpf_tier.rs`) can be added later without reshaping `correlator`/`classifier`/`tui` — but no such file exists or is compiled in v1 (`overview.md` §4.1, §9.2).
- No `socket/` module exists in v1 — the MCP/OTel listener is v2-only and must not be scaffolded early (see `rules.md`).

---

## 3. Tech Stack

| Layer | Choice | Why |
|---|---|---|
| Language | Rust | Static binary, no GC, fits the <10MB RSS / zero-idle-CPU target |
| Filesystem watching | `inotify` crate + raw `inotify_init1`/`epoll_create1` via `rustix` | Only unprivileged kernel primitive that fits v1's zero-privilege requirement |
| Event loop | `epoll` (via `rustix`), `timerfd_create`, signal self-pipes (`signal-hook`) | Single-threaded, synchronous reactor — no async runtime |
| Process introspection | `/proc` reads (no crate; direct `procfs` path parsing) | No special privilege needed for own-user `/proc/<pid>` entries |
| Caching | `lru` crate | Fixed-capacity, TTL-swept caches for pid cache (512, 30s) and rate trackers (2048, 30s) |
| Classification | `regex` crate | Agent-ancestor pattern matching (`claude`, `aider`, `python`, `node`, …) |
| TUI rendering | `ratatui` + `crossterm` | Buffer-diffed rendering, TrueColor 24-bit, Unicode box-drawing |
| Single-instance lock | `flock` (via `rustix`) | No extra dependency; standard unprivileged syscall |
| Build target | `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` | Static linking with zero C-library friction (unlike eBPF's libbpf) |
| Packaging | `cargo-deb`, `cargo-generate-rpm`, AUR `PKGBUILD`, `install.sh` | Covers Debian/Ubuntu, Fedora, Arch, and generic curl install |

**Explicitly excluded from v1's dependency tree:** `tokio`/any async runtime, `aya` (eBPF), `libbpf`-linked crates, any fanotify crate, any socket/IPC crate. These are v2-only and gated behind `--features ebpf`/`--privileged` when they land (`overview.md` §9.2).

---

## 4. Data Model (authoritative)

See `overview.md` §4.2 and §6 for the full, corrected (non-redundant) type definitions:

- `RawFsEvent` — raw inotify watch descriptor + mask + cookie + name.
- `TelemetryEvent` — thin envelope: timestamp, pid, ppid, process_name, `category`.
- `ActivityCategory` — enum carrying **all** path/size/rate data exactly once per variant (no duplicate fields with `TelemetryEvent`).
- `HeroCardType` — wraps an `ActivityCategory` plus pure UI/animation state (`started_at`, frame counters); never redeclares domain fields.

Do not reintroduce flat `raw_path`/`size_bytes` fields on `TelemetryEvent` alongside per-variant fields on `ActivityCategory` — that duplication was corrected in `overview.md` §9.1 item 12 and must stay fixed.

---

## 5. Performance Enforcement Hooks

Each §8 SLA target maps to a specific implementation obligation — see `rules.md` for the corresponding "must/must not" list:

- Idle CPU ≈ 0% → `epoll_wait(-1)`, `timerfd` armed only during active hero-card animation.
- RSS < 10MB → bounded `LruCache` everywhere; no unbounded `Vec`/`HashMap` growth.
- Latency < 10–15ms → classification runs only after debounce settles, never per raw write.
- FD cap ≤ 10 → exactly: 1 inotify fd, 1 epoll fd, 2 signal self-pipe fds, 1 intermittent timerfd, stdio, 1 lockfile fd.
