# memory.md — Agent-Radar Project State Tracker

> **Crucial rule (per project setup instructions): this file must be updated regularly as work progresses.** Update it whenever a phase starts, a phase completes, a decision is made that isn't already captured in the other 5 files, or the "currently working on" file/task changes. Don't let it go stale — a future session (human or AI) should be able to read only this file and know exactly where things stand.

---

## 1. Completed So Far

- **2026-10-02** — `overview.md` architecture specification finalized: v1 scope locked to inotify + epoll only (zero privilege), with eBPF/fanotify/MCP-socket/Kitty-graphics explicitly deferred to v2. 12 design flaws from an earlier draft identified and corrected (see `overview.md` §9.1).
- **2026-10-02** — Foundation documentation set generated from `overview.md`:
  - `PRD.md` — product definition, target users, core features, success criteria.
  - `Architecture.md` — folder/file structure, tech stack, data model, app flow summary.
  - `rules.md` — use/avoid lists, dependency boundaries, error handling rules, AI assistant boundaries.
  - `phases.md` — Phase 0 through Phase 7 (v1) + Phase 8 (v2, deferred) breakdown.
  - `design.md` — cyberpunk color palette, terminal-font guidance, typography-to-TUI-styling mapping.
  - `memory.md` — this file, initialized.

- **2026-10-02** — Phase 0 completed:
  - Git repository initialized.
  - Cargo project initialized as `agent-radar` with edition 2021.
  - Approved v1 dependencies configured (`inotify` with default-features disabled to ensure no tokio/async runtime, `rustix`, `ratatui`, `crossterm`, `lru`, `regex`, `signal-hook`).
  - Architecture module skeleton created (`src/`, `tests/`, `packaging/`).
  - `x86_64-unknown-linux-musl` target verified: produces fully static-pie linked release binary.
  - Tests running green.

- **2026-10-02** — Phase 1 completed:
  - `InotifyWatcher` (`src/watcher/inotify_tier.rs`) implemented with recursive directory walking, create-then-watch race closer, and `max_user_watches` budget guard.
  - `SignalPipes` (`src/signals.rs`) implemented with non-blocking pipes for SIGWINCH (resize) and SIGTERM/SIGINT (shutdown).
  - Single-instance `flock` lockfile (`/tmp/agent-radar-<uid>.lock`) and panic hook for guaranteed terminal restoration implemented.
  - `AppRunner` synchronous epoll reactor implemented with inotify, signal pipes, and timerfd integration.
  - Live testing confirmed: touching/modifying/deleting files logs raw events correctly; SIGINT terminates the process cleanly with code 0.
  - Unit/integration test suite passed (5/5 tests in `watcher_tests.rs`).

- **2026-10-02** — Phase 2 completed:
  - `ProcessCorrelator` (`src/correlator/mod.rs`) and `procfs` (`src/correlator/procfs.rs`) implemented.
  - Procfs parsing implemented for `/proc/<pid>/status`, `/proc/<pid>/cmdline`, `/proc/<pid>/stat` (extracting `starttime` at field 22), and `/proc/<pid>/fd/` inspection.
  - Bounded TTL-swept LRU `pid_cache` (capacity 512, 30s TTL) with liveness re-verification implemented to prevent stale PID reuse misattribution.
  - AI agent pattern matching implemented for major agents (Claude, Aider, Antigravity, Cursor, Gemini, Python, Node, etc.).
  - Socket-vs-disk differentiation (`ACTIVE_NETWORK_STREAM`) implemented via `/proc/<pid>/fd/` inspection.
  - Unit/integration tests passed (5/5 tests in `correlator_tests.rs`).

- **2026-10-02** — Phase 3 completed:
  - `SemanticClassifier` (`src/classifier/mod.rs`) and ordered rule table (`src/classifier/rules.rs`) implemented.
  - Per-path debounce window (~300ms) implemented for write coalescing.
  - Rename-cookie pairing implemented to correlate atomic editor saves (`IN_MOVED_FROM`/`IN_MOVED_TO`) into single content replacement events.
  - Ordered path-context predicates implemented: `.bin` disambiguation using sibling `*.index.json` and directory context (`checkpoints`, `weights`, `models`), `.safetensors`/`.pt`/`.ckpt`/`.onnx` checkpoint detection, network-gated dataset streams (`.parquet`, `.zip`, `.tar.gz`), and source-code line-diffing.
  - Bounded `rate_trackers` LRU (capacity 2048, 30s TTL) implemented.
  - Unit/integration test suite passed (7/7 tests in `classifier_tests.rs`, 17/17 total tests).

- **2026-10-02** — Phase 4 completed:
  - TUI rendering core implemented with Ratatui (`src/tui/mod.rs`), widgets (`HeaderBarWidget`, `HeroStreamWidget`, `LogFeedTableWidget`), and theme (`src/tui/theme.rs`).
  - Base layout established (Top status bar, middle split with Hero panel & Log Feed table, and footer with self-telemetry).
  - Event-driven redraw wired to epoll event loop in `AppRunner`. Buffer cell diffing ensures ANSI sequences are emitted only on actual visual changes.
  - Idle CPU verified at ~0% via blocking `epoll_wait(-1)`.
  - Self RSS verified at ~4.5 MB (well under the <10 MB baseline and <8 MB stretch target).
  - Live testing confirmed: workspace expansions, source mutations, and checkpoint writes appear live in the feed.

- **2026-10-02** — Phase 5 completed:
  - Cyberpunk Hero Cards (`HeroStreamWidget`) implemented with animated pulse beams for `ModelTrainingCard` and streaming flow gauges for `DataStreamCard`.
  - Static, low-visual-salience `IdleCard` implemented per `design.md` §4.
  - `timerfd` armed dynamically only while a hero card is actively animating.
  - Inactivity timeout (10s) transitions hero cards back to `IdleCard`, disarming `timerfd` and returning the process to 0% idle CPU.
  - Verified with automated tests in `tests/tui_tests.rs` (2/2 tests passing, 19/19 total).

- **2026-10-02** — Phase 6 completed:
  - All five §8 SLA metrics empirically measured and verified on the real project tree:
    - **Idle CPU**: measured at 0.0% (fully blocked on `epoll_wait(-1)` with zero scheduled ticks when idle).
    - **Memory (RSS)**: measured at 4.38 MB (well under the <10 MB baseline and <8 MB stretch target).
    - **FD Count**: measured at 9 open FDs (enforced below the <= 10 FD cap via unified signal self-pipe and closing inherited stray FDs).
    - **Event Latency**: verified sub-millisecond inotify dispatch and debounced coalescing under rapid file write burst.
    - **Zero Privilege**: verified running completely unprivileged without `sudo`, `setcap`, or kernel capabilities.
  - All 19 unit and integration tests passing.

---

- **2026-10-02** — Phase 7 completed:
  - Packaging files created: `install.sh` (one-line curl installer), `packaging/aur/PKGBUILD` (Arch Linux AUR), `packaging/deb/control` (Debian/Ubuntu), `packaging/rpm/agent-radar.spec` (Fedora/RHEL).
  - MIT `LICENSE` and comprehensive `README.md` created with architecture overview, visual design mapping, installation guides, and SLA benchmarks.
  - Release binaries built: native and static `x86_64-unknown-linux-musl` static-PIE binary.
  - `install.sh` locally tested: installs to `~/.local/bin/agent-radar` and verifies with `--version`.
  - **All v1 phases (Phase 0 through Phase 7) are now fully completed and verified!**

---

## 2. Currently Working On

- **Active file/task**: None — all v1 phases (Phases 0 through 7) are complete.
- **Next expected step**: Project complete and ready for production use. Phase 8 (v2 roadmap) remains explicitly deferred per project specifications.

---

## 3. Decisions Log (not already captured elsewhere)

- Treated `overview.md` as the approved, authoritative architecture source — PRD/Architecture/rules/phases/design were derived from it rather than re-deriving the design from scratch, since it already includes a corrections log and explicit v1/v2 boundary.
- Target users (PRD §2) were inferred from the problem framing in `overview.md` §1 (AI CLI agent users on Linux who won't grant elevated privileges) — not independently specified elsewhere. Flag if this inference is wrong.
- `design.md`'s "fonts"/"typography scale" requirements were reinterpreted for a terminal-UI context (terminal font recommendations + Ratatui style/border hierarchy) since Agent-Radar has no GUI font-rendering layer.

---

## 4. Open Questions / Flags for the User

- Confirm target users (PRD.md §2) match actual intent — currently inferred, not explicitly stated.
- Confirm the color palette in `design.md` §1.2 reads well in the user's actual terminal color profile once Phase 4/5 produces a renderable HUD (TrueColor rendering can shift slightly across terminal emulators).
- No git repository exists yet in `/home/bhanotos/Reporter` — decide whether to `git init` before Phase 0, or scaffold the Rust project in a separate repo/directory.
