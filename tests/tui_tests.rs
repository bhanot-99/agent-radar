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
