//! Unit tests for supervisor process types — state transitions,
//! health reporting, process table construction.

use reach_supervisor::processes::*;
use std::time::Duration;

// ═══════════════════════════════════════════════════════════
// ProcessState
// ═══════════════════════════════════════════════════════════

#[test]
fn stopped_state_is_not_running() {
    let state = ProcessState::Stopped;
    assert!(!state.is_running());
    assert!(state.pid().is_none());
    assert!(state.uptime().is_none());
    assert_eq!(state.restart_count(), 0);
}

#[test]
fn failed_state_preserves_restart_count() {
    let state = ProcessState::Failed {
        exit_code: Some(1),
        restart_count: 3,
        last_error: "segfault".into(),
    };
    assert!(!state.is_running());
    assert_eq!(state.restart_count(), 3);
}

// ═══════════════════════════════════════════════════════════
// ProcessHealth serialization
// ═══════════════════════════════════════════════════════════

#[test]
fn process_health_serializes_status_lowercase() {
    let health = ProcessHealth {
        name: "xvfb".into(),
        status: ProcessStatus::Running,
        pid: Some(42),
        uptime_secs: Some(120.5),
        restart_count: 0,
        exit_code: None,
        last_error: None,
    };
    let json = serde_json::to_value(&health).unwrap();
    assert_eq!(json["status"], "running");
    assert_eq!(json["pid"], 42);
    // exit_code and last_error should be absent (skip_serializing_if)
    assert!(json.get("exit_code").is_none());
    assert!(json.get("last_error").is_none());
}

#[test]
fn process_health_failed_status() {
    let health = ProcessHealth {
        name: "x11vnc".into(),
        status: ProcessStatus::Failed,
        pid: None,
        uptime_secs: None,
        restart_count: 5,
        exit_code: Some(139),
        last_error: Some("segfault".into()),
    };
    let json = serde_json::to_value(&health).unwrap();
    assert_eq!(json["status"], "failed");
    assert!(json["pid"].is_null());
    assert_eq!(json["restart_count"], 5);
    assert_eq!(json["exit_code"], 139);
    assert_eq!(json["last_error"], "segfault");
}

// ═══════════════════════════════════════════════════════════
// Supervisor construction
// ═══════════════════════════════════════════════════════════

#[test]
fn supervisor_new_starts_empty() {
    let sup = Supervisor::new();
    assert!(sup.all_healthy()); // vacuously true — no processes
    assert!(sup.health().is_empty());
}

// ═══════════════════════════════════════════════════════════
// X11 lock cleanup
// ═══════════════════════════════════════════════════════════

#[test]
fn clean_x11_locks_doesnt_fail_on_missing_files() {
    // Should succeed even when no lock files exist
    clean_x11_locks().unwrap();
}

// ═══════════════════════════════════════════════════════════
// RestartPolicy
// ═══════════════════════════════════════════════════════════

#[test]
fn restart_policy_always_has_backoff() {
    let policy = RestartPolicy::Always {
        max_restarts: 5,
        backoff: Duration::from_secs(2),
    };
    let RestartPolicy::Always {
        max_restarts,
        backoff,
    } = policy;
    assert_eq!(max_restarts, 5);
    assert_eq!(backoff, Duration::from_secs(2));
}

// ═══════════════════════════════════════════════════════════
// ReadyCheck
// ═══════════════════════════════════════════════════════════

#[test]
fn ready_check_variants_are_constructible() {
    let _file = ReadyCheck::FileExists("/tmp/.X99-lock".into());
    let _tcp = ReadyCheck::TcpPort(5900);
    let _imm = ReadyCheck::Immediate;
}
