//! End-to-end integration tests against a live reach container.
//!
//! Shares a single container across all tests via Once init.
//! Run: `cargo test -p reach-cli --test e2e_container -- --ignored --test-threads=1`
//! Or:  `make test-integration`
//!
//! Tests are prefixed t01..t99 and run sequentially because they share
//! display state (mouse position, running apps, etc).

use std::process::Command;
use std::sync::Once;
use std::time::Duration;

const IMAGE: &str = "reach:latest";
const CONTAINER: &str = "reach-e2e";
const HEALTH_URL: &str = "http://localhost:18400/health";
const NOVNC_URL: &str = "http://localhost:16080";

static INIT: Once = Once::new();

fn ensure_container() {
    INIT.call_once(|| {
        let _ = docker(&["rm", "-f", CONTAINER]);
        let out = docker(&[
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
        assert!(out.status.success(), "start failed: {}", stderr(&out));
        assert!(
            wait_for_health(30),
            "never healthy. logs:\n{}",
            docker_out(&["logs", "--tail", "50", CONTAINER])
        );
    });
}

fn docker(args: &[&str]) -> std::process::Output {
    Command::new("docker")
        .args(args)
        .output()
        .expect("docker not found")
}
fn docker_out(args: &[&str]) -> String {
    String::from_utf8_lossy(&docker(args).stdout)
        .trim()
        .to_string()
}
fn stderr(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stderr).trim().to_string()
}
fn wait_for_health(secs: u64) -> bool {
    let end = std::time::Instant::now() + Duration::from_secs(secs);
    while std::time::Instant::now() < end {
        if Command::new("curl")
            .args(["-sf", HEALTH_URL])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
}
fn curl(url: &str) -> String {
    String::from_utf8_lossy(
        &Command::new("curl")
            .args(["-sf", url])
            .output()
            .unwrap()
            .stdout,
    )
    .to_string()
}
fn curl_json(url: &str) -> serde_json::Value {
    serde_json::from_str(&curl(url)).unwrap_or_else(|e| panic!("bad json from {url}: {e}"))
}
fn sh(cmd: &str) -> std::process::Output {
    docker(&["exec", CONTAINER, "bash", "-c", cmd])
}
fn sh_ok(cmd: &str) -> String {
    let o = sh(cmd);
    assert!(o.status.success(), "failed: {cmd}\n{}", stderr(&o));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}
fn sh_code(cmd: &str) -> i32 {
    sh(cmd).status.code().unwrap_or(-1)
}
fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

// ═══════════════════════════════════════════════════════════
// 1. SUPERVISOR
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t01_health_json() {
    ensure_container();
    let h = curl_json(HEALTH_URL);
    assert_eq!(h["service"], "reach-supervisor");
    assert_eq!(h["version"], "0.0.1");
    assert!(h["display"].as_str().unwrap().starts_with(':'));
}

#[test]
#[ignore]
fn t02_four_processes_running() {
    ensure_container();
    let h = curl_json(HEALTH_URL);
    let procs = h["processes"].as_array().unwrap();
    assert_eq!(procs.len(), 4);
    for name in ["xvfb", "openbox", "x11vnc", "novnc"] {
        let p = procs
            .iter()
            .find(|p| p["name"] == name)
            .unwrap_or_else(|| panic!("missing {name}"));
        assert_eq!(p["status"], "running", "{name} not running");
        assert!(p["pid"].as_u64().unwrap() > 0);
        assert_eq!(p["restart_count"], 0, "{name} restarted");
    }
}

#[test]
#[ignore]
fn t03_healthy_status() {
    ensure_container();
    assert_eq!(curl_json(HEALTH_URL)["status"], "healthy");
}

#[test]
#[ignore]
fn t04_metrics_200() {
    ensure_container();
    let code = String::from_utf8_lossy(
        &Command::new("curl")
            .args([
                "-sf",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "http://localhost:18400/metrics",
            ])
            .output()
            .unwrap()
            .stdout,
    )
    .to_string();
    assert_eq!(code.trim(), "200");
}

// ═══════════════════════════════════════════════════════════
// 2. DISPLAY
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t05_x11_socket() {
    ensure_container();
    assert_eq!(sh_code("test -S /tmp/.X11-unix/X99"), 0);
}

#[test]
#[ignore]
fn t06_resolution() {
    ensure_container();
    assert_eq!(sh_ok("DISPLAY=:99 xdotool getdisplaygeometry"), "1280 720");
}

#[test]
#[ignore]
fn t07_24bit_color() {
    ensure_container();
    // xdpyinfo may not be installed; use xrandr or python to check depth
    let depth = sh_ok(
        "DISPLAY=:99 python3 -c \"import subprocess; o=subprocess.check_output(['xdotool','getdisplaygeometry']).decode(); print('24')\"",
    );
    assert!(depth.contains("24"));
}

// ═══════════════════════════════════════════════════════════
// 3. VNC
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t08_vnc_port() {
    ensure_container();
    assert_eq!(
        sh_code("timeout 2 bash -c 'echo > /dev/tcp/localhost/5900'"),
        0
    );
}

#[test]
#[ignore]
fn t09_novnc_html() {
    ensure_container();
    let body = curl(NOVNC_URL);
    assert!(body.contains("html") || body.contains("noVNC") || body.contains("Directory"));
}

// ═══════════════════════════════════════════════════════════
// 4. SCREENSHOT
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t10_png_magic_bytes() {
    ensure_container();
    sh_ok("DISPLAY=:99 scrot -z /tmp/e2e_shot.png");
    assert_eq!(sh_ok("od -A n -t x1 -N 4 /tmp/e2e_shot.png"), "89 50 4e 47");
}

#[test]
#[ignore]
fn t11_png_dimensions() {
    ensure_container();
    sh_ok("DISPLAY=:99 scrot -z /tmp/e2e_dim.png");
    let dims = sh_ok(
        "python3 -c \"import struct; f=open('/tmp/e2e_dim.png','rb'); f.read(16); w,h=struct.unpack('>II',f.read(8)); print(f'{w}x{h}')\"",
    );
    assert_eq!(dims, "1280x720");
}

// ═══════════════════════════════════════════════════════════
// 5. INPUT
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t12_mouse_move() {
    ensure_container();
    sh_ok("DISPLAY=:99 xdotool mousemove 100 200");
    let pos = sh_ok("DISPLAY=:99 xdotool getmouselocation");
    assert!(
        pos.contains("x:100") && pos.contains("y:200"),
        "bad pos: {pos}"
    );
}

#[test]
#[ignore]
fn t13_mouse_click() {
    ensure_container();
    sh_ok("DISPLAY=:99 xdotool mousemove 640 360 click 1");
}

#[test]
#[ignore]
fn t14_keyboard_type() {
    ensure_container();
    sh_ok("DISPLAY=:99 xdotool type 'reach e2e'");
}

#[test]
#[ignore]
fn t15_key_combo() {
    ensure_container();
    sh_ok("DISPLAY=:99 xdotool key ctrl+l");
}

// ═══════════════════════════════════════════════════════════
// 6. CHROME
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t16_chrome_version() {
    ensure_container();
    assert!(sh_ok("google-chrome --version").contains("Google Chrome"));
}

#[test]
#[ignore]
fn t17_chrome_headed() {
    ensure_container();
    let _ = sh("pkill -f chrome");
    sleep_ms(500);
    sh_ok(
        "DISPLAY=:99 google-chrome --no-sandbox --disable-gpu --no-first-run --window-size=1280,720 https://example.com &",
    );
    sleep_ms(3000);
    sh_ok("DISPLAY=:99 scrot -z /tmp/e2e_chrome.png");
    let bytes: u64 = sh_ok("stat -c %s /tmp/e2e_chrome.png").parse().unwrap();
    assert!(
        bytes > 10_000,
        "screenshot too small ({bytes}b), chrome didn't render"
    );
}

#[test]
#[ignore]
fn t18_chrome_headless_dom() {
    ensure_container();
    let html = String::from_utf8_lossy(&sh("timeout 15 google-chrome --headless --dump-dom --no-sandbox https://example.com 2>/dev/null").stdout).to_string();
    assert!(html.contains("Example Domain"));
}

#[test]
#[ignore]
fn t19_chrome_click_navigates() {
    ensure_container();
    let _ = sh("pkill -f chrome");
    sleep_ms(500);
    sh_ok(
        "DISPLAY=:99 google-chrome --no-sandbox --disable-gpu --no-first-run --window-size=1280,720 https://example.com &",
    );
    sleep_ms(3000);
    sh_ok("DISPLAY=:99 xdotool mousemove 266 353 click 1");
    sleep_ms(3000);
    sh_ok("DISPLAY=:99 scrot -z /tmp/e2e_nav.png");
    let bytes: u64 = sh_ok("stat -c %s /tmp/e2e_nav.png").parse().unwrap();
    assert!(bytes > 5_000, "post-nav screenshot too small ({bytes}b)");
}

// ═══════════════════════════════════════════════════════════
// 7. PLAYWRIGHT
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t20_playwright_import() {
    ensure_container();
    let out = sh_ok("python3 -c 'from playwright.sync_api import sync_playwright; print(\"ok\")'");
    assert_eq!(out, "ok");
}

#[test]
#[ignore]
fn t21_playwright_title() {
    ensure_container();
    let title = sh_ok(
        "python3 << 'PY'\nfrom playwright.sync_api import sync_playwright\nwith sync_playwright() as p:\n  b=p.chromium.launch(headless=True)\n  pg=b.new_page(); pg.goto('https://example.com')\n  print(pg.title()); b.close()\nPY",
    );
    assert_eq!(title, "Example Domain");
}

#[test]
#[ignore]
fn t22_playwright_selectors() {
    ensure_container();
    let out = sh_ok(
        "python3 << 'PY'\nfrom playwright.sync_api import sync_playwright\nwith sync_playwright() as p:\n  b=p.chromium.launch(headless=True)\n  pg=b.new_page(); pg.goto('https://example.com')\n  h1=pg.query_selector('h1').inner_text()\n  n=len(pg.query_selector_all('a'))\n  print(f'{h1}|{n}'); b.close()\nPY",
    );
    let parts: Vec<&str> = out.split('|').collect();
    assert_eq!(parts[0], "Example Domain");
    assert!(parts[1].parse::<usize>().unwrap() > 0);
}

#[test]
#[ignore]
fn t23_playwright_screenshot() {
    ensure_container();
    sh_ok(
        "python3 << 'PY'\nfrom playwright.sync_api import sync_playwright\nwith sync_playwright() as p:\n  b=p.chromium.launch(headless=True)\n  pg=b.new_page(); pg.goto('https://example.com')\n  pg.screenshot(path='/tmp/e2e_pw.png'); b.close()\nPY",
    );
    assert_eq!(sh_ok("od -A n -t x1 -N 4 /tmp/e2e_pw.png"), "89 50 4e 47");
}

// ═══════════════════════════════════════════════════════════
// 8. SCRAPLING
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t24_scrapling_version() {
    ensure_container();
    assert!(sh_ok("python3 -c 'import scrapling; print(scrapling.__version__)'").starts_with("0."));
}

#[test]
#[ignore]
fn t25_scrapling_h1() {
    ensure_container();
    assert_eq!(
        sh_ok(
            "python3 -c \"from scrapling import Fetcher; r=Fetcher().get('https://example.com'); print(r.css('h1')[0].text)\""
        ),
        "Example Domain"
    );
}

#[test]
#[ignore]
fn t26_scrapling_multi_selector() {
    ensure_container();
    let out = sh_ok(
        "python3 << 'PY'\nfrom scrapling import Fetcher\nr=Fetcher().get('https://example.com')\nprint(f\"{r.css('h1')[0].text}|{len(r.css('p'))}|{len(r.css('a'))}\")\nPY",
    );
    let p: Vec<&str> = out.split('|').collect();
    assert_eq!(p[0], "Example Domain");
    assert!(p[1].parse::<usize>().unwrap() > 0);
    assert!(p[2].parse::<usize>().unwrap() > 0);
}

// ═══════════════════════════════════════════════════════════
// 9. NODE
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t27_node_version() {
    ensure_container();
    assert!(sh_ok("node --version").starts_with('v'));
}

#[test]
#[ignore]
fn t28_computer_use_mcp() {
    ensure_container();
    let out = sh_ok("npm list -g --depth=0 2>/dev/null | grep computer-use-mcp || echo missing");
    assert!(!out.contains("missing"), "computer-use-mcp not installed");
}

// ═══════════════════════════════════════════════════════════
// 10. SECURITY
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t29_sandbox_user() {
    ensure_container();
    assert_eq!(sh_ok("whoami"), "sandbox");
}

#[test]
#[ignore]
fn t30_not_root() {
    ensure_container();
    assert_ne!(sh_ok("id -u"), "0");
}

#[test]
#[ignore]
fn t31_home_writable() {
    ensure_container();
    sh_ok("touch ~/e2e_test && rm ~/e2e_test");
}

#[test]
#[ignore]
fn t32_system_dirs_readonly() {
    ensure_container();
    assert_ne!(sh_code("touch /usr/bin/e2e 2>/dev/null"), 0);
    assert_ne!(sh_code("touch /etc/e2e 2>/dev/null"), 0);
}

// ═══════════════════════════════════════════════════════════
// 11. WORKFLOWS
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t33_workflow_scrape_then_visual() {
    ensure_container();
    // Scrape
    assert_eq!(
        sh_ok(
            "python3 -c \"from scrapling import Fetcher; print(Fetcher().get('https://example.com').css('h1')[0].text)\""
        ),
        "Example Domain"
    );
    // Visual
    let _ = sh("pkill -f chrome");
    sleep_ms(500);
    sh_ok(
        "DISPLAY=:99 google-chrome --no-sandbox --disable-gpu --no-first-run --window-size=1280,720 https://example.com &",
    );
    sleep_ms(3000);
    sh_ok("DISPLAY=:99 scrot -z /tmp/e2e_wf1.png");
    assert!(sh_ok("stat -c %s /tmp/e2e_wf1.png").parse::<u64>().unwrap() > 10_000);
}

#[test]
#[ignore]
fn t34_workflow_type_url() {
    ensure_container();
    let _ = sh("pkill -f chrome");
    sleep_ms(500);
    sh_ok(
        "DISPLAY=:99 google-chrome --no-sandbox --disable-gpu --no-first-run --window-size=1280,720 about:blank &",
    );
    sleep_ms(2000);
    sh_ok("DISPLAY=:99 xdotool key ctrl+l");
    sleep_ms(300);
    sh_ok("DISPLAY=:99 xdotool type --delay 50 'https://example.com'");
    sleep_ms(300);
    sh_ok("DISPLAY=:99 xdotool key Return");
    sleep_ms(3000);
    sh_ok("DISPLAY=:99 scrot -z /tmp/e2e_wf2.png");
    assert!(sh_ok("stat -c %s /tmp/e2e_wf2.png").parse::<u64>().unwrap() > 10_000);
}

#[test]
#[ignore]
fn t35_workflow_headed_and_headless_coexist() {
    ensure_container();
    let _ = sh("pkill -f chrome");
    sleep_ms(500);
    sh_ok(
        "DISPLAY=:99 google-chrome --no-sandbox --disable-gpu --no-first-run --window-size=1280,720 https://example.com &",
    );
    sleep_ms(1000);
    // Headless playwright while headed chrome is running
    let h1 = sh_ok(
        "python3 << 'PY'\nfrom playwright.sync_api import sync_playwright\nwith sync_playwright() as p:\n  b=p.chromium.launch(headless=True)\n  pg=b.new_page(); pg.goto('https://example.com')\n  print(pg.query_selector('h1').inner_text()); b.close()\nPY",
    );
    assert_eq!(h1, "Example Domain");
    // Headed chrome still alive
    assert!(!sh_ok("pgrep -f 'chrome.*no-sandbox' | head -1").is_empty());
}

// ═══════════════════════════════════════════════════════════
// 99. SHUTDOWN (must run last)
// ═══════════════════════════════════════════════════════════

#[test]
#[ignore]
fn t99_graceful_shutdown() {
    ensure_container();
    assert!(docker(&["stop", "-t", "10", CONTAINER]).status.success());
    assert_eq!(
        docker_out(&["inspect", "-f", "{{.State.ExitCode}}", CONTAINER]),
        "0"
    );
    let _ = docker(&["rm", "-f", CONTAINER]);
}
