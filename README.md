# Agent-Radar (v1)

> **Real-Time Terminal Telemetry HUD for AI Coding Agents**
> *Zero privilege escalation. Idle on `epoll_wait`. Single static binary.*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Target: static-musl](https://img.shields.io/badge/Target-musl--static-brightgreen.svg)]()
[![Privilege: Unprivileged](https://img.shields.io/badge/Privilege-0%20(None)-blueviolet.svg)]()

---

## 1. What is Agent-Radar?

When AI coding assistants (**Claude Code**, **Aider**, **Antigravity**, **Cursor**, **Copilot**) execute complex background operations — batch edits, tensor checkpointing, downloading datasets, running tests — standard CLI stdout logs quickly produce telemetry overload.

**Agent-Radar** is an unprivileged, lightweight workspace activity HUD that runs alongside your AI agents (typically inside a `tmux` split). It passively watches your workspace via Linux **inotify**, correlates events against the process tree via **procfs**, classifies the activity semantically into one of ~26 categories, and renders a live, cyberpunk-styled terminal visualization using **Ratatui**.

```
┌───────────────────────────────── AGENT-RADAR // HUD ──────────────────────────────────┐
│ WATCH: /home/user/project        AGENT: claude-code                  [ONLINE]         │
├──────────────────────────────────────┬────────────────────────────────────────────────┤
│ ╔═ MODEL TRAINING CHECKPOINT ═╗      │ TIME     AGENT / PID          CATEGORY         │
│ ● TENSOR FLUSH IN PROGRESS           │ 23:42:01 claude (23145)       CHECKPOINT       │
│ Path: checkpoints/epoch-04.pt        │ 23:41:59 claude (23145)       RUST_EDIT        │
│ Size: 812.45 MB  |  Active: 3s       │ 23:41:40 curl (24102)         DATA_STREAM      │
│ [░░░░░░░░░░▓▓██▓▓░░░░░░░░░░]         │ 23:40:12 claude (23145)       EXPANSION        │
├──────────────────────────────────────┴────────────────────────────────────────────────┤
│ AGENT-RADAR v0.1.0  |  RSS: 4.38 MB  |  CPU: ~0.0%  |  PIDS: 1  |  [Ctrl-C to Exit]   │
└───────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Key Features

- **Zero Privilege Escalation**: Runs as a standard, unprivileged Linux process. No `sudo`, no `setcap`, no kernel modules, no BPF capabilities required.
- **Near-zero Idle CPU**: The synchronous `epoll` reactor sleeps indefinitely on `epoll_wait(-1)` when idle. Animation timers arm only during active bursts.
- **Small Memory Footprint**: No async runtime (zero Tokio/async-std overhead). Employs bounded LRU caches with TTL eviction for process/correlation state.
- **Agent-Aware Process Correlation**: Matches process names/cmdlines against known AI coding agents (Claude Code, Aider, Antigravity, Cursor, Copilot) so the HUD can attribute activity to the right tool, with PID-reuse protection via `/proc/<pid>/stat` start-time comparison.
- **Socket-Verified Data Streams**: `IncomingDataStream` detection cross-references a process's own `/proc/<pid>/fd` socket inodes against `/proc/net/{tcp,tcp6,udp,udp6}` connection state — it won't fire on a stale or unrelated socket.
- **~26-Category Semantic Classification**, grouped as:
  - **Group A — Code & Docs**: per-language edits with line-delta tracking (`RustEdit`, `PythonEdit`, `WebEdit`, `StyleEdit`, `MarkupEdit`, `ConfigEdit`, `DocsEdit`, `ShellScriptEdit`).
  - **Group B — Data & Media**: `ImageAsset`, `AudioAsset`, `VideoAsset`, `FontAsset`, `NotebookActivity`.
  - **Group C — AI/ML**: `ModelTrainingCheckpoint` (`.pt`, `.safetensors`, `.ckpt`, `.onnx`, with path-context disambiguation for ambiguous `.bin` shards vs. downloads) and `ModelConfigEdit`.
  - **Group D — Network / Transfer**: `IncomingDataStream` (socket-correlated download progress) and `ArchiveWrite` (`.zip`, `.tar.gz`, etc.).
  - **Group E — Process / Action-based**: `GitOperation`, `DependencyLockUpdate`, `TestFileActivity`, `EnvSecretChange`, `CiPipelineEdit`, `ContainerConfigEdit`.
  - **Group F — Structural**: `WorkspaceExpansion`, `MassDeletion`, generic `FileMutation`.
- **Atomic Save Handling**: Pairs `IN_MOVED_FROM` / `IN_MOVED_TO` by inotify cookie so atomic editor writes register as a single logical replacement, not a delete + create.
- **Create-Then-Watch Race Closing**: New directories are rescanned immediately after their watch is armed, so files created in the gap aren't missed.
- **Inotify Watch-Budget Awareness**: Tracks `/proc/sys/fs/inotify/max_user_watches` and warns once instead of silently dropping coverage or crashing when the budget is exhausted.

---

## 3. Quick Start

### Running in your workspace
```bash
# Watch the current directory
agent-radar

# Or watch a specific project path
agent-radar /path/to/project

# Run in debug logging mode (no alternate screen)
agent-radar --debug /path/to/project
```

### Recommended Tmux Setup
Run Agent-Radar in a vertical or horizontal split pane alongside your AI agent:
```bash
# Create split pane and run agent-radar
tmux split-window -h -p 35 "agent-radar"
```

---

## 4. Installation

### Option 1: One-Line Curl Installer
```bash
curl -fsSL https://raw.githubusercontent.com/bhanot-99/agent-radar/master/install.sh | sh
```

### Option 2: Build From Source (git clone)
```bash
git clone https://github.com/bhanot-99/agent-radar.git
cd agent-radar
cargo build --release
./install.sh    # copies target/release/agent-radar onto your PATH
```

### Option 3: Cargo (Directly From GitHub)
```bash
cargo install --git https://github.com/bhanot-99/agent-radar.git
```

### Option 4: Static Musl Binary Build
```bash
# Build fully static PIE binary with zero dynamic library dependencies
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

Distro packaging manifests (AUR, `.deb`, RPM) live under [`packaging/`](packaging/README.md) and are maintained there, not published yet — check that directory for current status before relying on a package manager install.

### Requirements

- Linux (uses `inotify` + `/proc`) — x86_64 or aarch64
- Rust 1.75+ (only needed when building from source)

---

## 5. Visual Design System

Agent-Radar employs a TrueColor 24-bit cyberpunk aesthetic, mapping every one of the ~26 activity categories to its own accent color and glyph pair (see `src/tui/theme.rs::category_visual` for the authoritative table). A representative sample:

| Category | Accent Color | Hex | Semantic Meaning |
|---|---|---|---|
| `IncomingDataStream` | Electric Cyan | `#00F0FF` | Active network download / dataset stream |
| `ModelTrainingCheckpoint` | Neon Magenta | `#B24BFF` | Tensor checkpoint / weight flush |
| `RustEdit` | Acid Green | `#39FF88` | Rust source mutation & line deltas |
| `PythonEdit` | Chartreuse | `#C9D94A` | Python source mutation & line deltas |
| `WorkspaceExpansion` | Amber | `#FFC247` | New directory created |
| `MassDeletion` | Alert Red | `#FF2E2E` | Large-scale file removal |
| `EnvSecretChange` | Hot Pink-Red | `#FF3C6E` | `.env` / secret file touched |
| `FileMutation` | Slate Blue-Gray | `#7A8BA6` | Low-salience file operations |
| `SystemIdle` | Dim Gray | `#3A4254` | Quiet state, waiting for events |

### Recommended Terminal Fonts
For optimal rendering of box drawing and status glyphs:
1. **JetBrains Mono Nerd Font** (Preferred)
2. **FiraCode Nerd Font**
3. **Hack Nerd Font**

*(Standard Unicode fallback is included automatically for terminals without Nerd Fonts.)*

---

## 6. Design Goals

Agent-Radar is built around a small set of hard constraints rather than a fixed performance SLA table:

| Property | Approach |
|---|---|
| **Idle CPU** | Single-threaded `epoll_wait(-1)` reactor — blocks indefinitely with no polling when there is no filesystem or timer activity. |
| **Active Burst CPU** | Debounced classification (300ms) and bounded LRU state avoid O(n²) rescans during rapid multi-file bursts. |
| **Memory** | No async runtime; state is held in small, TTL-evicted LRU caches rather than unbounded history. |
| **Privilege Level** | Standard user permissions only — no `sudo`, `setcap`, or kernel modules. |
| **Redraw Mechanism** | Ratatui's diffed buffer — only changed terminal cells are written. |

These are architectural properties you can verify by reading the code (`src/app_runner.rs` for the reactor loop, `src/correlator/mod.rs` for the LRU/TTL state) rather than numbers re-asserted here; if you benchmark your own build, numbers will vary by kernel, terminal emulator, and workload.

---

## 7. Development

```bash
git clone https://github.com/bhanot-99/agent-radar.git
cd agent-radar
cargo build              # debug build
cargo test                # unit + integration test suite
cargo clippy --all-targets   # lint
cargo run --release --example scene_gallery   # browse every hero-card animation
```

Distro packaging manifests (AUR, `.deb`, RPM) live under [`packaging/`](packaging/README.md).

## 8. Contributing

Issues and pull requests are welcome. Please run `cargo test`, `cargo clippy --all-targets`, and `cargo build --release` before opening a PR.

## 9. License

Licensed under the [MIT License](LICENSE).
