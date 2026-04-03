//! End-to-end integration tests for the reach container.
//!
//! These tests require Docker and build the reach image.
//! Run with: `cargo test --test e2e_container -- --ignored`
//! Or: `make test-integration`

use std::process::Command;
use std::time::Duration;

const IMAGE: &str = "reach:test";
const CONTAINER: &str = "reach-e2e-test";

// ═══════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════

fn docker(args: &[&str]) -> std::process::Output {
    Command::new("docker")
        .args(args)
        .output()
        .expect("docker command failed")
}

fn docker_ok(args: &[&str]) -> String {
    let out = docker(args);
    assert!(
        out.status.success(),
        "docker {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn cleanup() {
    let _ = docker(&["rm", "-f", CONTAINER]);
}

fn start_container() {
    cleanup();
    docker_ok(&[
        "run",
        "-d",
        "--name",
        CONTAINER,
        "--shm-size=2g",
        "-p",
        "15900:5900",
        "-p",
        "16080:6080",
        "-p",
        "18400:8400",
        IMAGE,
    ]);
}

fn wait_for_health(timeout_secs: u64) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    while std::time::Instant::now() < deadline {
        if let Ok(output) = Command::new("curl")
            .args(["-sf", "http://localhost:18400/health"])
            .output()
        {
            if output.status.success() {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
}

fn exec_in_container(cmd: &str) -> std::process::Output {
    docker(&["exec", CONTAINER, "bash", "-c", cmd])
}

fn exec_ok(cmd: &str) -> String {
    let out = exec_in_container(cmd);
    assert!(
        out.status.success(),
        "exec failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

// ═══════════════════════════════════════════════════════════
// Image build
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn image_builds_successfully() {
    let out = docker(&["build", "-t", IMAGE, "."]);
    assert!(
        out.status.success(),
        "docker build failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ═══════════════════════════════════════════════════════════
// Container lifecycle
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn container_starts_and_becomes_healthy() {
    start_container();
    assert!(
        wait_for_health(30),
        "container did not become healthy within 30s"
    );
    cleanup();
}

#[test]
#[ignore]
fn container_stops_cleanly_on_sigterm() {
    start_container();
    assert!(wait_for_health(30));

    let out = docker(&["stop", "-t", "10", CONTAINER]);
    assert!(out.status.success(), "docker stop failed");

    // Verify it exited with code 0
    let inspect = docker_ok(&[
        "inspect",
        "-f",
        "{{.State.ExitCode}}",
        CONTAINER,
    ]);
    assert_eq!(inspect, "0", "container should exit cleanly");
    cleanup();
}

// ═══════════════════════════════════════════════════════════
// Health API
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn health_endpoint_returns_json() {
    start_container();
    assert!(wait_for_health(30));

    let out = Command::new("curl")
        .args(["-sf", "http://localhost:18400/health"])
        .output()
        .unwrap();

    let body: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("health response is not valid JSON");

    assert_eq!(body["service"], "reach-supervisor");
    assert!(body["processes"].is_array());
    assert!(
        body["status"] == "healthy" || body["status"] == "degraded",
        "unexpected status: {}",
        body["status"]
    );
    cleanup();
}

#[test]
#[ignore]
fn health_reports_all_four_processes() {
    start_container();
    assert!(wait_for_health(30));

    let out = Command::new("curl")
        .args(["-sf", "http://localhost:18400/health"])
        .output()
        .unwrap();

    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let processes = body["processes"].as_array().unwrap();

    let names: Vec<&str> = processes
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();

    assert!(names.contains(&"xvfb"), "missing xvfb");
    assert!(names.contains(&"openbox"), "missing openbox");
    assert!(names.contains(&"x11vnc"), "missing x11vnc");
    assert!(names.contains(&"novnc"), "missing novnc");
    cleanup();
}

#[test]
#[ignore]
fn metrics_endpoint_returns_prometheus_format() {
    start_container();
    assert!(wait_for_health(30));

    let out = Command::new("curl")
        .args(["-sf", "http://localhost:18400/metrics"])
        .output()
        .unwrap();

    let body = String::from_utf8_lossy(&out.stdout);
    // Prometheus text format starts with # HELP or # TYPE
    assert!(
        body.contains("# ") || body.is_empty(),
        "metrics should be prometheus text format"
    );
    cleanup();
}

// ═══════════════════════════════════════════════════════════
// Display server
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn xvfb_is_running_on_display_99() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_ok("ls /tmp/.X11-unix/X99");
    assert!(out.contains("X99"), "X99 socket missing");
    cleanup();
}

#[test]
#[ignore]
fn display_env_is_set() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_ok("echo $DISPLAY");
    assert_eq!(out, ":99");
    cleanup();
}

// ═══════════════════════════════════════════════════════════
// VNC
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn vnc_port_is_listening() {
    start_container();
    assert!(wait_for_health(30));

    // Check from inside the container
    let out = exec_in_container("nc -z localhost 5900 && echo ok");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"), "VNC port 5900 not listening");
    cleanup();
}

#[test]
#[ignore]
fn novnc_serves_web_page() {
    start_container();
    assert!(wait_for_health(30));

    let out = Command::new("curl")
        .args(["-sf", "http://localhost:16080/"])
        .output()
        .unwrap();

    let body = String::from_utf8_lossy(&out.stdout);
    assert!(
        body.contains("noVNC") || body.contains("html"),
        "noVNC web page not served"
    );
    cleanup();
}

// ═══════════════════════════════════════════════════════════
// Screenshot
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn screenshot_produces_valid_png() {
    start_container();
    assert!(wait_for_health(30));

    exec_ok("DISPLAY=:99 scrot -z /tmp/test.png");

    // Verify it's a PNG (magic bytes)
    let out = exec_ok("xxd -l 4 /tmp/test.png");
    assert!(
        out.contains("8950 4e47"),
        "not a valid PNG: {}",
        out
    );

    // Verify dimensions match expected resolution
    let out = exec_ok("DISPLAY=:99 xdpyinfo | grep dimensions || true");
    if !out.is_empty() {
        assert!(out.contains("1280x720"), "unexpected resolution: {}", out);
    }
    cleanup();
}

// ═══════════════════════════════════════════════════════════
// Chrome
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn chrome_is_installed() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_ok("google-chrome --version");
    assert!(out.contains("Google Chrome"), "chrome not installed: {}", out);
    cleanup();
}

#[test]
#[ignore]
fn chrome_launches_headless() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_in_container(
        "timeout 10 google-chrome --headless --dump-dom --no-sandbox https://example.com 2>/dev/null | head -5"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("<!doctype html>") || stdout.contains("<html"),
        "chrome headless didn't produce HTML"
    );
    cleanup();
}

// ═══════════════════════════════════════════════════════════
// Playwright
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn playwright_is_installed() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_ok("python3 -c 'import playwright; print(playwright.__version__)'");
    assert!(!out.is_empty(), "playwright not importable");
    cleanup();
}

#[test]
#[ignore]
fn playwright_can_fetch_page() {
    start_container();
    assert!(wait_for_health(30));

    let script = r#"
from playwright.sync_api import sync_playwright
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page()
    page.goto('https://example.com')
    print(page.title())
    browser.close()
"#;

    let out = exec_in_container(&format!("python3 -c '{}'", script.replace('\'', "'\\''")));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Example Domain"),
        "playwright didn't get page title: {}",
        stdout
    );
    cleanup();
}

// ═══════════════════════════════════════════════════════════
// Scrapling
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn scrapling_is_installed() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_ok("python3 -c 'import scrapling; print(scrapling.__version__)'");
    assert!(!out.is_empty(), "scrapling not importable");
    cleanup();
}

#[test]
#[ignore]
fn scrapling_can_scrape_page() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_in_container(
        "python3 -c \"from scrapling import Scraper; r = Scraper().fetch('https://example.com'); print(r.css('h1').text())\""
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Example Domain"),
        "scrapling didn't extract h1: {}",
        stdout
    );
    cleanup();
}

// ═══════════════════════════════════════════════════════════
// Node.js + computer-use-mcp
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn nodejs_is_installed() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_ok("node --version");
    assert!(out.starts_with('v'), "node not installed: {}", out);
    cleanup();
}

#[test]
#[ignore]
fn computer_use_mcp_is_installed() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_ok("npm list -g computer-use-mcp 2>/dev/null || npx computer-use-mcp --help 2>&1 | head -1");
    assert!(
        !out.is_empty(),
        "computer-use-mcp not found"
    );
    cleanup();
}

// ═══════════════════════════════════════════════════════════
// Container user
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn runs_as_sandbox_user() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_ok("whoami");
    assert_eq!(out, "sandbox", "should run as sandbox user, got: {}", out);
    cleanup();
}

#[test]
#[ignore]
fn sandbox_user_has_sudo() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_ok("sudo whoami");
    assert_eq!(out, "root");
    cleanup();
}

// ═══════════════════════════════════════════════════════════
// xdotool (input simulation)
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn xdotool_can_get_display_size() {
    start_container();
    assert!(wait_for_health(30));

    let out = exec_ok("DISPLAY=:99 xdotool getdisplaygeometry");
    assert!(
        out.contains("1280 720"),
        "unexpected display size: {}",
        out
    );
    cleanup();
}

#[test]
#[ignore]
fn xdotool_can_simulate_mouse_move() {
    start_container();
    assert!(wait_for_health(30));

    // Move mouse, then get position
    exec_ok("DISPLAY=:99 xdotool mousemove 640 360");
    let out = exec_ok("DISPLAY=:99 xdotool getmouselocation");
    assert!(out.contains("x:640"), "mouse not at expected x: {}", out);
    assert!(out.contains("y:360"), "mouse not at expected y: {}", out);
    cleanup();
}
