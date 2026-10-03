//! Live gallery of every hero scene, rendered through the real HUD.
//!
//!   cargo run --release --example scene_gallery            # live, ←/→ to switch, q to quit
//!   cargo run --release --example scene_gallery -- 3       # start at scene 3
//!   cargo run --release --example scene_gallery -- dump 3 40 120 30
//!                                                          # print scene 3, frame 40, at 120x30 as text

use std::io;
use std::time::{Duration, Instant, SystemTime};

use agent_radar::events::{ActivityCategory, FileOp, TelemetryEvent};
use agent_radar::tui::scenes::ANIM_FRAME_MS;
use agent_radar::tui::{render_hud, HeroCardType, TuiState};
use crossterm::event::{self, Event, KeyCode};
use ratatui::backend::{CrosstermBackend, TestBackend};
use ratatui::Terminal;

fn categories() -> Vec<ActivityCategory> {
    use ActivityCategory::*;
    vec![
        SystemIdle,
        RustEdit { path: "src/watcher/inotify_tier.rs".into(), lines_added: 42, lines_removed: 7 },
        PythonEdit { path: "train/loop.py".into(), lines_added: 12, lines_removed: 3 },
        WebEdit { path: "web/app.tsx".into(), lines_added: 9, lines_removed: 2 },
        StyleEdit { path: "web/theme.css".into(), lines_added: 5, lines_removed: 0 },
        MarkupEdit { path: "web/index.html".into(), lines_added: 3, lines_removed: 1 },
        ConfigEdit { path: "config.toml".into(), lines_added: 2, lines_removed: 1 },
        DocsEdit { path: "README.md".into(), lines_added: 30, lines_removed: 4 },
        ShellScriptEdit { path: "scripts/deploy.sh".into(), lines_added: 8, lines_removed: 2 },
        ImageAsset { path: "assets/hero.png".into() },
        AudioAsset { path: "assets/theme.wav".into() },
        VideoAsset { path: "assets/intro.mp4".into() },
        FontAsset { path: "assets/Display.otf".into() },
        NotebookActivity { path: "experiments/ablation.ipynb".into() },
        ModelTrainingCheckpoint { path: "ckpt/model.safetensors".into(), size_bytes: 452_984_832 },
        ModelConfigEdit { path: "models/config.json".into() },
        IncomingDataStream { path: "data/train.parquet".into(), bytes_per_sec: 3_400_000, progress_pct: Some(63) },
        ArchiveWrite { path: "dist/release.tar.gz".into() },
        GitOperation { path: ".git/HEAD".into() },
        DependencyLockUpdate { path: "Cargo.lock".into() },
        TestFileActivity { path: "tests/auth_test.rs".into() },
        EnvSecretChange { path: ".env".into() },
        CiPipelineEdit { path: ".github/workflows/ci.yml".into() },
        ContainerConfigEdit { path: "docker-compose.yml".into() },
        WorkspaceExpansion { path: "src/new_module/".into() },
        MassDeletion { count: 142, sample_paths: vec!["target/".into(), "dist/".into()] },
        FileMutation { path: "cache/manifest.json".into(), op: FileOp::Modified },
        FileMutation { path: "cache/new_index.bin".into(), op: FileOp::Created },
        FileMutation { path: "cache/stale.lock".into(), op: FileOp::Deleted },
        FileMutation { path: "notes/renamed_plan.md".into(), op: FileOp::Renamed },
    ]
}

fn state_for(cat: &ActivityCategory, frame: u64) -> TuiState {
    let mut state = TuiState::new(".", false);
    state.idle_frame = frame;
    if *cat != ActivityCategory::SystemIdle {
        state.push_event(TelemetryEvent {
            timestamp: SystemTime::now(),
            pid: 4242,
            ppid: 1,
            process_name: "claude".into(),
            category: cat.clone(),
        });
        if let Some(HeroCardType::Active { frame: f, .. }) = state.active_hero.as_mut() {
            *f = frame;
        }
    }
    state
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cats = categories();

    if args.first().map(|s| s.as_str()) == Some("dump") {
        let idx: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1) % cats.len();
        let frame: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
        let w: u16 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(120);
        let h: u16 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(32);
        let state = state_for(&cats[idx], frame);
        let mut term = Terminal::new(TestBackend::new(w, h))?;
        term.draw(|f| render_hud(f, &state))?;
        let buf = term.backend().buffer();
        for y in 0..h {
            let line: String = (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect();
            println!("{}", line);
        }
        return Ok(());
    }

    if args.first().map(|s| s.as_str()) == Some("bench") {
        // Average full-HUD render cost per frame for every scene.
        let w: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(200);
        let h: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60);
        let mut term = Terminal::new(TestBackend::new(w, h))?;
        for (i, cat) in cats.iter().enumerate() {
            let t0 = Instant::now();
            let n = 60;
            for frame in 0..n {
                let state = state_for(cat, frame * 3);
                term.draw(|f| render_hud(f, &state))?;
            }
            let label = agent_radar::tui::theme::category_visual(cat).label;
            println!("{:>2} {:<28} {:>6.2} ms/frame", i, label, t0.elapsed().as_secs_f64() * 1000.0 / n as f64);
        }
        return Ok(());
    }

    if args.first().map(|s| s.as_str()) == Some("ppm") {
        // Rasterizes the rendered terminal buffer (braille dots + colors) to
        // a binary PPM on stdout, 8x16 px per cell, for visual review.
        let idx: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1) % cats.len();
        let frame: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
        let w: u16 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(120);
        let h: u16 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(32);
        let state = state_for(&cats[idx], frame);
        let mut term = Terminal::new(TestBackend::new(w, h))?;
        term.draw(|f| render_hud(f, &state))?;
        let buf = term.backend().buffer();
        let (pw, ph) = (w as usize * 8, h as usize * 16);
        let mut img = vec![[10u8, 14, 20]; pw * ph];
        for cy in 0..h {
            for cx in 0..w {
                let cell = &buf[(cx, cy)];
                let rgb = match cell.fg {
                    ratatui::style::Color::Rgb(r, g, b) => [r, g, b],
                    _ => [200, 200, 200],
                };
                let ch = cell.symbol().chars().next().unwrap_or(' ');
                let code = ch as u32;
                let mut rects: Vec<(usize, usize, usize, usize)> = Vec::new();
                if (0x2800..=0x28FF).contains(&code) {
                    let bits = code - 0x2800;
                    let map = [(0, 0, 0x01), (0, 1, 0x02), (0, 2, 0x04), (1, 0, 0x08), (1, 1, 0x10), (1, 2, 0x20), (0, 3, 0x40), (1, 3, 0x80)];
                    for (dx, dy, b) in map {
                        if bits & b != 0 {
                            rects.push((dx * 4 + 1, dy * 4 + 1, 2, 2));
                        }
                    }
                } else if ch != ' ' {
                    rects.push((1, 4, 6, 8));
                }
                for (rx, ry, rw, rh) in rects {
                    for yy in 0..rh {
                        for xx in 0..rw {
                            let px = cx as usize * 8 + rx + xx;
                            let py = cy as usize * 16 + ry + yy;
                            img[py * pw + px] = rgb;
                        }
                    }
                }
            }
        }
        use std::io::Write;
        let mut out = io::stdout().lock();
        write!(out, "P6\n{} {}\n255\n", pw, ph)?;
        for p in img {
            out.write_all(&p)?;
        }
        return Ok(());
    }

    let mut idx: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(1) % cats.len();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen, crossterm::cursor::Hide)?;
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let start = Instant::now();
    let mut frame_base = 0u64;
    let result = (|| -> io::Result<()> {
        loop {
            let frame = (start.elapsed().as_millis() as u64 / ANIM_FRAME_MS).saturating_sub(frame_base);
            let state = state_for(&cats[idx], frame);
            term.draw(|f| render_hud(f, &state))?;
            if event::poll(Duration::from_millis(ANIM_FRAME_MS))? {
                if let Event::Key(k) = event::read()? {
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => idx = (idx + 1) % cats.len(),
                        KeyCode::Left | KeyCode::Char('h') => idx = (idx + cats.len() - 1) % cats.len(),
                        _ => continue,
                    }
                    frame_base = start.elapsed().as_millis() as u64 / ANIM_FRAME_MS;
                }
            }
        }
    })();
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen, crossterm::cursor::Show)?;
    crossterm::terminal::disable_raw_mode()?;
    result
}
