# Agent-Radar: 26-Category Expansion & Generic Hero-Card Animation — Design Spec

**Status:** Approved by user (categories, animation scope, tone, ignore-list policy). Ready for implementation.
**Scope:** Architectural — touches the event model, classification rule engine, debounce/burst logic, and the TUI hero-card rendering system.

## 1. Goal

Today `ActivityCategory` (src/events.rs) has 6 variants. Only 2 of them (`ModelTrainingCheckpoint`, `IncomingDataStream`) trigger the animated "hero card" on the TUI's left panel; the other 4 only ever appear as plain text rows in the log feed. The user wants **far more precise classification** (26 categories, up from 6) **and every single category to get its own animated hero-card treatment** — staying in the existing cyberpunk TrueColor aesthetic, but with a distinct icon, color, and funny all-caps caption per category.

## 2. Non-goals / hard constraints (confirmed with user)

- **Do not start watching `.git`, `node_modules`, `__pycache__`, or `target`.** These stay fully un-watched (no inotify watch ever placed inside them), exactly as today. Action-based categories (git, dependency lockfiles, tests) must be derivable from files in the *visible* working tree and/or process attribution — never from peeking inside ignored directories.
- **Do not regress the v1 SLAs**: idle CPU ~0%, RSS <10MB, FD count ≤10, zero privilege. All new logic must be O(event) or O(debounce-flush), never O(all processes) on a hot path.
- **Keep the existing 6 semantics where they still fit** (`ModelTrainingCheckpoint`, `IncomingDataStream`, `WorkspaceExpansion`, `FileMutation`, `SystemIdle` are kept as-is or as the final catch-all; only `SourceCodeMutation` is fully retired, split across Group A).
- All 26 variants of `ActivityCategory` must be handled in every `match` (`theme.rs`, `log_feed_table.rs`, hero rendering, rules.rs) — rely on the Rust compiler's exhaustiveness checking to catch gaps; never add a `_ => ...` catch-all that would silently hide a missing category.

## 3. Full category taxonomy (26 + SystemIdle)

Each row: **Category name** — detection rule — primary glyph (animation-safe Unicode symbol, NOT emoji — see §6.3) — caption (all-caps, funny) — color (hex) — animation primitive (see §6).

### Group A — Code & Docs (replaces today's single `SourceCodeMutation`)

| # | Category | Detection | Glyph | Caption | Color | Primitive |
|---|---|---|---|---|---|---|
| 1 | `RustEdit` | `.rs` | `⚡` / `✦` | NEURAL INK FLOWING | `#39FF88` | wave |
| 2 | `PythonEdit` | `.py` | `≈` / `~` | PYTHON ON THE PROWL | `#C9D94A` | flow-arrow |
| 3 | `WebEdit` | `.ts .tsx .js .jsx` | `✦` / `✧` | WEB SPUN TIGHTER | `#FFD166` | spinner |
| 4 | `StyleEdit` | `.css .scss .less` | `❖` / `◇` | PIXELS GETTING PAINTED | `#FF6FB5` | wave |
| 5 | `MarkupEdit` | `.html` | `▦` / `▧` | SCAFFOLDING RAISED | `#E8702A` | bounce-bar |
| 6 | `ConfigEdit` | `.toml .yaml .yml .json` (generic, not CI/container/model — see priority in §5) | `⚙` / `✲` | BOLTS BEING TIGHTENED | `#9AA7B8` | pulse-dot |
| 7 | `DocsEdit` | `.md .rst .txt` | `✎` / `✏` | SCRIBE AT WORK | `#D8C28A` | pulse-dot |
| 8 | `ShellScriptEdit` | `.sh .bash .zsh` | `$` / `❯` | SHELL INCANTATION CAST | `#5CF0C2` | spinner |

### Group B — Data & Media (previously generic `FileMutation`)

| # | Category | Detection | Glyph | Caption | Color | Primitive |
|---|---|---|---|---|---|---|
| 9 | `ImageAsset` | `.png .jpg .jpeg .gif .svg .webp` | `▣` / `▢` | CANVAS SPLATTERED | `#FF8AD8` | wave |
| 10 | `AudioAsset` | `.mp3 .wav .flac` | `♪` / `♫` | SOUNDWAVE RIPPLING | `#8AD8FF` | wave |
| 11 | `VideoAsset` | `.mp4 .mov .webm` | `▶` / `▷` | REEL ROLLING | `#B58AFF` | spinner |
| 12 | `FontAsset` | `.ttf .otf .woff .woff2` | `Æ` / `æ` | GLYPHS FORGED | `#FFD98A` | pulse-dot |
| 13 | `NotebookActivity` | `.ipynb` | `≡` / `≣` | LAB NOTEBOOK SCRIBBLED | `#8AFFC2` | bounce-bar |

### Group C — AI/ML (expands today's checkpoint logic)

| # | Category | Detection | Glyph | Caption | Color | Primitive |
|---|---|---|---|---|---|---|
| 14 | `ModelTrainingCheckpoint` *(kept)* | `.pt .safetensors .ckpt .onnx`, or `.bin` in model-context path | `●` / `◈` | TENSOR FLUSH IN PROGRESS | `#B24BFF` | bounce-bar *(unchanged from today)* |
| 15 | `ModelConfigEdit` | `.json`/`.yaml` **inside** a model-context path (same `is_checkpoint_path_context` check reused, but non-weight file) | `≈` / `∿` | SYNAPSES REWIRED | `#D48BFF` | spinner |

### Group D — Network / Transfer (expands today's data-stream logic)

| # | Category | Detection | Glyph | Caption | Color | Primitive |
|---|---|---|---|---|---|---|
| 16 | `IncomingDataStream` *(kept)* | archive/dataset ext. **while `active_network_stream`** | `▼` / `▽` | SIGNAL BEING SUCKED DOWN | `#00F0FF` | flow-arrow *(unchanged from today)* |
| 17 | `ArchiveWrite` | same extensions, **no active socket** (was silently `FileMutation` before) | `▪` / `▫` | BOX TAPED SHUT | `#4FA8E8` | pulse-dot |

### Group E — Process/action-based (new signal: filename pattern + git-process attribution)

| # | Category | Detection | Glyph | Caption | Color | Primitive |
|---|---|---|---|---|---|---|
| 18 | `GitOperation` | write attributed to a process whose name/ancestor is literally `git`, touching a tracked working-tree file | `Y` / `⑂` | TIMELINE BRANCHING | `#FF8C42` | flow-arrow |
| 19 | `DependencyLockUpdate` | filename exactly `package-lock.json`, `Cargo.lock`, `yarn.lock`, `pnpm-lock.yaml`, `poetry.lock`, `Gemfile.lock` | `≡` / `⚓` | ANCHOR CHAIN RATTLING | `#6B8CFF` | spinner |
| 20 | `TestFileActivity` | filename matches `test_*.*`, `*_test.*`, `*.spec.*`, or path contains a `tests/`/`test/` component | `✓` / `✗` | BUG HUNT IN PROGRESS | `#FFA13C` | bounce-bar |
| 21 | `EnvSecretChange` | filename is `.env` or starts with `.env.` | `▓` / `▒` | VAULT DOOR CREAKING | `#FF3C6E` | pulse-dot (slow) |
| 22 | `CiPipelineEdit` | path contains `.github/workflows/` + `.yml`/`.yaml`, or filename `.gitlab-ci.yml` | `⚙` / `⟲` | ROBOT ARMS RECALIBRATED | `#8A8AFF` | spinner |
| 23 | `ContainerConfigEdit` | filename `Dockerfile`, `Dockerfile.*`, `docker-compose.yml`/`.yaml` | `▢` / `◫` | WHALE SURFACING | `#3C9AFF` | wave |

### Group F — Structural

| # | Category | Detection | Glyph | Caption | Color | Primitive |
|---|---|---|---|---|---|---|
| 24 | `WorkspaceExpansion` *(kept)* | new directory created | `▲` / `△` | NEW WING UNDER CONSTRUCTION | `#FFC247` | bounce-bar |
| 25 | `MassDeletion` | ≥5 deletions in one 300ms burst window (see §5.2) | `✕` / `×` | CONTROLLED DEMOLITION | `#FF2E2E` | bounce-bar (fast) |
| 26 | `FileMutation` *(kept, final catch-all)* | anything not matched above | `•` / `◦` | SOMETHING STIRRED | `#7A8BA6` | spinner (slow) |

### System state (not animated)

`SystemIdle` — unchanged. Static `IdleCard`, no primitive, color `#3A4254`, label "SYSTEM QUIET".

## 4. Data model changes (`src/events.rs`)

```rust
pub enum ActivityCategory {
    RustEdit { path: String },
    PythonEdit { path: String },
    WebEdit { path: String },
    StyleEdit { path: String },
    MarkupEdit { path: String },
    ConfigEdit { path: String },
    DocsEdit { path: String },
    ShellScriptEdit { path: String },
    ImageAsset { path: String },
    AudioAsset { path: String },
    VideoAsset { path: String },
    FontAsset { path: String },
    NotebookActivity { path: String },
    ModelTrainingCheckpoint { path: String, size_bytes: u64 },
    ModelConfigEdit { path: String },
    IncomingDataStream { path: String, bytes_per_sec: u64, progress_pct: Option<u8> },
    ArchiveWrite { path: String },
    GitOperation { path: String },
    DependencyLockUpdate { path: String },
    TestFileActivity { path: String },
    EnvSecretChange { path: String },
    CiPipelineEdit { path: String },
    ContainerConfigEdit { path: String },
    WorkspaceExpansion { path: String },
    MassDeletion { count: usize, sample_paths: Vec<String> },
    FileMutation { path: String, op: FileOp },
    SystemIdle,
}
```

Note: the per-language code categories (RustEdit, PythonEdit, WebEdit, etc.) drop the `lines_added`/`lines_removed` fields the old `SourceCodeMutation` had — **decision needed from implementer**: either (a) add `lines_added`/`lines_removed` to every Group A variant (more code, consistent line-diff feature preserved per-language), or (b) keep line-diffing as a feature of a shared helper that all Group A variants call into but only surface in `format_details` for these specific variants. Recommendation: **(a)**, add the fields — it's mechanical and keeps the existing line-diff feature intact per language rather than regressing it.

New field on `EnrichedEvent` (`src/correlator/mod.rs`):
```rust
pub is_git_process: bool,
```
Computed in `ProcessCorrelator::correlate`: after resolving `effective_pid`'s `CachedPidInfo`, check `info.name == "git"` (exact match on the process `comm` name, case-sensitive — `git` is not matched via the existing `agent_patterns` list, which is reserved for AI-CLI-agent detection and must not be polluted with this).

## 5. Classification rule engine (`src/classifier/rules.rs`)

### 5.1 Priority order (most-specific first; mirrors today's existing `.bin` disambiguation pattern)

1. **Directory create** → `WorkspaceExpansion` (unchanged, highest priority)
2. **Filename-exact matches** (checked before any extension logic): `.env`/`.env.*` → `EnvSecretChange`; `Dockerfile`/`Dockerfile.*`/`docker-compose.y*ml` → `ContainerConfigEdit`; lockfile names → `DependencyLockUpdate`; `.gitlab-ci.yml` or path contains `.github/workflows/` → `CiPipelineEdit`
3. **Test-pattern match** (filename `test_*`/`*_test.*`/`*.spec.*`, or path has `tests/`/`test/` component) → `TestFileActivity` — checked **before** the per-language code splits, so `tests/test_foo.py` is `TestFileActivity`, not `PythonEdit`
4. **Process-attributed**: `enriched.is_git_process` → `GitOperation` (checked before per-language splits too, so a `git checkout` touching `src/main.rs` shows as `GitOperation`, not `RustEdit`)
5. **Checkpoint extensions** (`.pt/.safetensors/.ckpt/.onnx`) → `ModelTrainingCheckpoint` (unchanged)
6. **`.bin` disambiguation** (unchanged existing `is_checkpoint_path_context` check): checkpoint context → `ModelTrainingCheckpoint`; active network → `IncomingDataStream`; else → `FileMutation`
7. **Model-config context**: `.json`/`.yaml`/`.yml` **and** `is_checkpoint_path_context(path)` is true → `ModelConfigEdit` (checked before generic `ConfigEdit`)
8. **Archive/dataset extensions**: active network → `IncomingDataStream` (unchanged); else → `ArchiveWrite` (was silently `FileMutation` before — now a named category)
9. **Media extensions** → `ImageAsset`/`AudioAsset`/`VideoAsset`/`FontAsset`/`NotebookActivity`
10. **Per-language code extensions** → `RustEdit`/`PythonEdit`/`WebEdit`/`StyleEdit`/`MarkupEdit`/`ShellScriptEdit`/`DocsEdit`/`ConfigEdit` (generic `.toml/.yaml/.yml/.json` not already claimed by model-config/CI/container tiers above)
11. **Catch-all** → `FileMutation` (unchanged)

`MassDeletion` is **not** part of this per-event function — it's decided at the debounce/burst layer before `evaluate_rules` is even called (see §5.2).

### 5.2 MassDeletion burst detection (`src/classifier/mod.rs`)

New field on `SemanticClassifier`:
```rust
delete_burst: Vec<(PathBuf, EnrichedEvent)>,
burst_last_seen: Option<Instant>,
```

Logic in `push_event`, replacing today's immediate-emit-on-DELETE branch:
- On a `DELETE` event: push `(path, enriched)` into `delete_burst`, update `burst_last_seen = Some(now)`, return `None` (defer, don't emit yet — this is a behavior change from today's immediate emission).
- In `flush_ready` (called every loop iteration already): if `burst_last_seen` is `Some(t)` and `now.duration_since(t) >= debounce_window` (reuse the existing 300ms constant):
  - if `delete_burst.len() >= 5`: emit **one** `MassDeletion { count: delete_burst.len(), sample_paths: delete_burst.iter().take(3).map(path_to_string) }` event (classify via the *first* buffered event's `enriched` for process/pid attribution)
  - else: emit each buffered delete individually via `classify_direct(.., FileOp::Deleted)`, exactly as today
  - clear `delete_burst`, reset `burst_last_seen = None`

This adds at most 300ms of latency to delete-event visibility (same latency single-file writes already get from the existing debounce), and bounds memory by the same practical limits as the existing debounce map.

## 6. Hero card generalization (`src/tui/mod.rs`, `src/tui/widgets/hero_stream.rs`, `src/tui/theme.rs`)

### 6.1 Replace the 2 hardcoded variants with 1 generic one

```rust
pub enum HeroCardType {
    Active {
        category: ActivityCategory,
        started_at: Instant,
        last_updated: Instant,
        frame: u64,
    },
    IdleCard,
}
```
`is_animating()`/`advance_frame()`/the "preserve `started_at` across updates of the same thing" logic from the earlier bug fix all collapse to a single match arm instead of two near-duplicates.

### 6.2 `category_visual()` table (new, in `theme.rs`)

```rust
pub struct CategoryVisual {
    pub glyph_a: char,
    pub glyph_b: char,
    pub caption: &'static str,
    pub color: Color,
    pub primitive: AnimationPrimitive,
}

pub enum AnimationPrimitive { PulseDot, FlowArrow, BounceBar, Spinner, Wave }

pub fn category_visual(cat: &ActivityCategory) -> CategoryVisual { /* one entry per variant, exhaustive match */ }
```
This is the single source of truth mapping every category to its look — the table encodes exactly the §3 rows above. `category_color()`/`category_label()` (used by the log feed) can be rewritten as thin wrappers over `category_visual()` so there's only one place to update per category, not three.

### 6.3 Glyph choice: Unicode symbols, not emoji, for the animated glyph itself

Emoji are kept **only** as flavor inside the all-caps caption strings if desired later (e.g. a user-facing doc), never as the thing that gets measured/positioned every animation frame — `UnicodeWidthChar` support for emoji (especially ZWJ sequences / variation selectors) is inconsistent across terminals, and this is exactly the class of bug the earlier Unicode-truncation fix (`unicode_util.rs`) had to clean up once already. All glyphs in §3's table are single-width-safe Unicode symbols (`⚡ ≈ ✦ ❖ ▦ ⚙ ✎ $ ▣ ♪ ▶ Æ ≡ ● ◈ ▼ ▽ ▪ Y ✓ ▓ ▢ ▲ ✕ •` etc.) — no emoji in the actual rendered animation.

### 6.4 The 5 animation primitives (`hero_stream.rs`)

Each primitive is a small `fn(frame: usize, width: usize, glyph_a: char, glyph_b: char) -> String` building one line of animated text, parameterized by the category's two glyphs:

- **PulseDot**: alternates `glyph_a`/`glyph_b` on `frame % 2` (reuses today's checkpoint-card logic exactly — already implemented, just needs to take glyphs as params instead of being hardcoded).
- **FlowArrow**: reuses today's `build_stream_bar` exactly, but the traveling marker is `glyph_a` instead of hardcoded `►`.
- **BounceBar**: reuses today's `build_pulse_bar` exactly, but the three gauge states (`░`/`▓`/filled) use `glyph_b`/mid/`glyph_a` instead of hardcoded `░▓█`.
- **Spinner**: classic 4-frame rotation `['◐','◓','◑','◒']` cycling on `frame % 4`, glyphs unused (rotation look doesn't need category-specific chars — keeps this primitive visually distinct from the other 4).
- **Wave**: a `width`-long line where each column's "height" character cycles through `▁▂▃▄▅▆▇█▇▆▅▄▃▂▁` phase-shifted by `(column + frame) % 16`, tinted by the category color.

A category's hero card renders: border (category color) → caption line (glyph_a + caption text) → path line (Unicode-safe truncated, reusing `unicode_util::truncate_to_width`) → the primitive's animated line. This replaces the current bespoke per-variant rendering blocks in `hero_stream.rs` with one generic render path keyed off `category_visual()`.

## 7. Testing plan

- **Rule-priority tests** (`classifier_tests.rs`): one test per priority tier proving the more-specific category wins — e.g. `tests/test_foo.py` → `TestFileActivity` not `PythonEdit`; a git-attributed write to `src/main.rs` → `GitOperation` not `RustEdit`; `.env.production` → `EnvSecretChange`; `config/model/model_config.json` → `ModelConfigEdit` not generic `ConfigEdit`.
- **Extension coverage tests**: table-driven test iterating every Group A/B/D extension asserting the expected category.
- **MassDeletion burst test**: 4 deletes in one window → 4 individual `FileMutation(Deleted)` events; 5+ → exactly 1 `MassDeletion` event with correct `count`.
- **`is_git_process` test** (`correlator_tests.rs`): a process named `git` resolves `is_git_process: true`; everything else `false`.
- **Hero-card generalization tests** (`tui_tests.rs`): the generic `Active` variant correctly preserves `started_at` across repeated updates of the *same* category (regression test for the bug fixed earlier in this project), and correctly expires to `IdleCard` after 10s, for an arbitrary category (not just the 2 that existed before).
- **Exhaustiveness is enforced by the compiler**: every `match` over `ActivityCategory` (rules.rs reverse-lookup if any, theme.rs, log_feed_table.rs, hero_stream.rs) must NOT have a wildcard `_` arm — if a 27th category is ever added later and a match is missed, it should fail to compile, not silently misrender.
- Full regression: existing 33 tests must continue passing unchanged in spirit (some will need updating since `SourceCodeMutation` no longer exists — e.g. `test_source_code_mutation_and_line_diff` becomes `test_rust_edit_and_line_diff` or similar, same assertions against the new variant name).

## 8. Acceptance criteria

- [ ] All 26 categories + `SystemIdle` exist as `ActivityCategory` variants with no compiler warnings about unused variants.
- [ ] Every category has a `category_visual()` entry (color, 2 glyphs, caption, primitive) — compiler-enforced exhaustive match.
- [ ] Priority order in §5.1 is implemented and covered by tests proving more-specific wins over generic.
- [ ] `MassDeletion` burst logic implemented exactly as §5.2; verified by test.
- [ ] Hero card renders a distinct animation for every one of the 26 active categories (manually spot-checked in a real terminal for at least: one from each of the 5 primitives).
- [ ] `.git`, `node_modules`, `__pycache__`, `target` remain completely unwatched — no new watch calls reference them anywhere in the diff.
- [ ] `cargo test` all green, `cargo clippy --all-targets` zero warnings, idle-CPU/RSS/FD SLAs re-verified on the final binary (same method as the original Phase 6 verification).

## 9. Open implementation decision for the executor

§4 flagged one decision not yet made: whether Group A code-edit categories keep per-category `lines_added`/`lines_removed` fields (recommended) or drop line-diffing. Pick (a) (keep it) unless there's a strong reason found during implementation to do otherwise, and note the choice in the PR/commit description.
