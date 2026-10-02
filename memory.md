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
  - All v1 phases (Phase 0 through Phase 7) fully completed and verified.

- **2026-10-03** — Deep Review Remediation & Architecture Hardening:
  - **Critical Bug 1 (AI Agent vs Runtime False Positives)**: Separated `agent_patterns` (`claude`, `aider`, `antigravity`, `cursor`, `copilot`, `gemini`) from intermediate runtimes (`python`, `node`, `bash`, `sh`). Plain `python script.py` or `node server.js` now cleanly resolves as `is_ai_agent: false`, while subprocesses spawned by agents climb `ppid` to correctly attribute ancestry.
  - **Critical Bug 2 (Unicode TUI Panic)**: Replaced byte-slicing with Unicode-aware character/column boundary truncation and padding utility (`src/tui/unicode_util.rs`) across `hero_stream.rs`, `log_feed_table.rs`, and `header_bar.rs`. Zero panics under arbitrary CJK, emoji, and accented filenames in all terminal widths.
  - **Critical Bug 3 (Unpaired MOVED_FROM Event Loss)**: Updated `flush_ready` to sweep expired unpaired renames (>1s) as deletion events (`FileOp::Deleted`), accurately surfacing files moved out of the workspace.
  - **High Bug 4 (Self-Telemetry Real Metrics)**: Replaced hardcoded `cpu_pct: 0.0` and `update_sys_stats(1)` with real `CpuTracker` sampling `/proc/self/stat` (utime + stime over elapsed time) and passing actual correlator tracked PID count.
  - **High Bug 5 (Dead Code Paths / SystemIdle)**: Initialized HUD with `ActivityCategory::SystemIdle`, emitted `SystemIdle` event upon 10s inactivity transition to `IdleCard`, and rendered `progress_pct` on `IncomingDataStream`.
  - **High Bug 6 (Terminal Raw Mode Startup Leak)**: Added `restore_terminal()` calls to `AppRunner::new` failure path and `src/main.rs` before `exit(1)`.
  - **High Bug 7 (O(processes × fds) Proc Scan Burst)**: Implemented candidate-first PID matching (`pid_cache` + `last_known_agent_pid`), checking ~5-10 processes in microseconds, and rate-limited full `/proc` scan to at most once per 500ms for current UID only.
  - **Medium Bug 8 (Global Lock vs Per-Path)**: Switched single-instance lock to canonical path hash (`/tmp/agent-radar-{uid}-{hash:016x}.lock`), allowing concurrent instances on distinct workspaces.
  - **Medium Bug 9 (Rename All Lines Added Bug)**: Transferred line count cache entries from source path to destination path during rename pairing, reporting 0 added / 0 removed for pure renames.
  - **Medium Bug 10 (Hero Card Progress Reset)**: Preserved `started_at` and frame counter across continuous updates for the same active hero card.
  - **Medium Bug 11 (Socket vs Disk Differentiation & Strict Namespace Scoping)**: Fixed network namespace scoping issue where `/proc/<pid>/net/tcp` reflected the whole host namespace. Implemented precise per-PID socket resolution by reading `/proc/<pid>/fd/*`, extracting `socket:[<inode>]`, and cross-referencing against `/proc/net/tcp`, `/proc/net/tcp6`, `/proc/net/udp`, `/proc/net/udp6` requiring state `01` (`TCP_ESTABLISHED`). Removed broad fallback heuristics so only processes with established network sockets trigger `IncomingDataStream`.
  - **High Bug 5 (Progress Percentage Calculation & Dead Code Paths)**: Populated `progress_pct` with real calculations in `classify_direct`. Supports both sibling metadata (`.size`, `.metadata`, `.json`, directory manifests) and sparse pre-allocation detection (`st_blocks * 512 < st_size`), accurately computing physical bytes written against target logical size. Initialized HUD with `ActivityCategory::SystemIdle` and emitted `SystemIdle` on idle transitions.
  - **Packaging (13-16 & Upstream Overrides)**: Closed supply-chain gap in `packaging/aur/PKGBUILD` by removing unhashed aarch64 `SKIP` from the binary package and adding `packaging/aur/PKGBUILD.src` for native multi-arch builds (`x86_64` and `aarch64`) via `cargo build --release --locked`. Updated `PKGBUILD`, `agent-radar.spec`, and `install.sh` to support configurable upstream repository origins (`AGENT_RADAR_REPO` and `--define 'agent_radar_repo ...'`).
  - **Minor / Clippy**: Fixed all 15 clippy warnings; `cargo clippy --all-targets` runs with 0 warnings.
  - **Test Suite**: Expanded test suite to 31 unit and integration tests, covering Unicode truncation, unpaired moves, runtime ancestry walk, footer telemetry, hero persistence, per-path locking, active TCP socket PID scoping, and sparse/metadata progress calculation. All 31 tests pass green.

- **2026-10-03** — 26-Category Taxonomy & Generic Hero-Card Animation Expansion (`docs/superpowers/specs/2026-10-03-category-expansion-design.md`):
  - **Category Taxonomy (26 + SystemIdle)**: Fully expanded `ActivityCategory` across 6 groups:
    - Group A (Code & Docs, preserving per-language line diffing): `RustEdit`, `PythonEdit`, `WebEdit`, `StyleEdit`, `MarkupEdit`, `ConfigEdit`, `DocsEdit`, `ShellScriptEdit`.
    - Group B (Data & Media): `ImageAsset`, `AudioAsset`, `VideoAsset`, `FontAsset`, `NotebookActivity`.
    - Group C (AI/ML): `ModelTrainingCheckpoint`, `ModelConfigEdit`.
    - Group D (Network / Transfer): `IncomingDataStream`, `ArchiveWrite`.
    - Group E (Process & Action): `GitOperation`, `DependencyLockUpdate`, `TestFileActivity`, `EnvSecretChange`, `CiPipelineEdit`, `ContainerConfigEdit`.
    - Group F (Structural): `WorkspaceExpansion`, `MassDeletion`, `FileMutation`.
    - System State: `SystemIdle`.
  - **Strict Compiler-Enforced Exhaustiveness**: Every `match` over `ActivityCategory` in `theme.rs`, `log_feed_table.rs`, `hero_stream.rs`, and `classifier/mod.rs` is exhaustive with zero wildcard `_` fallback arms.
  - **Single Source of Truth (`theme.rs`)**: Implemented `CategoryVisual` mapping each category to its 2 animation-safe Unicode glyphs, funny all-caps caption, TrueColor hex, log label, and animation primitive.
  - **5 Cyberpunk Animation Primitives (`hero_stream.rs`)**:
    - `PulseDot`: category-parameterized alternating pulse dot.
    - `FlowArrow`: directional traveling marker stream gauge.
    - `BounceBar`: multi-state bouncing gauge (`glyph_b` / `▒` / `glyph_a`).
    - `Spinner`: 4-frame rotation cycling on `['◐','◓','◑','◒']`.
    - `Wave`: phase-shifted column wave cycling `▁▂▃▄▅▆▇█▇▆▅▄▃▂▁`.
  - **Rule Engine Priority Order (§5.1)**:
    1. Directory create -> `WorkspaceExpansion`
    2. Exact matches (`.env`, `Dockerfile`, lockfiles, CI workflows)
    3. Test pattern (`test_*`, `*_test.*`, `*.spec.*`, `tests/` path)
    4. Git attribution (`is_git_process: true`)
    5. Checkpoint weights (`.pt`, `.safetensors`, `.ckpt`, `.onnx`)
    6. `.bin` disambiguation
    7. Model config (`.json`/`.yaml` in model context)
    8. Dataset/archive (`active_network_stream` -> `IncomingDataStream`, else `ArchiveWrite`)
    9. Media extensions
    10. Per-language code edits with debounced line-diffing
    11. `FileMutation` catch-all
  - **MassDeletion Burst Coalescing (§5.2)**: Buffers deletions into `delete_burst` during 300ms window; emits single `MassDeletion` event if count >= 5, or individual deletions if < 5.
  - **Generic HeroCardType**: Consolidated `HeroCardType` into `Active { category, started_at, last_updated, frame }` and `IdleCard`, animating all 26 active categories while preserving frame/started_at on continuous updates.
  - **Raw Mode Keyboard Handling**: Added `TAG_STDIN` to epoll reactor to handle Ctrl-C (0x03) and 'q' keypresses directly in raw mode for instantaneous clean terminal exit.
  - **Empirical SLA Verification on Release Binary**:
    - Idle CPU: 0.0% (0 ticks over sleep window while blocked on `epoll_wait(-1)`).
    - RSS: 4.65 MB (well under <10 MB baseline and <8 MB stretch target).
    - Open FDs: 9 (enforced strictly <= 10 cap).
  - **Test Suite**: 38 tests passing green (14 classifier tests, 8 correlator tests, 7 TUI tests, 5 watcher tests, 4 lib unit tests). Clippy runs with 0 warnings.

---

## 2. Currently Working On

- **Active file/task**: Fully completed 26-category expansion and generic hero animation system per design spec.
- **Next expected step**: Ready for user review and live streaming.

---

## 3. Decisions Log (not already captured elsewhere)

- Treated `overview.md` as the approved, authoritative architecture source — PRD/Architecture/rules/phases/design were derived from it rather than re-deriving the design from scratch, since it already includes a corrections log and explicit v1/v2 boundary.
- Target users (PRD §2) were inferred from the problem framing in `overview.md` §1 (AI CLI agent users on Linux who won't grant elevated privileges) — not independently specified elsewhere. Flag if this inference is wrong.
- `design.md`'s "fonts"/"typography scale" requirements were reinterpreted for a terminal-UI context (terminal font recommendations + Ratatui style/border hierarchy) since Agent-Radar has no GUI font-rendering layer.

---

## 4. Open Questions / Flags for the User

- Confirm target users (PRD.md §2) match actual intent — currently inferred from the problem framing in overview.md §1.
- Confirm the color palette in design.md §1.2 reads well in the user's actual terminal color profile (TrueColor rendering can shift slightly across terminal emulators).
- Git repository is initialized with master branch, tracking all v1 implementation work and packaging files.
