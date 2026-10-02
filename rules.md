# rules.md — Engineering Rules & AI Boundaries for Agent-Radar

> These rules exist to protect the four v1 design priorities stated in `overview.md` §1, in order: (1) zero privilege escalation, (2) ~0% idle CPU, (3) <10MB RSS, (4) single static binary, minimal deps. Any change that threatens one of these needs an explicit, called-out trade-off discussion before it's made — not a silent judgment call.

---

## 1. Use

- **Unprivileged syscalls only, in v1 code paths.** `inotify_init1`, `epoll_create1`, `flock`, `timerfd_create`, `/proc/<pid>/*` reads for own-user processes. Nothing else.
- **Single-threaded, synchronous `epoll` reactor.** All event sources (inotify fd, signal self-pipes, timerfd) are multiplexed through one `epoll_wait` call in `AppRunner::run_loop`.
- **Bounded, TTL-swept `LruCache`** for every cache (`pid_cache`: 512 entries/30s TTL; `rate_trackers`: 2048 entries/30s TTL). No cache without a capacity and an eviction policy.
- **Debounce before classify.** Every write-triggered classification (especially line-diffing for `SourceCodeMutation`) happens only after a per-path debounce window (~300–500ms) elapses with no further writes.
- **Rename-cookie pairing** for `IN_MOVED_FROM`/`IN_MOVED_TO` so atomic saves (write-temp + rename) are treated as one logical content-replace, never a spurious delete+create.
- **Ordered, path-context-aware rule tables** for classification (see `overview.md` §3.3) — when adding a new category or extension mapping, add an ordered predicate, not a flat extension→category map entry, so ambiguous extensions (like `.bin`) stay disambiguable by path context.
- **Event-driven rendering.** Redraw only on actual `TuiState` change; diff the Ratatui cell buffer and emit ANSI only for changed cells.
- **Panic-safe terminal teardown.** Any panic path must still restore the terminal (raw mode off, alternate screen off) — register a panic hook that does this before the default panic handler runs.
- **Static musl targets** (`x86_64`/`aarch64-unknown-linux-musl`) for all release builds.
- **Graceful degradation on resource limits** — e.g., if `fs.inotify.max_user_watches` would be exceeded, warn and keep watching whatever was already registered; never hard-fail the whole process over one oversized subtree.

## 2. Avoid

- **No async runtime** (no `tokio`, `async-std`, `smol`) anywhere in v1. The whole point of the synchronous `epoll` reactor is to avoid runtime overhead and keep the binary and RSS small.
- **No privilege escalation of any kind** — no `sudo` invocation, no `setcap` instructions baked into install flows, no code path that silently requires `CAP_BPF`, `CAP_SYS_ADMIN`, or `CAP_DAC_READ_SEARCH`. If a feature needs one of these, it belongs in v2 behind an explicit `--privileged`/`--features ebpf` opt-in, never the default path.
- **No eBPF or fanotify code in the default v1 build.** The `EventSource` trait exists so these can be added later (`overview.md` §4.1), but no `EbpfObserver`/`FanotifyObserver` type is compiled or linked today.
- **No unbounded collections.** No raw `HashMap`/`Vec` used as a cache without an eviction bound — this was flaw #2 in the corrections log (`overview.md` §9.1) and must not regress.
- **No per-raw-event (per `IN_MODIFY`) expensive work** — specifically, no line-diffing or heavy classification before the debounce window settles (flaw #7).
- **No fixed-Hz render timer.** Never re-introduce a perpetual 60Hz (or any-Hz) timer tick; the `timerfd` is armed only while a hero card is actively animating and disarmed otherwise (flaw #3).
- **No MCP/OpenTelemetry socket listener, no Kitty graphics protocol code** in v1 — both are explicitly deferred (`overview.md` §9.2). Don't scaffold `src/socket/` or Kitty-specific render paths "for later."
- **No duplicate data fields** between `TelemetryEvent` and `ActivityCategory`/`HeroCardType` — path/size/rate data lives exactly once, inside the `ActivityCategory` variant (flaw #12). Don't reintroduce flat `raw_path`/`size_bytes` on the envelope type.
- **No new C-library-linked dependencies** that would fight static musl linking (this is explicitly why eBPF/libbpf is kept out of v1 — `overview.md` §7).

## 3. Library & Dependency Boundaries

Allowed v1 dependency set (do not add to this list without flagging the RSS/binary-size/static-linking trade-off explicitly):

`inotify`, `rustix`, `ratatui`, `crossterm`, `lru`, `regex`, `signal-hook`.

- Any new dependency must be justified against the §8 SLA targets (RSS, FD count, static-link compatibility) before being added.
- Do not add a crate that pulls in an async runtime transitively — check `cargo tree` before adding anything that touches I/O.
- v2-only crates (`aya` for eBPF, any fanotify or socket/IPC crate) must land behind a Cargo feature flag (`--features ebpf`) that is **off by default**, never in the default dependency tree.

## 4. Error Handling

- **No silent failures that hide real state.** A watch-budget overrun is surfaced as a warning, not swallowed — the user should always know if part of their tree isn't being watched.
- **PID races are expected, not exceptional.** A `/proc/<pid>` read failing because the process already exited is a normal, transient condition — handle it by discarding that attribution attempt, not by propagating an error up the stack or crashing.
- **Liveness-check every cache hit** against `/proc/<pid>` before trusting a cached PID→agent mapping, to avoid misattributing a reused PID to a stale, unrelated process (this is the fix for flaw #2's liveness gap).
- **Terminal state must always be restored on exit** — normal shutdown (`SIGTERM`/`SIGINT`), and panic, both go through the same teardown path (close watches, restore terminal, release lockfile).
- **Single-instance violations fail fast and loud**: if `flock` is already held, print "already running" and exit 1 — do not queue, wait, or silently no-op.

## 5. AI / Assistant Boundaries

When an AI coding assistant (including this one) is implementing against this spec:

- **Do not silently expand scope into v2.** If a task seems to need fanotify, eBPF, or the socket listener to be "done properly," stop and say so explicitly rather than implementing it behind a flag nobody asked for.
- **Do not relax the privilege boundary to make a feature easier.** If something is only easy with `CAP_BPF` or root, that is a sign it belongs in v2, not a reason to add a privilege check-and-prompt to v1.
- **Preserve the corrections in `overview.md` §9.1.** Each of the 12 listed flaws was fixed intentionally; don't reintroduce any of them while refactoring (e.g., don't go back to a flat extension map, don't remove the LRU bounds, don't bring back a perpetual render timer).
- **Follow the ordered-rule-table pattern** for any new classification logic rather than inventing a second dispatch mechanism.
- **Ask before changing an SLA target in `overview.md` §8** — these numbers are enforcement targets, not suggestions; a design that can't hit them needs explicit renegotiation, not a quiet downgrade of the target.
- **Update `memory.md`** after completing or starting work on any phase from `phases.md` — this is a standing instruction, not a one-time request.
