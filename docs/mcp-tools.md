# MCP Tools Reference

reach exposes these tools to AI agents via the MCP protocol. Start the server with `reach serve --port 4200`.

## screenshot

Capture the sandbox display as a PNG image.

**Parameters:**

```json
{
  "type": "object",
  "properties": {},
  "required": []
}
```

**Example call:**

```json
{
  "name": "screenshot",
  "arguments": {}
}
```

**Returns:** Base64-encoded PNG image of the current display.

---

## click

Click at a position on the screen.

**Parameters:**

```json
{
  "type": "object",
  "properties": {
    "x": {
      "type": "integer",
      "description": "X coordinate in pixels"
    },
    "y": {
      "type": "integer",
      "description": "Y coordinate in pixels"
    },
    "button": {
      "type": "string",
      "enum": ["left", "middle", "right"],
      "default": "left",
      "description": "Mouse button to click"
    },
    "clicks": {
      "type": "integer",
      "default": 1,
      "description": "Number of clicks (2 for double-click)"
    }
  },
  "required": ["x", "y"]
}
```

**Example call:**

```json
{
  "name": "click",
  "arguments": {
    "x": 640,
    "y": 360,
    "button": "left"
  }
}
```

**Implementation:** Uses `xdotool mousemove --sync <x> <y> click <button>`.

---

## type

Type text via the keyboard.

**Parameters:**

```json
{
  "type": "object",
  "properties": {
    "text": {
      "type": "string",
      "description": "Text to type"
    },
    "delay_ms": {
      "type": "integer",
      "default": 12,
      "description": "Delay between keystrokes in milliseconds"
    }
  },
  "required": ["text"]
}
```

**Example call:**

```json
{
  "name": "type",
  "arguments": {
    "text": "hello world"
  }
}
```

**Implementation:** Uses `xdotool type --delay <delay_ms> "<text>"`.

---

## key

Send a key combination.

**Parameters:**

```json
{
  "type": "object",
  "properties": {
    "keys": {
      "type": "string",
      "description": "Key combination (e.g. 'ctrl+c', 'Return', 'alt+F4')"
    }
  },
  "required": ["keys"]
}
```

**Example call:**

```json
{
  "name": "key",
  "arguments": {
    "keys": "ctrl+l"
  }
}
```

**Implementation:** Uses `xdotool key <keys>`.

**Common key names:** Return, Tab, Escape, BackSpace, Delete, space, ctrl, alt, shift, super, F1-F12, Up, Down, Left, Right, Home, End, Page_Up, Page_Down.

---

## browse

Navigate Chrome to a URL.

**Parameters:**

```json
{
  "type": "object",
  "properties": {
    "url": {
      "type": "string",
      "description": "URL to navigate to"
    }
  },
  "required": ["url"]
}
```

**Example call:**

```json
{
  "name": "browse",
  "arguments": {
    "url": "https://example.com"
  }
}
```

**Implementation:** Launches or navigates the headed Chrome instance on the virtual display. The page is visible through VNC/noVNC.

---

## scrape

Extract structured content from a web page.

**Parameters:**

```json
{
  "type": "object",
  "properties": {
    "url": {
      "type": "string",
      "description": "URL to scrape"
    },
    "selector": {
      "type": "string",
      "description": "CSS selector to extract (optional, returns full page text if omitted)"
    },
    "wait_for": {
      "type": "string",
      "description": "CSS selector to wait for before extracting"
    },
    "use_playwright": {
      "type": "boolean",
      "default": false,
      "description": "Use Playwright instead of Scrapling for JavaScript-heavy pages"
    }
  },
  "required": ["url"]
}
```

**Example call:**

```json
{
  "name": "scrape",
  "arguments": {
    "url": "https://example.com",
    "selector": "h1",
    "use_playwright": false
  }
}
```

**Returns:** Extracted text content or structured data.

---

## playwright_eval

Execute a Playwright Python script inside the sandbox.

**Parameters:**

```json
{
  "type": "object",
  "properties": {
    "script": {
      "type": "string",
      "description": "Playwright Python script to execute. Has access to 'page' and 'browser' objects."
    },
    "timeout_ms": {
      "type": "integer",
      "default": 30000,
      "description": "Script execution timeout in milliseconds"
    }
  },
  "required": ["script"]
}
```

**Example call:**

```json
{
  "name": "playwright_eval",
  "arguments": {
    "script": "page.goto('https://example.com')\nresult = page.title()\nprint(result)"
  }
}
```

**Returns:** stdout from the script execution.

---

## page_text

Navigate to a URL using Playwright (real Chromium on the sandbox display) and return the visible text content. This is the right tool for JavaScript-heavy single-page apps that Scrapling can't render.

The browser is launched headed on Xvfb so you can watch the page through noVNC if you need to debug. Pass `use_profile` to reuse a persistent Chrome profile created with `reach create --persist-profile <name>` and skip re-authenticating every session.

**Parameters:**

```json
{
  "type": "object",
  "required": ["url"],
  "properties": {
    "url": {
      "type": "string",
      "description": "URL to load"
    },
    "wait_for": {
      "type": "string",
      "description": "CSS selector to wait for before extracting (default: networkidle)"
    },
    "selector": {
      "type": "string",
      "description": "Only extract text from elements matching this selector (default: body)"
    },
    "timeout_ms": {
      "type": "integer",
      "default": 30000,
      "description": "Max wait time in milliseconds"
    },
    "use_profile": {
      "type": "string",
      "description": "Persistent Chrome profile name (see `reach create --persist-profile`)"
    },
    "sandbox": {
      "type": "string"
    }
  }
}
```

**Example call:**

```json
{
  "name": "page_text",
  "arguments": {
    "url": "https://www.threads.com/@todie.ai/post/DWzHGm0FRJw",
    "wait_for": "article",
    "use_profile": "threads"
  }
}
```

**Returns:** JSON object with `status`, `text`, `url`, and `title` fields.

```json
{
  "status": "ok",
  "url": "https://www.threads.com/...",
  "title": "Threads",
  "text": "..."
}
```

On failure the helper still returns JSON: `{"status": "error", "message": "..."}`.

---

## auth_handoff

Open a URL in the sandbox's Chrome and pause until the user has authenticated. Returns the noVNC URL the user should open in their host browser to perform the login interactively. If `wait_for_selector` or `wait_for_url_contains` is supplied, the tool polls Playwright until the condition is met (or `timeout_seconds` elapses) and returns `status: "authenticated"`.

The browser is launched as a `launch_persistent_context` so cookies and tokens persist for follow-up `page_text` calls — combine with `--persist-profile` on the host side to survive sandbox restarts.

**Parameters:**

```json
{
  "type": "object",
  "required": ["url"],
  "properties": {
    "url": {
      "type": "string",
      "description": "URL that requires auth"
    },
    "wait_for_selector": {
      "type": "string",
      "description": "CSS selector that appears after successful auth"
    },
    "wait_for_url_contains": {
      "type": "string",
      "description": "Substring that should appear in the URL after auth"
    },
    "timeout_seconds": {
      "type": "integer",
      "default": 300,
      "description": "How long to wait for the auth signal"
    },
    "use_profile": {
      "type": "string",
      "description": "Persistent Chrome profile name (see `reach create --persist-profile`)"
    },
    "sandbox": {
      "type": "string"
    }
  }
}
```

**Example call:**

```json
{
  "name": "auth_handoff",
  "arguments": {
    "url": "https://www.threads.com/login",
    "wait_for_url_contains": "/home",
    "use_profile": "threads"
  }
}
```

**Returns:** JSON object with the noVNC URL the user should open.

```json
{
  "status": "auth_required",
  "vnc_url": "http://localhost:6080/vnc.html?autoconnect=1&resize=remote",
  "url": "https://www.threads.com/login",
  "instructions": "Open the vnc_url in your browser to log in. Re-call `auth_handoff` (with wait_for_*) or `page_text` once done."
}
```

When polling completes successfully the response is `{"status": "authenticated", ...}`. On timeout it is `{"status": "timeout", ...}`.

---

## exec

Run a shell command inside the sandbox.

**Parameters:**

```json
{
  "type": "object",
  "properties": {
    "command": {
      "type": "string",
      "description": "Shell command to execute"
    },
    "timeout_ms": {
      "type": "integer",
      "default": 30000,
      "description": "Command execution timeout in milliseconds"
    },
    "working_dir": {
      "type": "string",
      "default": "/home/sandbox",
      "description": "Working directory for the command"
    }
  },
  "required": ["command"]
}
```

**Example call:**

```json
{
  "name": "exec",
  "arguments": {
    "command": "ls -la /home/sandbox"
  }
}
```

**Returns:** JSON with `stdout`, `stderr`, and `exit_code`.

```json
{
  "stdout": "total 4\ndrwxr-xr-x 2 sandbox sandbox 4096 ...",
  "stderr": "",
  "exit_code": 0
}
```
