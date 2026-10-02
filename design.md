# design.md — Visual Design System: Agent-Radar

> Agent-Radar is a **terminal UI** (Ratatui + Crossterm, TrueColor 24-bit), not a GUI or web app. "Fonts" and "typography scale" below are adapted to what that actually means in a terminal context: the user's terminal emulator font choice, Unicode glyph/box-drawing usage, and Ratatui text styling (bold/dim/underline) in place of font-size steps.

---

## 1. Theme & Color Palette

**Theme: cyberpunk/sci-fi HUD** — dark background, neon accent colors mapped one-to-one to `ActivityCategory` variants so the user can recognize activity type by color alone, at a glance, before reading text.

### 1.1 Base Surface

| Token | Hex (TrueColor) | Usage |
|---|---|---|
| `bg.base` | `#0A0E14` | Terminal background / main surface |
| `bg.panel` | `#10151F` | Log feed table background, slightly lifted from base |
| `border.default` | `#2A3344` | Default box-drawing border color (dim, idle state) |
| `border.active` | `#00F0FF` | Border color for the actively-animating hero card |
| `text.primary` | `#D8E4F0` | Default readable text |
| `text.dim` | `#5C6B80` | Secondary text, timestamps, footer stats |

### 1.2 Category Accent Colors (semantic, maps to `ActivityCategory`)

| Category | Color | Hex | Rationale |
|---|---|---|---|
| `IncomingDataStream` | Electric cyan | `#00F0FF` | Signals "data flowing in," cold/network association |
| `ModelTrainingCheckpoint` | Neon magenta/violet | `#B24BFF` | Distinct, high-salience — checkpoint writes are often large/important |
| `SourceCodeMutation` | Acid green | `#39FF88` | Classic "code/terminal" green, lowest-alarm category (expected, frequent) |
| `WorkspaceExpansion` | Amber | `#FFC247` | New directories — "structure changing," medium salience |
| `FileMutation` (generic) | Slate blue-gray | `#7A8BA6` | Low-salience catch-all, intentionally desaturated |
| `SystemIdle` | Dim gray | `#3A4254` | Idle card — recedes visually, confirms "nothing wrong, just quiet" |
| Warning/degraded state (e.g. watch-budget exceeded) | Burnt orange-red | `#FF5A3C` | Reserved exclusively for genuine warnings — never reused for normal categories |

**Rule:** a color is never reused across two different semantic meanings. If a new `ActivityCategory` variant is added later, it gets a new, previously-unused hue — don't reuse `#FF5A3C` (warning) or any existing category color for anything else.

### 1.3 Contrast & Accessibility
- All text colors maintain ≥ 4.5:1 contrast against `bg.base`/`bg.panel` (verify with a contrast checker when finalizing exact hexes against the user's actual terminal profile — true-color rendering can vary slightly by terminal emulator color management).
- Category colors are chosen to remain distinguishable under common color-vision deficiencies (cyan/magenta/green/amber/gray spread rather than red/green-only contrast) — do not add a category color that depends on red/green discrimination alone.

---

## 2. Typography (Terminal Equivalent)

There is no font *file* bundled with Agent-Radar — rendering is whatever monospace font the user's terminal emulator is configured with. Design guidance instead covers:

### 2.1 Recommended Terminal Fonts (documentation only, not bundled)
Recommend in the README, not enforced in code:
1. **JetBrains Mono Nerd Font** — primary recommendation; excellent glyph coverage + ligature-free clarity at small sizes.
2. **FiraCode Nerd Font** — acceptable alternative, slightly more decorative ligatures.
3. **Hack Nerd Font** — fallback for users who want maximum plainness.

A "Nerd Font" variant is recommended (not required) because the header bar and hero cards use a small set of Nerd Font glyphs for status icons; **a plain Unicode fallback (box-drawing + basic symbols only, no Nerd Font glyphs) must always be available** so the HUD still renders correctly on an unmodified terminal font. Detect/fall back gracefully — never require the user to install a patched font to get a functional (if slightly plainer) HUD.

### 2.2 Typography Scale → Ratatui Style Equivalents

| Web/GUI concept | Terminal equivalent | Usage |
|---|---|---|
| H1 / display text | Bold + double-line box border (`╔═╗`) + category accent color | Hero card title (e.g., `MODEL_TRAINING_CHECKPOINT`) |
| H2 / section header | Bold, `text.primary`, single-line border (`┌─┐`) | Panel titles: "ACTIVE", "LOG FEED" |
| Body text | Regular weight, `text.primary` | Log feed row content |
| Caption / metadata | Dim modifier, `text.dim` | Timestamps, byte sizes, PID labels |
| Emphasis (inline) | Bold, no color change | Numeric deltas (`+42 / -11`) within a log row |
| Disabled/idle | Dim modifier + `SystemIdle` gray | Idle card, inactive footer stats |

### 2.3 Box-Drawing Weight as Hierarchy
Since there's no font-size control in a terminal grid, visual hierarchy is carried by **border weight and color**, not size:
- Double-line borders (`╔ ═ ╗ ║ ╚ ╝`) + bright accent color → the one actively-animating hero card (highest attention).
- Single-line borders (`┌ ─ ┐ │ └ ┘`) + `border.default` → static panels (header, log feed, footer).
- No border, just indentation → individual log feed rows within the log panel.

---

## 3. Layout Reference

(See `Architecture.md` §1 and `overview.md` §5.3 for the authoritative layout spec — restated here only for the design/visual read.)

- **Top bar**: system status, active AI agent name, clock — single-line border, `text.dim` for the clock, category-colored dot/icon if an agent is actively attributed.
- **Main left**: the current hero card — double-line border, category accent color, animated only while `timerfd` is armed.
- **Main right**: scrolling log feed table — `bg.panel`, rows colored by category, newest at top or bottom (pick one and keep it consistent; recommend newest-at-top so the user's eye doesn't have to track to the bottom during a burst).
- **Bottom footer**: self telemetry (own CPU%, RSS, tracked PID count) — always `text.dim`, never uses a category color, so it's visually distinct from agent-activity rows at a glance.

---

## 4. Motion / Animation Guidelines

- Hero card animation (frame advance via `timerfd`) should be subtle — a pulsing border brightness or a progress-bar fill, not a distracting spinner. The performance budget (`overview.md` §8) caps redraws at ~60/sec during bursts; animation should look smooth at that cap, not rely on exceeding it.
- No animation plays while idle — this is as much a design rule as a performance rule: a fully idle `SystemIdle` card must be visually and literally static, reinforcing the "nothing is happening" message.
