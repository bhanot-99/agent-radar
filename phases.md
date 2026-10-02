# phases.md — Agent-Radar Build Phases

> Each phase should produce something runnable/testable before moving on. Update `memory.md` when a phase starts and when it completes. Phases 0–7 are v1 scope. Phase 8 is v2 and explicitly deferred — do not pull work forward from it without an explicit decision to re-scope (see `rules.md` §5).

---

### Phase 0: Project Scaffolding
- `cargo init`, set up `Cargo.toml` with the approved v1 dependency set only (`rules.md` §3).
- Set up `x86_64-unknown-linux-musl` build target locally; confirm a trivial "hello" binary links statically.
- Establish folder structure per `Architecture.md` §2 (empty modules with `mod.rs` stubs is fine).
- CI/test scaffold (`cargo test` running green on an empty test suite).

**Exit criteria:** `cargo build --release --target x86_64-unknown-linux-musl` produces a static binary.

---

### Phase 1: Unprivileged Watcher Core
- Implement `InotifyWatcher` (`src/watcher/inotify_tier.rs`): `inotify_init1`, recursive watch walk, create-then-watch race closer, `max_user_watches` guard with graceful degradation.
- Implement the `AppRunner` event loop skeleton: `epoll_create1`, register inotify fd + signal self-pipes, `epoll_wait` blocking loop.
- Implement single-instance `flock` lock + "already running" exit path.
- Implement `SIGWINCH`/`SIGTERM`/`SIGINT` self-pipe handling and guaranteed teardown (including panic hook).

**Exit criteria:** Running the binary against a real directory and touching/creating/deleting files in another terminal produces correctly-deduplicated raw events in a debug log; resize and Ctrl-C both exit cleanly.

---

### Phase 2: Process Correlator
- Implement procfs reads (`/proc/<pid>/cmdline`, `/ppid`, `/fd/`).
- Implement the bounded, TTL-swept `pid_cache` (LRU, 512, 30s) with liveness re-check on cache hit.
- Implement AI-agent ancestor matching (`agent_patterns`: `claude`, `aider`, `python`, `node`, etc.).
- Implement socket-vs-disk fd differentiation (`ACTIVE_NETWORK_STREAM` flag).

**Exit criteria:** Events from Phase 1 are enriched with correct PID ancestry and network/disk flags, verified against a running `aider`/`claude` session and a plain `touch`/`curl` test.

---

### Phase 3: Semantic Classifier
- Implement per-path debounce window (~300–500ms) and rename-cookie pairing.
- Implement the ordered, path-context-aware rule table (`overview.md` §3.3): checkpoint extensions, `.bin` disambiguation via sibling `*.index.json`/path segments, archive/dataset extensions gated on `ACTIVE_NETWORK_STREAM`, source-code extensions triggering gated line-diffing, directory creation → `WorkspaceExpansion`.
- Implement the bounded `rate_trackers` LRU (2048, 30s).

**Exit criteria:** A scripted test fixture (file writes, renames, a simulated checkpoint write, a simulated dataset download) produces the expected `ActivityCategory` for each case, including the `.bin` disambiguation case.

---

### Phase 4: TUI Render Core
- Implement `TuiState`, `FpsTracker`, `SysStats` (self RSS/CPU via `/proc/self/status`).
- Implement base layout: header bar, log feed table, footer — no hero card animation yet.
- Wire the event-driven redraw: only render on actual state change, Ratatui buffer diffing for ANSI emission.
- Implement `timerfd` plumbing (created but not armed yet — no animation to drive).

**Exit criteria:** Live classified events from Phase 3 appear in the log feed table in real time; idle CPU measured at ~0% via `top`/`perf stat` over a quiet period.

---

### Phase 5: Cyberpunk Hero Cards & Animation
- Implement `HeroCardType` (DataStreamCard, ModelTrainingCard, IdleCard) wrapping `ActivityCategory` + animation state.
- Arm/disarm `timerfd` based on whether a hero card is actively animating.
- Apply the full palette and widget styling from `design.md`.

**Exit criteria:** A simulated dataset download or checkpoint write triggers an animated hero card; `timerfd` disarms and CPU returns to ~0% once the hero card goes idle.

---

### Phase 6: Performance Validation Against §8 SLAs
- Measure and record: idle CPU, RSS baseline, event latency (inotify→render), FD count, redraw rate under burst.
- Fix any regression against `overview.md` §8 targets before proceeding — this phase is a gate, not a formality.

**Exit criteria:** All five §8 metrics pass on a real project tree (e.g., a mid-sized git repo) under both idle and active-agent conditions.

---

### Phase 7: Packaging & Distribution
- Static release builds for `x86_64`/`aarch64-unknown-linux-musl`.
- `install.sh` one-line curl installer.
- `cargo-deb`, `cargo-generate-rpm`, AUR `PKGBUILD`.
- README with install instructions for all four paths plus `cargo install agent-radar`.

**Exit criteria:** A clean VM/container for each of Debian, Fedora, and Arch can install and run the binary via its respective packaging path with no manual dependency installation.

---

### Phase 8 (v2 — Deferred, Not In Scope Yet)
Listed here only for roadmap visibility — do not begin without an explicit decision to start v2:

- Opt-in `fanotify` tier (`--privileged` flag), documented with its real `CAP_SYS_ADMIN`/`CAP_DAC_READ_SEARCH` requirement.
- Opt-in eBPF tier (`--features ebpf`, via `aya`) for sub-2ms latency.
- MCP/OpenTelemetry Unix-socket listener (`/tmp/agent-radar.sock`, 0700/0600 perms, per-run token auth, stale-socket recovery).
- Kitty graphics protocol rendering for supporting terminals.

See `overview.md` §9.2 for full detail on each.
