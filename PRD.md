# PRD.md — Product Requirements Document: Agent-Radar

> Source of truth for architecture: `overview.md` (Agent-Radar v1 Architecture Specification). This PRD translates that spec into product terms: who it's for, what it does, and what "done" means for v1.

---

## 1. What We Are Building

**Agent-Radar** is a real-time, zero-privilege terminal HUD (heads-up display) that runs alongside AI coding CLI agents (Claude Code, Aider, Cursor CLI, Antigravity, etc.) and visualizes what they're actually doing on disk — file edits, model checkpoint writes, dependency/dataset downloads — in a readable, cyberpunk-styled TUI instead of raw stdout scroll.

It is **not** a wrapper, proxy, or plugin for any specific agent. It is a standalone, unprivileged observer process that watches a project directory via the Linux kernel's `inotify` API, correlates filesystem events to the process tree via `procfs`, classifies the activity semantically, and renders it live via Ratatui in a terminal pane (typically a `tmux` split next to the agent).

**v1 explicitly ships on inotify + epoll only.** No root, no `CAP_BPF`, no `setcap`, no kernel module. Elevated-privilege tiers (fanotify, eBPF) and richer integrations (MCP/OpenTelemetry socket, Kitty graphics protocol) are deferred to v2 — see `overview.md` §9.2 and `phases.md` Phase 8.

### 1.1 Problem Statement

When an AI coding agent runs autonomously for minutes at a time, its CLI output is either too sparse (silent while doing a lot) or too noisy (verbose logs burying the signal) to tell, at a glance:

- Is it still working, or has it stalled?
- Is it editing source code, or pulling down a multi-GB dataset?
- Is a checkpoint write in progress that shouldn't be interrupted?

Agent-Radar exists to answer those questions passively, from outside the agent process, without requiring the agent to cooperate, instrument itself, or run with elevated privileges.

### 1.2 Non-Goals (v1)

- Not a security/audit tool — it does not sandbox, block, or restrict agent behavior.
- Not a network traffic inspector beyond the coarse socket-vs-disk fd check in `overview.md` §3.2.
- Not cross-platform — v1 is Linux-only (depends on `inotify`/`procfs`).
- Not a logging/telemetry aggregation service — no socket listener, no remote export (that's v2's MCP/OTel tier).

---

## 2. Target Users

| Segment | Description | Why Agent-Radar matters to them |
|---|---|---|
| **Primary: AI-pair-programming developers** | Devs running Claude Code, Aider, Cursor CLI, or similar agents against real repos, often letting the agent run unattended for stretches | Need a passive "what's it doing right now" view without reading agent logs |
| **Secondary: ML/dataset-heavy developers** | Developers whose agents fetch datasets or write model checkpoints as part of a workflow | Care specifically about distinguishing `INCOMING_DATA_STREAM` / `MODEL_TRAINING_CHECKPOINT` from ordinary file edits |
| **Tertiary: privacy/security-conscious power users** | Linux users who will not grant `sudo`/`CAP_BPF` to a third-party tool as a matter of policy | The "zero-privilege" guarantee (overview.md §1) is the actual purchase decision for this group — v1 must never silently require elevated capabilities |

All three segments are assumed to be comfortable with a terminal, `tmux`/split panes, and installing a CLI binary (curl script, `cargo install`, or a distro package). No GUI or web dashboard is in scope for v1.

---

## 3. Core Features (v1)

Features are grouped by the subsystem that implements them (see `Architecture.md` for the technical breakdown).

### 3.1 Unprivileged Filesystem Watching
- Recursive directory watching via `inotify`, emulated in userspace (no native recursive flag) with a create-then-watch race closer.
- Watch-budget guard: warns and degrades gracefully instead of crashing if `fs.inotify.max_user_watches` is exceeded.
- Ignores noise directories by default (`.git`, `node_modules`, `__pycache__`, `target`).

### 3.2 AI Agent Attribution
- Maps the process that touched a file back to its root AI-CLI ancestor (`claude`, `aider`, `python`, `node`, etc.) via `/proc/<pid>/cmdline` + `/proc/<pid>/ppid`, bounded by a liveness-checked LRU cache.
- Distinguishes "AI Agent Activity" from "Unrelated Background I/O."

### 3.3 Semantic Classification
- Debounces rapid writes (~300–500ms) and pairs renames via inotify's cookie, so atomic saves (write-temp + rename) are seen as one logical event, not a delete+create pair.
- Ordered, path-context-aware rule table (not a flat extension map) producing one of: `IncomingDataStream`, `ModelTrainingCheckpoint`, `SourceCodeMutation`, `WorkspaceExpansion`, `FileMutation`, `SystemIdle`.
- Distinguishes active network streams from local disk mutation via `/proc/<pid>/fd/` socket inspection.

### 3.4 Cyberpunk Terminal HUD
- Event-driven Ratatui render loop (never a fixed-Hz timer at idle).
- Layout: top status bar, a "hero card" for the currently dominant activity (animated), a scrolling log feed, and a footer with the tool's own resource usage.
- TrueColor (24-bit) rendering via Crossterm; see `design.md` for the palette.

### 3.5 Operational Basics
- Single-instance enforcement via `flock`.
- Graceful `SIGWINCH` (resize), `SIGTERM`/`SIGINT` (shutdown) handling with guaranteed terminal state restoration.
- Self resource telemetry (own CPU%, RSS, tracked PID count) shown in the footer — the tool proves its own lightness live.

### 3.6 Distribution
- Static binary (`x86_64`/`aarch64`-linux-musl).
- One-line curl installer, `cargo install agent-radar`, AUR package, `.deb`, `.rpm`.

---

## 4. Success Criteria (v1)

Directly inherited from `overview.md` §8 — these are the product's actual acceptance criteria, not aspirational marketing numbers:

| Metric | Target |
|---|---|
| Idle CPU | ~0% (blocked on `epoll_wait(-1)`, no scheduled wakeups) |
| RSS | < 10 MB baseline (stretch < 8 MB) |
| Event latency (inotify → rendered) | < 10–15 ms typical |
| Redraw rate | Event-driven; 0/sec at idle, burst cap ~60/sec |
| File descriptors held | ≤ 10 |
| Privilege required | None — runs as a normal unprivileged user process, always |

A v1 build that requires `sudo`, `setcap`, or a kernel capability under any default code path is a failed build, full stop — this is the one requirement that cannot be relaxed through the roadmap (see `rules.md`).

---

## 5. Open Assumptions

These are inferred from `overview.md` since it is an architecture document, not a product brief. Flag if any of these are wrong:

- Target OS is desktop/server Linux with a standard inotify-capable kernel (not WSL1, not restricted containers without inotify).
- Users run Agent-Radar themselves, pointed at their own project directory — no multi-user or remote-monitoring scenario in v1.
- "Done" for v1 is a working, packaged, installable binary meeting the §8 SLAs — not a specific feature-complete date.
