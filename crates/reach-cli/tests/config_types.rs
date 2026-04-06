//! Unit tests for configuration types — defaults, TOML parsing, paths.

use reach_cli::config::*;

// ═══════════════════════════════════════════════════════════
// Defaults
// ═══════════════════════════════════════════════════════════

#[test]
fn default_config_has_sane_values() {
    let config = ReachConfig::default();
    assert_eq!(config.sandbox.image, "reach:latest");
    assert_eq!(config.sandbox.resolution, "1280x720");
    assert_eq!(config.server.port, 4200);
    assert_eq!(config.server.host, "127.0.0.1");
}

#[test]
fn default_shm_is_2gb() {
    let config = ReachConfig::default();
    assert_eq!(config.sandbox.shm_size, 2 * 1024 * 1024 * 1024);
}

#[test]
fn default_ports_match_convention() {
    let config = ReachConfig::default();
    assert_eq!(config.sandbox.vnc_port, 5900);
    assert_eq!(config.sandbox.novnc_port, 6080);
    assert_eq!(config.sandbox.health_port, 8400);
}

// ═══════════════════════════════════════════════════════════
// TOML deserialization
// ═══════════════════════════════════════════════════════════

#[test]
fn empty_toml_produces_defaults() {
    let config: ReachConfig = toml::from_str("").unwrap();
    assert_eq!(config.sandbox.image, "reach:latest");
    assert_eq!(config.server.port, 4200);
}

#[test]
fn partial_toml_fills_missing_with_defaults() {
    let config: ReachConfig = toml::from_str(
        r#"
        [server]
        port = 9999
        "#,
    )
    .unwrap();
    assert_eq!(config.server.port, 9999);
    assert_eq!(config.server.host, "127.0.0.1"); // default
    assert_eq!(config.sandbox.image, "reach:latest"); // default
}

#[test]
fn full_toml_overrides_all() {
    let config: ReachConfig = toml::from_str(
        r#"
        [sandbox]
        image = "reach:dev"
        resolution = "1920x1080"
        shm_size = 4294967296
        vnc_port = 5901
        novnc_port = 6081
        health_port = 8401

        [server]
        port = 8080
        host = "0.0.0.0"

        [docker]
        socket = "/var/run/docker.sock"
        "#,
    )
    .unwrap();
    assert_eq!(config.sandbox.image, "reach:dev");
    assert_eq!(config.sandbox.resolution, "1920x1080");
    assert_eq!(config.sandbox.shm_size, 4294967296);
    assert_eq!(config.sandbox.vnc_port, 5901);
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.docker.socket, "/var/run/docker.sock");
}

#[test]
fn config_roundtrips_through_toml() {
    let config = ReachConfig::default();
    let toml_str = toml::to_string(&config).unwrap();
    let back: ReachConfig = toml::from_str(&toml_str).unwrap();
    assert_eq!(back.server.port, config.server.port);
    assert_eq!(back.sandbox.image, config.sandbox.image);
}

// ═══════════════════════════════════════════════════════════
// Config path
// ═══════════════════════════════════════════════════════════

#[test]
fn config_path_ends_with_reach_config_toml() {
    let path = ReachConfig::config_path();
    assert!(path.ends_with("reach/config.toml"));
}

// ═══════════════════════════════════════════════════════════
// Profile directory
// ═══════════════════════════════════════════════════════════

#[test]
fn profile_dir_defaults_to_none_in_serde() {
    let config = ReachConfig::default();
    assert!(config.sandbox.profile_dir.is_none());
}

#[test]
fn profile_dir_can_be_overridden_via_toml() {
    let config: ReachConfig = toml::from_str(
        r#"
        [sandbox]
        profile_dir = "/srv/reach/profiles"
        "#,
    )
    .unwrap();
    assert_eq!(
        config.sandbox.profile_dir,
        Some(std::path::PathBuf::from("/srv/reach/profiles"))
    );
    assert_eq!(
        config.sandbox.resolved_profile_dir(),
        std::path::PathBuf::from("/srv/reach/profiles")
    );
}

#[test]
fn resolved_profile_dir_falls_back_when_unset() {
    let config = ReachConfig::default();
    let resolved = config.sandbox.resolved_profile_dir();
    assert!(resolved.to_string_lossy().contains("reach"));
    assert!(resolved.ends_with("profiles"));
}

#[test]
fn default_profile_dir_helper_returns_reach_path() {
    let dir = default_profile_dir();
    assert!(dir.to_string_lossy().contains("reach"));
}
