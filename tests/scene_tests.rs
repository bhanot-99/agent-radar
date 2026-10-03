use std::time::{Instant, SystemTime};
use agent_radar::events::{ActivityCategory, FileOp, TelemetryEvent};
use agent_radar::tui::scenes::{category_skin, ProgressKind};
use agent_radar::tui::{render_hud, TuiState};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

/// One instance of every active `ActivityCategory` variant (26, excluding
/// `SystemIdle` which never drives an `Active` hero card) -- used to
/// exercise every one of the 27 bespoke scenes without repeating the same
/// construction 26 times per test.
fn all_active_categories() -> Vec<ActivityCategory> {
    vec![
        ActivityCategory::RustEdit { path: "src/main.rs".to_string(), lines_added: 12, lines_removed: 3 },
        ActivityCategory::PythonEdit { path: "app.py".to_string(), lines_added: 4, lines_removed: 1 },
        ActivityCategory::WebEdit { path: "app.tsx".to_string(), lines_added: 7, lines_removed: 2 },
        ActivityCategory::StyleEdit { path: "theme.css".to_string(), lines_added: 3, lines_removed: 0 },
        ActivityCategory::MarkupEdit { path: "index.html".to_string(), lines_added: 2, lines_removed: 1 },
        ActivityCategory::ConfigEdit { path: "config.toml".to_string(), lines_added: 1, lines_removed: 0 },
        ActivityCategory::DocsEdit { path: "README.md".to_string(), lines_added: 10, lines_removed: 2 },
        ActivityCategory::ShellScriptEdit { path: "deploy.sh".to_string(), lines_added: 5, lines_removed: 1 },
        ActivityCategory::ImageAsset { path: "a.png".to_string() },
        ActivityCategory::AudioAsset { path: "a.wav".to_string() },
        ActivityCategory::VideoAsset { path: "a.mp4".to_string() },
        ActivityCategory::FontAsset { path: "a.otf".to_string() },
        ActivityCategory::NotebookActivity { path: "exp.ipynb".to_string() },
        ActivityCategory::ModelTrainingCheckpoint { path: "m.safetensors".to_string(), size_bytes: 450_000_000 },
        ActivityCategory::ModelConfigEdit { path: "models/cfg.json".to_string() },
        ActivityCategory::IncomingDataStream { path: "d.parquet".to_string(), bytes_per_sec: 500_000, progress_pct: Some(42) },
        ActivityCategory::ArchiveWrite { path: "a.tar.gz".to_string() },
        ActivityCategory::GitOperation { path: ".git/HEAD".to_string() },
        ActivityCategory::DependencyLockUpdate { path: "Cargo.lock".to_string() },
        ActivityCategory::TestFileActivity { path: "tests/test_a.rs".to_string() },
        ActivityCategory::EnvSecretChange { path: ".env".to_string() },
        ActivityCategory::CiPipelineEdit { path: ".github/workflows/ci.yml".to_string() },
        ActivityCategory::ContainerConfigEdit { path: "docker-compose.yml".to_string() },
        ActivityCategory::WorkspaceExpansion { path: "src/new_module/".to_string() },
        ActivityCategory::MassDeletion { count: 142, sample_paths: vec!["target/".to_string(), "dist/".to_string()] },
        ActivityCategory::FileMutation { path: "cache/manifest.json".to_string(), op: FileOp::Modified },
    ]
}

#[test]
fn test_category_skin_has_caption_and_color_for_every_category() {
    for cat in all_active_categories() {
        let skin = category_skin(&cat);
        assert!(!skin.caption.is_empty(), "empty caption for {:?}", cat);
    }
    // SystemIdle is excluded from all_active_categories (it never drives an
    // Active hero card) but category_skin must still handle it.
    let skin = category_skin(&ActivityCategory::SystemIdle);
    assert!(!skin.caption.is_empty());
}

#[test]
fn test_scene_stats_never_fabricates_progress() {
    use agent_radar::tui::scenes::build_scene_stats;

    let indeterminate_cases = [
        ActivityCategory::ArchiveWrite { path: "a.zip".to_string() },
        ActivityCategory::ModelTrainingCheckpoint { path: "m.pt".to_string(), size_bytes: 500 },
        ActivityCategory::MassDeletion { count: 5, sample_paths: vec![] },
        ActivityCategory::IncomingDataStream {
            path: "d.parquet".to_string(),
            bytes_per_sec: 1000,
            progress_pct: None,
        },
    ];
    for cat in indeterminate_cases {
        let stats = build_scene_stats(&cat, Instant::now());
        assert_eq!(
            stats.progress,
            ProgressKind::Indeterminate,
            "category without a real completion percentage must never report Determinate: {:?}",
            cat
        );
    }

    let determinate = ActivityCategory::IncomingDataStream {
        path: "d.parquet".to_string(),
        bytes_per_sec: 1000,
        progress_pct: Some(42),
    };
    let stats = build_scene_stats(&determinate, Instant::now());
    assert_eq!(stats.progress, ProgressKind::Determinate { pct: 42 });
}

#[test]
fn test_file_mutation_stats_carry_the_real_file_op() {
    use agent_radar::tui::scenes::build_scene_stats;

    for (op, expected) in [
        (FileOp::Created, "CREATED"),
        (FileOp::Modified, "MODIFIED"),
        (FileOp::Deleted, "DELETED"),
        (FileOp::Renamed, "RENAMED"),
    ] {
        let cat = ActivityCategory::FileMutation { path: "x".to_string(), op };
        let stats = build_scene_stats(&cat, Instant::now());
        let op_row = stats.rows.iter().find(|(l, _)| *l == "Op");
        assert_eq!(op_row.map(|(_, v)| v.as_str()), Some(expected));
    }
}

#[test]
fn test_every_category_renders_without_panic_full_and_compact() {
    let push_and_render = |cat: ActivityCategory, width: u16, height: u16| {
        let mut state = TuiState::new("/tmp/test", false);
        state.push_event(TelemetryEvent {
            timestamp: SystemTime::now(),
            pid: 1,
            ppid: 1,
            process_name: "proc".to_string(),
            category: cat.clone(),
        });
        for _ in 0..5 {
            state.tick_animation();
            let backend = TestBackend::new(width, height);
            let mut term = Terminal::new(backend).unwrap();
            term.draw(|f| render_hud(f, &state))
                .unwrap_or_else(|_| panic!("render panicked for {:?} at {}x{}", cat, width, height));
        }
    };

    for cat in all_active_categories() {
        // Above MIN_HERO_SIZE (46, 12): exercises the bespoke full scene.
        push_and_render(cat.clone(), 90, 30);
        // Below it: exercises the shared compact fallback.
        push_and_render(cat, 30, 8);
    }
}

fn active_state(cat: &ActivityCategory) -> TuiState {
    let mut state = TuiState::new("/tmp/test", false);
    state.push_event(TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 1,
        ppid: 1,
        process_name: "proc".to_string(),
        category: cat.clone(),
    });
    state
}

/// Renders the hero widget into a sentinel-filled buffer larger than the
/// hero rect and asserts nothing outside the rect was touched. The hero is
/// borderless (full-bleed dot art), so this -- not a border check -- is the
/// structural containment guarantee `SceneCanvas` provides.
fn assert_hero_contained(state: &TuiState, hero: Rect, label: &str) {
    use agent_radar::tui::widgets::hero_stream::HeroStreamWidget;
    use ratatui::buffer::Buffer;
    use ratatui::widgets::Widget;

    let outer = Rect::new(0, 0, hero.right() + 6, hero.bottom() + 4);
    let mut buf = Buffer::empty(outer);
    for y in 0..outer.height {
        for x in 0..outer.width {
            buf[(x, y)].set_char('Z');
        }
    }
    HeroStreamWidget { active_hero: state.active_hero.as_ref(), idle_frame: 7 }.render(hero, &mut buf);
    for y in 0..outer.height {
        for x in 0..outer.width {
            let inside = x >= hero.x && x < hero.right() && y >= hero.y && y < hero.bottom();
            if !inside {
                assert_eq!(buf[(x, y)].symbol(), "Z", "{} wrote outside the hero rect at ({}, {}) for {:?}", label, x, y, hero);
            }
        }
    }
}

#[test]
fn test_every_scene_stays_inside_the_hero_rect_at_every_size() {
    let sizes = [
        Rect::new(3, 2, 50, 14),  // just above MIN_HERO_SIZE: full scene
        Rect::new(5, 4, 140, 34), // large terminal
        Rect::new(2, 3, 18, 8),   // tiny: compact fallback
        Rect::new(1, 1, 47, 12),  // exactly at the boundary
    ];
    for cat in all_active_categories() {
        let mut state = active_state(&cat);
        // Several frames so glitch bursts, particles and sweeps all fire.
        for _ in 0..8 {
            for _ in 0..7 {
                state.tick_animation();
            }
            for hero in sizes {
                assert_hero_contained(&state, hero, &format!("{:?}", cat));
            }
        }
    }
    let idle = TuiState::new("/tmp/test", false);
    for hero in sizes {
        assert_hero_contained(&idle, hero, "SystemIdle");
    }
}

#[test]
fn test_every_full_scene_draws_hundreds_of_micro_dots() {
    use agent_radar::tui::scenes::{build_scene_stats, category_path_line, render_scene_field};

    let mut cats = all_active_categories();
    cats.push(ActivityCategory::SystemIdle);
    for cat in cats {
        let stats = build_scene_stats(&cat, Instant::now());
        let path = category_path_line(&cat);
        for frame in [0u64, 37, 113, 260] {
            let field = render_scene_field(&cat, &stats, &path, frame, 100, 24);
            let lit = field.lit_count();
            assert!(lit > 500, "{:?} frame {} only lit {} dots", cat, frame, lit);
        }
    }
}

#[test]
fn test_every_scene_is_in_constant_motion() {
    use ratatui::buffer::Buffer;
    use ratatui::widgets::Widget;
    use agent_radar::tui::widgets::hero_stream::HeroStreamWidget;

    let hero = Rect::new(0, 0, 100, 24);
    let render = |state: &TuiState| {
        let mut buf = Buffer::empty(hero);
        HeroStreamWidget { active_hero: state.active_hero.as_ref(), idle_frame: state.idle_frame }.render(hero, &mut buf);
        buf
    };
    let mut cats = all_active_categories();
    cats.push(ActivityCategory::SystemIdle);
    for cat in cats {
        let mut state = if cat == ActivityCategory::SystemIdle { TuiState::new("/tmp/test", false) } else { active_state(&cat) };
        for _ in 0..20 {
            state.tick_animation();
        }
        // Every consecutive-frame pair must differ in many cells: the art
        // moves continuously, like video, not in occasional discrete steps.
        for _ in 0..5 {
            let a = render(&state);
            state.tick_animation();
            let b = render(&state);
            let changed = (0..hero.height)
                .flat_map(|y| (0..hero.width).map(move |x| (x, y)))
                .filter(|&(x, y)| a[(x, y)] != b[(x, y)])
                .count();
            assert!(changed > 40, "{:?} barely moved between frames ({} cells changed)", cat, changed);
        }
    }
}
