# Web Scraping

reach includes two scraping tools inside the sandbox container: **Scrapling** (Python, adaptive selectors, anti-bot bypass) and **Playwright** (browser automation with headless Chromium). Both are available as MCP tools and can also be used directly via `reach exec`.

## Scrapling

[Scrapling](https://github.com/D4Vinci/Scrapling) is a Python web scraping library with adaptive selectors and anti-bot capabilities.

### Features

- **Adaptive selectors**: selectors that survive minor page layout changes.
- **Anti-bot bypass**: built-in handling for common bot detection (Cloudflare, etc.).
- **Fetchers**: multiple backend fetchers for different scenarios.

### Installation in Container

Scrapling and its fetchers are pre-installed in the Docker image:

```dockerfile
RUN pip install --break-system-packages playwright "scrapling[fetchers]"
RUN scrapling install
```

The `scripts/setup-scrapling.sh` script handles post-install browser setup.

### Usage via MCP

Use the `scrape` tool:

```json
{
  "name": "scrape",
  "arguments": {
    "url": "https://example.com",
    "selector": "article h2"
  }
}
```

### Usage via exec

```bash
reach exec my-sandbox -- python3 -c "
from scrapling import Fetcher

fetcher = Fetcher()
page = fetcher.get('https://example.com')
for heading in page.css('h1'):
    print(heading.text)
"
```

### Adaptive Selectors

Scrapling can find elements even when the page structure changes slightly:

```python
from scrapling import Adaptor

# First visit: learn the page structure
page = Adaptor(html_content, auto_match=True)
element = page.css('.product-title').first

# Later visits: Scrapling adapts if the class name changes
# as long as the element structure is similar
```

### Anti-Bot Fetchers

For sites with bot detection:

```python
from scrapling import StealthyFetcher

fetcher = StealthyFetcher()
page = fetcher.get('https://protected-site.com')
print(page.status)
print(page.text)
```

## Playwright

[Playwright](https://playwright.dev/python/) provides browser automation with a headless Chromium instance.

### Installation in Container

Pre-installed in the Docker image:

```dockerfile
RUN pip install --break-system-packages playwright
RUN playwright install chromium
```

### Usage via MCP

Use the `playwright_eval` tool for arbitrary Playwright scripts:

```json
{
  "name": "playwright_eval",
  "arguments": {
    "script": "page.goto('https://example.com')\nprint(page.title())"
  }
}
```

The script has access to `page` and `browser` objects pre-configured.

### Usage via exec

```bash
reach exec my-sandbox -- python3 -c "
from playwright.sync_api import sync_playwright

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page()
    page.goto('https://example.com')
    print(page.title())
    browser.close()
"
```

### Headless vs. Headed

Playwright runs headless Chromium by default. To run headed (visible on the virtual display):

```python
browser = p.chromium.launch(headless=False)
```

The headed browser window appears on Xvfb `:99` and is visible through VNC/noVNC. This is useful for debugging or when you need to interact with the page visually.

### Waiting for Content

```python
page.goto('https://example.com')
page.wait_for_selector('.dynamic-content')
content = page.query_selector('.dynamic-content').text_content()
```

### Screenshots via Playwright

```python
page.goto('https://example.com')
page.screenshot(path='/tmp/page.png', full_page=True)
```

## Headed Chrome

The sandbox includes Google Chrome (stable), running on the virtual display. This is separate from Playwright's bundled Chromium.

### Launching via MCP

Use the `browse` tool:

```json
{
  "name": "browse",
  "arguments": {
    "url": "https://example.com"
  }
}
```

### Launching via exec

```bash
reach exec my-sandbox -- google-chrome-stable \
  --no-first-run \
  --disable-background-timer-throttling \
  --disable-renderer-backgrounding \
  "https://example.com"
```

Chrome is configured with managed policies (`config/chrome-policies.json`) that disable auto-updates and first-run dialogs.

### Chrome on the Display

Chrome renders on Xvfb `:99`. You can:

1. View it through noVNC at `http://localhost:6080`
2. Capture it with `reach screenshot`
3. Interact with it using `click`, `type`, and `key` MCP tools

## Auth Handoff for Login-Walled Sites

Some sites — Threads, Instagram, LinkedIn, paywalled news, internal tools — gate their content behind a login wall. Reach exposes two MCP tools that turn this into a clean human-in-the-loop flow:

- `auth_handoff` — opens the URL in Chrome on Xvfb, returns the noVNC URL for the human, and (optionally) polls for an auth-complete signal.
- `page_text` — Playwright-driven "navigate and extract rendered text". Reuses the same persistent profile so the cookies set during `auth_handoff` carry over.

Combine both with `reach create --persist-profile <name>` so the session survives sandbox destroys.

### Threads-style example

1. **Create a sandbox with a persistent profile:**

   ```bash
   reach create --name threads --persist-profile threads
   ```

   This bind-mounts `~/.local/share/reach/profiles/threads` into the container as the Chrome user data dir.

2. **Hand off to a human for login:**

   ```json
   {
     "name": "auth_handoff",
     "arguments": {
       "url": "https://www.threads.com/login",
       "use_profile": "threads",
       "wait_for_url_contains": "/home",
       "timeout_seconds": 600
     }
   }
   ```

   The agent gets back:

   ```json
   {
     "status": "auth_required",
     "vnc_url": "http://localhost:6080/vnc.html?autoconnect=1&resize=remote",
     "instructions": "Open the vnc_url in your browser to log in..."
   }
   ```

   The human opens that URL, logs into Threads, and the polling helper notices the redirect to `/home` and returns `status: "authenticated"`.

3. **Scrape gated content with the now-authenticated profile:**

   ```json
   {
     "name": "page_text",
     "arguments": {
       "url": "https://www.threads.com/@todie.ai/post/DWzHGm0FRJw",
       "use_profile": "threads",
       "wait_for": "article",
       "timeout_ms": 30000
     }
   }
   ```

   The Playwright helper launches a `launch_persistent_context` against the same profile, navigates, waits for the `article` element, and returns:

   ```json
   {
     "status": "ok",
     "url": "https://www.threads.com/@todie.ai/post/DWzHGm0FRJw",
     "title": "...",
     "text": "..."
   }
   ```

4. **Subsequent runs skip the handoff.** As long as the cookies in `~/.local/share/reach/profiles/threads` are still valid, you can `reach destroy threads && reach create --name threads --persist-profile threads` and call `page_text` directly.

### When polling, when not

| Situation | wait_for_* | Pattern |
|-----------|-----------|---------|
| Quick interactive login (oauth, password) | `wait_for_url_contains` | One round trip |
| 2FA / SMS / Captcha | `wait_for_selector` on the post-auth element | Set `timeout_seconds` generously |
| Already authenticated (warm profile) | omit | Returns `auth_required` immediately, agent calls `page_text` next |

`auth_handoff` always launches Chrome via `launch_persistent_context`, so even the "no wait" path leaves a usable session in the profile dir.

## Choosing a Scraping Strategy

| Scenario | Tool | Why |
|----------|------|-----|
| Static HTML, simple extraction | Scrapling | Fast, no browser overhead |
| Bot-protected sites | Scrapling (StealthyFetcher) | Built-in anti-bot bypass |
| JavaScript-rendered content (no auth) | `page_text` | Playwright with sane defaults |
| JavaScript-rendered content (login wall) | `auth_handoff` + `page_text` + `--persist-profile` | Human logs in once, agent scrapes after |
| Visual interaction needed | Headed Chrome + screenshot | See what the agent sees |
| Complex multi-step automation | Playwright script | Full browser API |
| Adaptive selectors (resilient to changes) | Scrapling | Auto-match survives layout changes |

## Examples

### Scrape a table

```json
{
  "name": "scrape",
  "arguments": {
    "url": "https://example.com/data",
    "selector": "table tbody tr",
    "use_playwright": false
  }
}
```

### Fill a form with Playwright

```json
{
  "name": "playwright_eval",
  "arguments": {
    "script": "page.goto('https://example.com/login')\npage.fill('#username', 'user')\npage.fill('#password', 'pass')\npage.click('button[type=submit]')\npage.wait_for_url('**/dashboard')\nprint(page.title())"
  }
}
```

### Visual browsing with Chrome

```json
{"name": "browse", "arguments": {"url": "https://example.com"}}
```
```json
{"name": "screenshot", "arguments": {}}
```
```json
{"name": "click", "arguments": {"x": 400, "y": 300}}
```
```json
{"name": "screenshot", "arguments": {}}
```
