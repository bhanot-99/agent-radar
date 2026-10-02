use std::time::{Duration, Instant, SystemTime};
use agent_radar::events::{ActivityCategory, TelemetryEvent};
use agent_radar::tui::{HeroCardType, TuiState};

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
    assert!(matches!(state.active_hero, Some(HeroCardType::ModelTrainingCard { .. })));

    // Tick animation
    let changed = state.tick_animation();
    assert!(changed, "tick_animation should return true while animating");

    if let Some(HeroCardType::ModelTrainingCard { frame, .. }) = state.active_hero {
        assert_eq!(frame, 1);
    } else {
        panic!("Expected ModelTrainingCard");
    }

    // Simulate elapsed time (> 10s)
    if let Some(HeroCardType::ModelTrainingCard { ref mut last_updated, .. }) = state.active_hero {
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
    assert!(matches!(state.active_hero, Some(HeroCardType::DataStreamCard { .. })));
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
        Some(HeroCardType::ModelTrainingCard { started_at, .. }) => started_at,
        _ => panic!("Expected ModelTrainingCard"),
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
        Some(HeroCardType::ModelTrainingCard { started_at, .. }) => started_at,
        _ => panic!("Expected ModelTrainingCard"),
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

    if let Some(HeroCardType::ModelTrainingCard { ref mut last_updated, .. }) = state.active_hero {
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

    let _backend = TestBackend::new(80, 24);

    let mut state = TuiState::new("/tmp/日本語/プロジェクト/🚀", false);

    // Add events with complex Unicode (CJK, emojis, accents)
    state.push_event(TelemetryEvent {
        timestamp: SystemTime::now(),
        pid: 1001,
        ppid: 1,
        process_name: "claude-code (日本語)".to_string(),
        category: ActivityCategory::SourceCodeMutation {
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

    // Drawing narrow and wide terminal frames must NEVER panic! (Finding 2)
    for width in [20, 40, 80, 120] {
        let backend = TestBackend::new(width, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render_hud(f, &state)).expect("Rendering Unicode HUD must never panic");
    }
}
