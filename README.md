# Agent-Radar (v1)

> **Real-Time Terminal Telemetry HUD for AI Coding Agents**  
> *Zero privilege escalation. ~0% idle CPU. <10MB RSS. Single static binary.*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Target: static-musl](https://img.shields.io/badge/Target-musl--static-brightgreen.svg)]()
[![Privilege: Unprivileged](https://img.shields.io/badge/Privilege-0%20(None)-blueviolet.svg)]()

---

## 1. What is Agent-Radar?

When AI coding assistants (**Claude Code**, **Aider**, **Antigravity**, **Cursor CLI**) execute complex background operations — like batch file edits, tensor checkpointing, or downloading datasets — standard CLI stdout logs quickly produce telemetry overload.

**Agent-Radar** is an unprivileged, ultra-lightweight workspace activity HUD that runs alongside your AI agents (typically inside a `tmux` split). It passively monitors your workspace via Linux **inotify**, correlates events against the process tree via **procfs**, classifies the activity semantically, and renders a live, cyberpunk-styled terminal visualization using **Ratatui**.

```
┌───────────────────────────────── AGENT-RADAR // HUD ──────────────────────────────────┐
│ WATCH: /home/user/project        AGENT: claude-code                  [ONLINE]         │
├──────────────────────────────────────┬────────────────────────────────────────────────┤
│ ╔═ MODEL TRAINING CHECKPOINT ═╗      │ TIME     AGENT / PID          CATEGORY         │
│ ● TENSOR FLUSH IN PROGRESS           │ 23:42:01 claude (23145)       CHECKPOINT       │
│ Path: checkpoints/epoch-04.pt        │ 23:41:59 claude (23145)       CODE_MUTATION    │
│ Size: 812.45 MB  |  Active: 3s       │ 23:41:40 curl (24102)         DATA_STREAM      │
│ [░░░░░░░░░░▓▓██▓▓░░░░░░░░░░]         │ 23:40:12 claude (23145)       EXPANSION        │
├──────────────────────────────────────┴────────────────────────────────────────────────┤
│ AGENT-RADAR v0.1.0  |  RSS: 4.38 MB  |  CPU: ~0.0%  |  PIDS: 1  |  [Ctrl-C to Exit]   │
└───────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Key Features

- **Zero Privilege Escalation**: Runs as a standard, unprivileged Linux process. No `sudo`, no `setcap`, no kernel modules, no BPF capabilities required.
- **Genuine ~0% Idle CPU**: The synchronous `epoll` reactor sleeps indefinitely on `epoll_wait(-1)` when idle. Animation timers arm only during active bursts.
- **Micro Memory Footprint (<5 MB RSS)**: No async runtime (zero Tokio/async-std overhead). Employs bounded LRU caches with strict 30s TTL eviction.
- **Smart Semantic Classification**:
  - **Model Training Checkpoints**: Detects `.pt`, `.safetensors`, `.ckpt`, `.onnx`.
  - **.bin Disambiguation**: Path-context predicates resolve `.bin` files as tensor shards (via sibling `*.index.json` or `models/`/`checkpoints/` directories) vs downloads.
  - **Incoming Data Streams**: Correlates socket descriptors (`/proc/<pid>/fd/`) with file writes for `.parquet`, `.zip`, `.tar.gz`.
  - **Source Code Line Diffing**: Debounces writes (300ms) before computing clean line delta metrics (`+42 / -11`).
  - **Atomic Save Handling**: Pairs `IN_MOVED_FROM` and `IN_MOVED_TO` by inotify cookie to treat atomic editor writes as single logical replacements.
  - **Workspace Expansion**: Real-time directory creation tracking with race-closing directory rescans.

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

### Option 1: One-Line Curl Installer (Recommended)
```bash
curl -fsSL https://raw.githubusercontent.com/agent-radar/agent-radar/main/install.sh | sh
```

### Option 2: Build From Source (git clone)
```bash
git clone https://github.com/agent-radar/agent-radar.git
cd agent-radar
cargo build --release
./install.sh    # copies target/release/agent-radar onto your PATH
```

### Option 3: Cargo (Directly From GitHub)
```bash
cargo install --git https://github.com/agent-radar/agent-radar.git
```

### Option 4: Arch Linux (AUR)
```bash
yay -S agent-radar-bin
```

### Option 5: Static Musl Binary Build
```bash
# Build fully static PIE binary with zero dynamic library dependencies
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

### Requirements

- Linux (uses `inotify` + `/proc`) — x86_64 or aarch64
- Rust 1.75+ (only needed when building from source)

---

## 5. Visual Design System

Agent-Radar employs a TrueColor 24-bit cyberpunk aesthetic mapped 1-to-1 to semantic activity categories:

| Category | Accent Color | Hex | Semantic Meaning |
|---|---|---|---|
| `IncomingDataStream` | Electric Cyan | `#00F0FF` | Active network download / dataset stream |
| `ModelTrainingCheckpoint` | Neon Magenta | `#B24BFF` | Tensor checkpoint / weight flush |
| `SourceCodeMutation` | Acid Green | `#39FF88` | Source code mutation & line deltas |
| `WorkspaceExpansion` | Amber | `#FFC247` | New directory created |
| `FileMutation` | Slate Blue-Gray | `#7A8BA6` | Low-salience file operations |
| `SystemIdle` | Dim Gray | `#3A4254` | Quiet state, waiting for events |
| `Warning` | Burnt Orange-Red | `#FF5A3C` | Watch budget warning |

### Recommended Terminal Fonts
For optimal rendering of box drawing and status glyphs:
1. **JetBrains Mono Nerd Font** (Preferred)
2. **FiraCode Nerd Font**
3. **Hack Nerd Font**

*(Standard Unicode fallback is included automatically for terminals without Nerd Fonts).*

---

## 6. Performance Guarantees & SLA Verification

Empirically verified on Linux x86_64:

| Metric | Target SLA | Measured Baseline |
|---|---|---|
| **Idle CPU** | ~0.0% | **0.0%** (blocks on `epoll_wait(-1)`) |
| **Active Burst CPU** | < 1.0% | **0.6%** during 50-file rapid burst |
| **Resident Set Size (RSS)**| < 10 MB (stretch < 8 MB) | **4.38 MB** |
| **File Descriptors (FDs)** | Max 10 FDs | **9 FDs** |
| **Event Latency** | < 10–15 ms | **< 1 ms** dispatch |
| **Redraw Mechanism** | Event-driven diffing | Changed terminal cells only |
| **Privilege Level** | 0 (Unprivileged) | Standard user permissions |

---

## 7. Development

```bash
git clone https://github.com/agent-radar/agent-radar.git
cd agent-radar
cargo build              # debug build
cargo test                # unit + integration test suite
cargo run --release --example scene_gallery   # browse every hero-card animation
```

Distro packaging manifests (AUR, `.deb`, RPM) live under [`packaging/`](packaging/README.md).

## 8. Contributing

Issues and pull requests are welcome. Please run `cargo test` and `cargo build --release` before opening a PR.

## 9. License

Licensed under the [MIT License](LICENSE).
