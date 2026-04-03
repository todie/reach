(function(){let e=document.createElement(`link`).relList;if(e&&e.supports&&e.supports(`modulepreload`))return;for(let e of document.querySelectorAll(`link[rel="modulepreload"]`))n(e);new MutationObserver(e=>{for(let t of e)if(t.type===`childList`)for(let e of t.addedNodes)e.tagName===`LINK`&&e.rel===`modulepreload`&&n(e)}).observe(document,{childList:!0,subtree:!0});function t(e){let t={};return e.integrity&&(t.integrity=e.integrity),e.referrerPolicy&&(t.referrerPolicy=e.referrerPolicy),e.crossOrigin===`use-credentials`?t.credentials=`include`:e.crossOrigin===`anonymous`?t.credentials=`omit`:t.credentials=`same-origin`,t}function n(e){if(e.ep)return;e.ep=!0;let n=t(e);fetch(e.href,n)}})();var e=document.querySelector(`#app`);e.innerHTML=`
<!-- Nav -->
<nav class="nav">
  <div class="nav-inner">
    <a href="#" class="nav-logo">
      <span class="logo-icon">R</span>
      <span>reach</span>
    </a>
    <div class="nav-links">
      <a href="#features">Features</a>
      <a href="#inside">What's Inside</a>
      <a href="#architecture">Architecture</a>
      <a href="#quickstart">Quickstart</a>
      <a href="https://github.com/todie/reach" target="_blank" class="nav-github">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/></svg>
        GitHub
      </a>
    </div>
  </div>
</nav>

<!-- Hero -->
<section class="hero">
  <div class="hero-badge">Open Source -- 88 Tests Passing</div>
  <h1 class="hero-title">
    Give AI agents<br/>
    <span class="gradient-text">a desktop to drive.</span>
  </h1>
  <p class="hero-sub">
    Sandboxed Linux desktop with Chrome, Playwright, and anti-bot scraping.<br/>
    Boots in 1 second. Disposable. Observable. One command.
  </p>
  <div class="hero-actions">
    <a href="#quickstart" class="btn btn-primary">Get Started</a>
    <a href="https://github.com/todie/reach" target="_blank" class="btn btn-secondary">View on GitHub</a>
  </div>
  <div class="hero-terminal">
    <div class="terminal-bar">
      <span class="terminal-dot red"></span>
      <span class="terminal-dot yellow"></span>
      <span class="terminal-dot green"></span>
      <span class="terminal-title">terminal</span>
    </div>
    <pre class="terminal-body"><code><span class="t-prompt">$</span> <span class="t-cmd">reach create</span> <span class="t-flag">--name</span> sandbox
<span class="t-cyan">reach</span> <span class="t-dim">|</span> Creating sandbox <span class="t-str">"sandbox"</span>...
<span class="t-cyan">reach</span> <span class="t-dim">|</span>   Display    Xvfb :99 (1280x720)
<span class="t-cyan">reach</span> <span class="t-dim">|</span>   Browser    Chrome 136 + Playwright
<span class="t-cyan">reach</span> <span class="t-dim">|</span>   Scraping   Scrapling (Cloudflare bypass)
<span class="t-cyan">reach</span> <span class="t-dim">|</span>   VNC        http://localhost:6080
<span class="t-ok">reach</span> <span class="t-dim">|</span> <span class="t-ok">Sandbox "sandbox" ready in 1.1s.</span>

<span class="t-prompt">$</span> <span class="t-cmd">reach list</span>
<span class="t-header">NAME       STATUS    VNC              UPTIME</span>
<span class="t-text">sandbox    running   localhost:6080   12s</span>

<span class="t-prompt">$</span> <span class="t-cmd">reach serve</span> <span class="t-flag">--sandbox</span> sandbox
<span class="t-cyan">mcp</span>  <span class="t-dim">|</span> MCP server listening on <span class="t-str">:4200</span>
<span class="t-cyan">mcp</span>  <span class="t-dim">|</span> 8 tools: screenshot, click, type, key, browse, scrape, playwright_eval, exec
<span class="t-ok">mcp</span>  <span class="t-dim">|</span> <span class="t-ok">Claude Code connected.</span></code></pre>
  </div>
</section>

<!-- Problem -->
<section class="problem" id="problem">
  <div class="section-inner">
    <h2 class="section-label">The Problem</h2>
    <div class="problem-grid">
      <div class="problem-card">
        <div class="problem-icon">X</div>
        <h3>Agents can't see the web</h3>
        <p>Paywalls, anti-bot protection, dynamic JS rendering, CAPTCHAs. Text-only APIs miss half the internet.</p>
      </div>
      <div class="problem-card">
        <div class="problem-icon">X</div>
        <h3>Desktop access is dangerous</h3>
        <p>Giving an AI agent your real desktop means it can read your email, access your credentials, and break things.</p>
      </div>
      <div class="problem-card">
        <div class="problem-icon">X</div>
        <h3>Setup is painful</h3>
        <p>Xvfb, Chrome, Playwright, VNC, MCP servers, process supervision &mdash; hours of yak-shaving before anything works.</p>
      </div>
    </div>
  </div>
</section>

<!-- Features -->
<section class="features" id="features">
  <div class="section-inner">
    <h2 class="section-label">The Solution</h2>
    <p class="section-desc">Everything an AI agent needs to interact with the visual web, in a single disposable container.</p>
    <div class="feature-grid">
      <div class="feature-card highlight">
        <div class="feature-emoji">D</div>
        <h3>Full Desktop in 1 Second</h3>
        <p>Xvfb + openbox + x11vnc + noVNC. A real GUI environment your agent can screenshot, click, and type in. Boots in under 2 seconds.</p>
      </div>
      <div class="feature-card">
        <div class="feature-emoji">S</div>
        <h3>Rust Supervisor (PID 1)</h3>
        <p>Process management, health API, and Prometheus metrics endpoint. Manages all child processes with proper lifecycle and signal handling.</p>
      </div>
      <div class="feature-card">
        <div class="feature-emoji">C</div>
        <h3>Chrome + Playwright</h3>
        <p>Headed Chrome on the virtual display for anti-bot scenarios. Headless Playwright Chromium for fast scripting. Both available simultaneously.</p>
      </div>
      <div class="feature-card">
        <div class="feature-emoji">A</div>
        <h3>Scrapling</h3>
        <p>Adaptive web scraping with anti-bot bypass. Selectors that survive site redesigns. Cloudflare Turnstile? Handled.</p>
      </div>
      <div class="feature-card">
        <div class="feature-emoji">M</div>
        <h3>MCP Server (8 Tools)</h3>
        <p>screenshot, click, type, key, browse, scrape, playwright_eval, exec. Any MCP-compatible client (Claude Code, Claude Desktop) can connect.</p>
      </div>
      <div class="feature-card">
        <div class="feature-emoji">R</div>
        <h3>Rust CLI (8 Commands)</h3>
        <p>create, destroy, list, exec, vnc, screenshot, connect, serve. Fast, typed, no runtime dependencies. Full container lifecycle from the terminal.</p>
      </div>
      <div class="feature-card">
        <div class="feature-emoji">U</div>
        <h3>computer-use-mcp</h3>
        <p>Desktop control via Model Context Protocol. Screenshot, click, type, and key actions against the full virtual display.</p>
      </div>
      <div class="feature-card">
        <div class="feature-emoji">T</div>
        <h3>88 Tests, CI/CD</h3>
        <p>52 unit tests + 36 end-to-end tests, all passing. 3 GitHub Actions workflows for continuous integration. Confidence on every commit.</p>
      </div>
      <div class="feature-card">
        <div class="feature-emoji">P</div>
        <h3>Observable</h3>
        <p>Prometheus /metrics endpoint, /health API on the supervisor. See what your agent is doing in real time.</p>
      </div>
    </div>
  </div>
</section>

<!-- What's Inside -->
<section class="inside" id="inside">
  <div class="section-inner">
    <h2 class="section-label">What's Inside</h2>
    <p class="section-desc">The full container stack, from PID 1 to the browser.</p>
    <div class="stack-container">
      <div class="stack-layer stack-supervisor">
        <div class="stack-label">PID 1</div>
        <div class="stack-title">reach-supervisor <span class="stack-lang">Rust</span></div>
        <div class="stack-detail">/health &middot; /metrics &middot; process management</div>
      </div>
      <div class="stack-row">
        <div class="stack-layer stack-display">
          <div class="stack-label">Display</div>
          <div class="stack-title">Xvfb :99</div>
          <div class="stack-detail">1280x720 virtual framebuffer</div>
        </div>
        <div class="stack-layer stack-wm">
          <div class="stack-label">Window Manager</div>
          <div class="stack-title">openbox</div>
          <div class="stack-detail">lightweight X11 WM</div>
        </div>
        <div class="stack-layer stack-vnc">
          <div class="stack-label">VNC</div>
          <div class="stack-title">x11vnc + noVNC</div>
          <div class="stack-detail">:5900 / :6080 (web)</div>
        </div>
      </div>
      <div class="stack-row">
        <div class="stack-layer stack-chrome">
          <div class="stack-label">Browser</div>
          <div class="stack-title">Chrome 136</div>
          <div class="stack-detail">headed, on display</div>
        </div>
        <div class="stack-layer stack-playwright">
          <div class="stack-label">Scripting</div>
          <div class="stack-title">Playwright</div>
          <div class="stack-detail">headless Chromium</div>
        </div>
        <div class="stack-layer stack-scrapling">
          <div class="stack-label">Scraping</div>
          <div class="stack-title">Scrapling</div>
          <div class="stack-detail">anti-bot + CF bypass</div>
        </div>
      </div>
      <div class="stack-layer stack-mcp">
        <div class="stack-label">Interface</div>
        <div class="stack-title">computer-use-mcp <span class="stack-lang">Python</span></div>
        <div class="stack-detail">screenshot &middot; click &middot; type &middot; key</div>
      </div>
    </div>
  </div>
</section>

<!-- Architecture -->
<section class="architecture" id="architecture">
  <div class="section-inner">
    <h2 class="section-label">Architecture</h2>
    <p class="section-desc">Two Rust binaries. One on your host, one inside the container.</p>
    <div class="arch-diagram">
      <pre><code><span class="arch-host">HOST</span>
<span class="arch-box">reach CLI <span class="arch-dim">(Rust)</span>
  create / destroy / list / exec / vnc / screenshot / connect / serve
  MCP SSE server on :4200 (8 tools)</span>
       <span class="arch-arrow">|  Docker API (bollard)</span>
       <span class="arch-arrow">v</span>
<span class="arch-container">CONTAINER</span>
<span class="arch-box">reach-supervisor <span class="arch-dim">(Rust, PID 1)</span>
  /health  /metrics (Prometheus)

  Xvfb :99 <span class="arch-arrow">-></span> openbox <span class="arch-arrow">-></span> x11vnc :5900 <span class="arch-arrow">-></span> noVNC :6080

  Chrome          <span class="arch-dim">headed, on display</span>
  Playwright      <span class="arch-dim">headless Chromium scripting</span>
  Scrapling       <span class="arch-dim">adaptive anti-bot + Cloudflare bypass</span>
  computer-use    <span class="arch-dim">screenshot / click / type / key</span></span></code></pre>
    </div>
  </div>
</section>

<!-- Quickstart -->
<section class="quickstart" id="quickstart">
  <div class="section-inner">
    <h2 class="section-label">Quickstart</h2>
    <div class="steps">
      <div class="step">
        <div class="step-num">1</div>
        <div class="step-content">
          <h3>Install reach</h3>
          <pre><code><span class="t-prompt">$</span> cargo install reach</code></pre>
        </div>
      </div>
      <div class="step">
        <div class="step-num">2</div>
        <div class="step-content">
          <h3>Create a sandbox</h3>
          <pre><code><span class="t-prompt">$</span> reach create <span class="t-flag">--name</span> dev
<span class="t-ok">Sandbox "dev" ready in 1.1s.</span></code></pre>
        </div>
      </div>
      <div class="step">
        <div class="step-num">3</div>
        <div class="step-content">
          <h3>Connect Claude Code via MCP</h3>
          <pre><code><span class="t-prompt">$</span> reach serve <span class="t-flag">--sandbox</span> dev <span class="t-flag">--port</span> 4200
<span class="t-prompt">$</span> claude mcp add reach <span class="t-flag">--url</span> http://localhost:4200/mcp</code></pre>
        </div>
      </div>
      <div class="step">
        <div class="step-num">4</div>
        <div class="step-content">
          <h3>Watch the agent work</h3>
          <pre><code><span class="t-prompt">$</span> reach vnc dev
<span class="t-dim"># Opens VNC in your browser at localhost:6080</span>
<span class="t-dim"># Or take a quick look:</span>
<span class="t-prompt">$</span> reach screenshot dev <span class="t-flag">--out</span> snap.png</code></pre>
        </div>
      </div>
      <div class="step">
        <div class="step-num">5</div>
        <div class="step-content">
          <h3>Tear it down</h3>
          <pre><code><span class="t-prompt">$</span> reach destroy dev
<span class="t-ok">Sandbox "dev" destroyed. Nothing persists.</span></code></pre>
        </div>
      </div>
    </div>
  </div>
</section>

<!-- CTA -->
<section class="cta">
  <div class="section-inner">
    <h2 class="cta-title">Ready to give your agents eyes and hands?</h2>
    <p class="cta-sub">reach is open source and free. Star us on GitHub.</p>
    <div class="hero-actions">
      <a href="https://github.com/todie/reach" target="_blank" class="btn btn-primary">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" style="margin-right:8px"><path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/></svg>
        Star on GitHub
      </a>
      <a href="https://github.com/todie/reach#quickstart" target="_blank" class="btn btn-secondary">Read the Docs</a>
    </div>
  </div>
</section>

<!-- Footer -->
<footer class="footer">
  <div class="section-inner">
    <p>Built with Rust. Powered by Docker. MIT License.</p>
    <p class="footer-dim">todie/reach</p>
  </div>
</footer>
`,document.querySelectorAll(`a[href^="#"]`).forEach(e=>{e.addEventListener(`click`,t=>{t.preventDefault(),document.querySelector(e.getAttribute(`href`))?.scrollIntoView({behavior:`smooth`})})});var t=new IntersectionObserver(e=>{e.forEach(e=>{e.isIntersecting&&e.target.classList.add(`visible`)})},{threshold:.1});document.querySelectorAll(`.feature-card, .problem-card, .step, .arch-diagram, .stack-layer, .stack-row`).forEach(e=>{t.observe(e)});