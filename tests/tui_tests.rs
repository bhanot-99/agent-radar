use std::time::{Duration, Instant, SystemTime};
use ratatui::layout::Rect;
use agent_radar::events::{ActivityCategory, TelemetryEvent};
use agent_radar::tui::{compute_regions, HeroCardType, TuiState};

#[test]
fn test_hero_card_animation_lifecycle() {
    let mut state = TuiState::new("/tmp/test", false);
    assert!(!state.is_animating(), "Should not be animating initially");

    // Push ModelTrainingCheckpoint
    let event = TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 4321,
        ppid: 1,
        process_name: "claude".to_string(),
        category: ActivityCategory::ModelTrainingCheckpoint {
            path: "/tmp/test/model.safetensors".to_string(),
            size_bytes: 100 * 1024 * 1024,
        },
    };
    state.push_event(event);

    assert!(state.is_animating(), "Hero card should be animating after checkpoint event");
    assert!(matches!(state.active_hero, Some(HeroCardType::Active { .. })));

    // Tick animation
    let changed = state.tick_animation();
    assert!(changed, "tick_animation should return true while animating");

    if let Some(HeroCardType::Active { frame, .. }) = state.active_hero {
        assert_eq!(frame, 1);
    } else {
        panic!("Expected Active HeroCard");
    }

    // Simulate elapsed time (> 10s)
    if let Some(HeroCardType::Active { ref mut last_updated, .. }) = state.active_hero {
        *last_updated = Instant::now() - Duration::from_secs(15);
    }

    assert!(!state.is_animating(), "Should no longer be animating after 15s");

    // Next tick should transition back to IdleCard
    let changed_to_idle = state.tick_animation();
    assert!(changed_to_idle);
    assert!(matches!(state.active_hero, Some(HeroCardType::IdleCard)));
    assert!(!state.is_animating(), "IdleCard is not animating");
}

#[test]
fn test_datastream_hero_card() {
    let mut state = TuiState::new("/tmp/test", false);

    let event = TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 5432,
        ppid: 1,
        process_name: "curl".to_string(),
        category: ActivityCategory::IncomingDataStream {
            path: "/tmp/test/data.parquet".to_string(),
            bytes_per_sec: 500_000,
            progress_pct: None,
        },
    };
    state.push_event(event);

    assert!(state.is_animating());
    assert!(matches!(state.active_hero, Some(HeroCardType::Active { .. })));
}

#[test]
fn test_hero_card_persists_started_at_across_updates() {
    let mut state = TuiState::new("/tmp/test", false);

    let event1 = TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 4321,
        ppid: 1,
        process_name: "claude".to_string(),
        category: ActivityCategory::ModelTrainingCheckpoint {
            path: "/tmp/test/model.safetensors".to_string(),
            size_bytes: 10 * 1024 * 1024,
        },
    };
    state.push_event(event1);

    let started_at_first = match state.active_hero {
        Some(HeroCardType::Active { started_at, .. }) => started_at,
        _ => panic!("Expected Active HeroCard"),
    };

    std::thread::sleep(Duration::from_millis(50));

    // Push update for the same ongoing checkpoint
    let event2 = TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 4321,
        ppid: 1,
        process_name: "claude".to_string(),
        category: ActivityCategory::ModelTrainingCheckpoint {
            path: "/tmp/test/model.safetensors".to_string(),
            size_bytes: 50 * 1024 * 1024,
        },
    };
    state.push_event(event2);

    let started_at_second = match state.active_hero {
        Some(HeroCardType::Active { started_at, .. }) => started_at,
        _ => panic!("Expected Active HeroCard"),
    };

    // started_at must persist and NOT reset to now! (Finding 10)
    assert_eq!(started_at_first, started_at_second, "started_at must accumulate and not reset on update");
}

#[test]
fn test_system_idle_event_generation() {
    let mut state = TuiState::new("/tmp/test", false);

    // Initial state must contain SystemIdle (Finding 5)
    assert_eq!(state.events.len(), 1);
    assert_eq!(state.events[0].category, ActivityCategory::SystemIdle);

    // Push an event then simulate timeout
    state.push_event(TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 123,
        ppid: 1,
        process_name: "claude".to_string(),
        category: ActivityCategory::ModelTrainingCheckpoint {
            path: "ckpt.pt".to_string(),
            size_bytes: 100,
        },
    });

    if let Some(HeroCardType::Active { ref mut last_updated, .. }) = state.active_hero {
        *last_updated = Instant::now() - Duration::from_secs(15);
    }

    state.tick_animation();

    // Transition back to idle generates SystemIdle event
    assert_eq!(state.events[0].category, ActivityCategory::SystemIdle);
    assert_eq!(state.events[0].process_name, "system");
}

#[test]
fn test_sys_stats_cpu_and_tracked_pids() {
    let mut state = TuiState::new("/tmp/test", false);
    state.update_sys_stats(42);

    // Tracked PIDs must be accurately recorded (Finding 4)
    assert_eq!(state.system_metrics.tracked_pids, 42);
    // CPU % must be a valid percentage
    assert!(state.system_metrics.cpu_pct >= 0.0 && state.system_metrics.cpu_pct <= 100.0);
    // RSS must be positive
    assert!(state.system_metrics.rss_bytes > 0);
}

#[test]
fn test_unicode_safe_hud_rendering() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use agent_radar::tui::render_hud;

    let mut state = TuiState::new("/tmp/日本語/プロジェクト/🚀", false);

    // Add events with complex Unicode (CJK, emojis, accents)
    state.push_event(TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 1001,
        ppid: 1,
        process_name: "claude-code (日本語)".to_string(),
        category: ActivityCategory::RustEdit {
            path: "src/日本語/メイン_🔥_café.rs".to_string(),
            lines_added: 12,
            lines_removed: 4,
        },
    });

    state.push_event(TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 1002,
        ppid: 1,
        process_name: "aider".to_string(),
        category: ActivityCategory::ModelTrainingCheckpoint {
            path: "weights/モデル_チェックポイント.safetensors".to_string(),
            size_bytes: 500_000_000,
        },
    });

    // Drawing narrow/wide AND short/tall terminal frames must NEVER panic!
    // (Finding 2, extended to cover small heights alongside widths.)
    for width in [1, 10, 20, 40, 80, 120] {
        for height in [1, 3, 6, 8, 10, 24] {
            let backend = TestBackend::new(width, height);
            let mut term = Terminal::new(backend).unwrap();
            term.draw(|f| render_hud(f, &state))
                .unwrap_or_else(|_| panic!("Rendering Unicode HUD at {}x{} must never panic", width, height));
        }
    }
}

#[test]
fn test_log_panel_right_border_intact_at_narrow_width() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use agent_radar::tui::render_hud;

    // Agent name + PID is long enough that, pre-fix, the fixed-width Agent
    // column would overflow past the panel's own right border.
    let width = 24;
    let height = 20;
    let mut state = TuiState::new("/tmp/test", false);
    state.push_event(TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 123456,
        ppid: 1,
        process_name: "claude-code-long-name".to_string(),
        category: ActivityCategory::RustEdit {
            path: "src/main.rs".to_string(),
            lines_added: 3,
            lines_removed: 1,
        },
    });

    let regions = compute_regions(Rect::new(0, 0, width, height));
    let mut border_symbols: Vec<(u16, String)> = Vec::new();

    let backend = TestBackend::new(width, height);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| {
        render_hud(f, &state);
        let buf = f.buffer_mut();
        for row in (regions.log.y + 1)..(regions.log.bottom() - 1) {
            border_symbols.push((row, buf[(regions.log.right() - 1, row)].symbol().to_string()));
        }
    }).expect("render must not panic");

    let expected_vertical = "│";
    for (row, symbol) in border_symbols {
        assert_eq!(
            symbol, expected_vertical,
            "log panel's right border at row {} was overwritten by a fixed-width column overflow",
            row
        );
    }
}

#[test]
fn test_compute_regions_splits_hero_70_log_30_vertically() {
    let size = Rect::new(0, 0, 100, 53);
    let regions = compute_regions(size);

    // Header fixed at 3 rows, footer fixed at 1 row, both full width.
    assert_eq!(regions.header, Rect::new(0, 0, 100, 3));
    assert_eq!(regions.footer, Rect::new(0, 52, 100, 1));

    // Hero sits directly below the header; log sits directly below hero;
    // both are full terminal width (no side-by-side split anymore).
    assert_eq!(regions.hero.x, 0);
    assert_eq!(regions.hero.y, regions.header.bottom());
    assert_eq!(regions.hero.width, 100);
    assert_eq!(regions.log.x, 0);
    assert_eq!(regions.log.y, regions.hero.bottom());
    assert_eq!(regions.log.width, 100);
    assert_eq!(regions.log.bottom(), regions.footer.y);

    // Hero gets ~70% and log ~30% of the middle (header..footer) region.
    let middle_height = regions.footer.y - regions.header.bottom();
    assert_eq!(regions.hero.height + regions.log.height, middle_height);
    let hero_ratio = regions.hero.height as f64 / middle_height as f64;
    assert!(
        (hero_ratio - 0.70).abs() < 0.05,
        "hero should occupy ~70% of the middle region, got {:.2}",
        hero_ratio
    );
}

#[test]
fn test_hero_panel_never_writes_outside_its_own_rect() {
    use agent_radar::tui::widgets::hero_stream::HeroStreamWidget;
    use ratatui::buffer::Buffer;
    use ratatui::widgets::Widget;

    // The hero is borderless now, so the guarantee is checked directly: a
    // narrow panel with a caption wider than itself is rendered into a
    // buffer pre-filled with a sentinel, and every cell outside the hero
    // rect must still hold the sentinel afterwards.
    let mut state = TuiState::new("/tmp/test", false);
    state.push_event(TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 1,
        ppid: 1,
        process_name: "curl".to_string(),
        category: ActivityCategory::IncomingDataStream {
            path: "/tmp/data.parquet".to_string(),
            bytes_per_sec: 500_000,
            progress_pct: None,
        },
    });

    let outer = Rect::new(0, 0, 40, 20);
    let hero = Rect::new(8, 4, 22, 11);
    let mut buf = Buffer::empty(outer);
    for y in 0..outer.height {
        for x in 0..outer.width {
            buf[(x, y)].set_char('Z');
        }
    }
    HeroStreamWidget { active_hero: state.active_hero.as_ref(), idle_frame: 0 }.render(hero, &mut buf);
    for y in 0..outer.height {
        for x in 0..outer.width {
            let inside = x >= hero.x && x < hero.right() && y >= hero.y && y < hero.bottom();
            if !inside {
                assert_eq!(buf[(x, y)].symbol(), "Z", "hero wrote outside its rect at ({}, {})", x, y);
            }
        }
    }
}

#[test]
fn test_log_panel_header_intact_when_hero_caption_overflows() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use agent_radar::tui::render_hud;

    let width = 24;
    let height = 20;
    let mut state = TuiState::new("/tmp/test", false);
    state.push_event(TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 1,
        ppid: 1,
        process_name: "curl".to_string(),
        category: ActivityCategory::IncomingDataStream {
            path: "/tmp/data.parquet".to_string(),
            bytes_per_sec: 500_000,
            progress_pct: None,
        },
    });

    let regions = compute_regions(Rect::new(0, 0, width, height));
    let mut corner_symbol = String::new();

    let backend = TestBackend::new(width, height);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| {
        render_hud(f, &state);
        // Log panel's own top-left border corner must be untouched by
        // anything the hero panel drew above it.
        corner_symbol = f.buffer_mut()[(regions.log.x, regions.log.y)].symbol().to_string();
    }).expect("render must not panic");

    assert_eq!(corner_symbol, "┌", "log panel's own border corner was corrupted");
}

#[test]
fn test_scene_templates_render_without_panicking() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use agent_radar::tui::render_hud;

    // One category per SceneTemplateKind (see tests/scene_tests.rs for the
    // category -> template identity assertions this used to make via the
    // now-deleted AnimationPrimitive). This test's enduring contract is the
    // "tick 5x then render without panicking" behavior, kept from the old
    // test_all_five_animation_primitives.
    let sample_categories = [
        ActivityCategory::IncomingDataStream {
            path: "dataset.parquet".to_string(),
            bytes_per_sec: 1024 * 1024,
            progress_pct: Some(50),
        },
        ActivityCategory::RustEdit {
            path: "src/main.rs".to_string(),
            lines_added: 5,
            lines_removed: 1,
        },
        ActivityCategory::ImageAsset { path: "a.png".to_string() },
        ActivityCategory::ModelTrainingCheckpoint { path: "m.pt".to_string(), size_bytes: 1024 },
        ActivityCategory::GitOperation { path: ".git/HEAD".to_string() },
        ActivityCategory::TestFileActivity { path: "tests/test_auth.rs".to_string() },
        ActivityCategory::MassDeletion { count: 7, sample_paths: vec!["a".to_string()] },
        ActivityCategory::SystemIdle,
    ];

    for cat in sample_categories {
        let mut state = TuiState::new("/tmp/test", false);
        state.push_event(TelemetryEvent {
            timestamp: SystemTime::now(),
            pid: 999,
            ppid: 1,
            process_name: "claude".to_string(),
            category: cat,
        });

        // Render 5 animation frames to verify no panic
        for _ in 0..5 {
            state.tick_animation();
            let backend = TestBackend::new(80, 24);
            let mut term = Terminal::new(backend).unwrap();
            term.draw(|f| render_hud(f, &state)).expect("HUD render must succeed");
        }
    }
}
