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
