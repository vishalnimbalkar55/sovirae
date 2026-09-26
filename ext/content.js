// Sovirae content script: long-press trigger, block picker, confirmation
// panel, and on-page status (spec §12.3–12.7).
//
// Everything visible lives in one closed shadow root on a fixed host, so the
// page's CSS cannot restyle it and no page element is ever modified.

(() => {
  if (window.__soviraeContent) return;
  window.__soviraeContent = true;

  const T = globalThis.SoviraeText;
  const trigger = globalThis.SoviraeTrigger.createTrigger();
  const INACTIVITY_MS = 15000;
  const BLOCK_TAGS = new Set([
    'P', 'LI', 'BLOCKQUOTE', 'H1', 'H2', 'H3', 'H4', 'H5', 'H6', 'ARTICLE', 'SECTION', 'MAIN', 'PRE', 'TD', 'TH',
    'DD', 'DT', 'FIGCAPTION', 'CAPTION', 'SUMMARY', 'DETAILS', 'ASIDE', 'DIV', 'UL', 'OL', 'DL', 'TABLE', 'FIGURE',
  ]);
  const SKIP_TAGS = new Set(['SCRIPT', 'STYLE', 'NOSCRIPT', 'TEMPLATE', 'SVG', 'CANVAS', 'IFRAME', 'VIDEO', 'AUDIO', 'IMG', 'INPUT', 'TEXTAREA', 'SELECT', 'BUTTON']);

  let settings = { enabled: true, activeHere: true, trigger: 'alt', holdMs: 450 };
  let mode = 'idle'; // idle | picking | confirming
  let candidate = null;
  let history = [];
  let pointer = { x: -1, y: -1 };
  let holdTimer = null;
  let idleTimer = null;
  let frameRequest = 0;
  let ui = null;

  // ---- Messaging --------------------------------------------------------------------

  function alive() {
    try {
      return !!chrome.runtime?.id;
    } catch {
      return false;
    }
  }

  function send(message) {
    return new Promise((resolve) => {
      if (!alive()) {
        resolve({ ok: false, message: 'Sovirae was updated. Reload this page to keep using it.' });
        return;
      }
      try {
        chrome.runtime.sendMessage(message, (reply) => {
          if (chrome.runtime.lastError) resolve({ ok: false, message: chrome.runtime.lastError.message });
          else resolve(reply);
        });
      } catch (error) {
        resolve({ ok: false, message: String(error?.message || error) });
      }
    });
  }

  async function loadSettings() {
    const reply = await send({ type: 'sovirae:settings' });
    if (reply && typeof reply.enabled === 'boolean') {
      settings = reply;
      trigger.configure({ trigger: settings.trigger, holdMs: settings.holdMs });
      if (!settings.activeHere) exitPicker();
    }
  }

  // ---- UI -------------------------------------------------------------------------

  const STYLE = `
    :host { all: initial; }
    * { box-sizing: border-box; }
    .root {
      --surface: #fff; --ink: #1a1a1a; --ink2: #6b6862; --violet: #7c5cff; --action: #6242d6; --on-action: #fff;
      --hair: rgba(38,32,20,.12); --soft: rgba(124,92,255,.12); --error: #b63229;
      font: 13px/1.45 -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif; color: var(--ink);
    }
    @media (prefers-color-scheme: dark) {
      .root { --surface: #222329; --ink: #f1f0f4; --ink2: #b8b6c2; --violet: #a28bff; --action: #b5a2ff; --on-action: #17181c;
        --hair: rgba(255,255,255,.12); --soft: rgba(162,139,255,.16); --error: #ff9c91; }
    }
    .frame { position: fixed; inset: 0; border: 2px solid var(--violet); border-radius: 4px; pointer-events: none; opacity: .55; }
    .box { position: fixed; border: 2px solid var(--violet); border-radius: 6px; background: var(--soft); pointer-events: none;
      transition: top 90ms, left 90ms, width 90ms, height 90ms; }
    .chip { position: fixed; display: flex; align-items: center; gap: 8px; padding: 3px 4px 3px 9px; border-radius: 8px;
      background: var(--action); color: var(--on-action); font-weight: 600; font-size: 12px; white-space: nowrap; pointer-events: auto;
      box-shadow: 0 4px 14px rgba(0,0,0,.18); }
    .chip button { all: unset; cursor: pointer; padding: 1px 7px; border-radius: 6px; font-weight: 700; }
    .chip button:hover, .chip button:focus-visible { background: rgba(255,255,255,.22); }
    .banner { position: fixed; top: 12px; left: 50%; transform: translateX(-50%); display: flex; align-items: center; gap: 10px;
      padding: 8px 8px 8px 14px; border-radius: 12px; background: var(--surface); color: var(--ink);
      box-shadow: 0 10px 30px rgba(0,0,0,.18), 0 0 0 1px var(--hair); pointer-events: auto; max-width: calc(100vw - 24px); }
    .banner b { font-weight: 650; }
    .muted { color: var(--ink2); }
    .kbd { display: inline-block; min-width: 18px; padding: 0 5px; border-radius: 5px; border: 1px solid var(--hair);
      font-size: 11px; text-align: center; }
    .hold { position: fixed; top: 12px; left: 50%; transform: translateX(-50%); padding: 7px 14px 9px; border-radius: 12px;
      background: var(--surface); box-shadow: 0 10px 30px rgba(0,0,0,.18), 0 0 0 1px var(--hair); pointer-events: none; }
    .hold .bar { height: 3px; margin-top: 6px; border-radius: 3px; background: var(--hair); overflow: hidden; }
    .hold .bar span { display: block; height: 100%; width: 0; background: var(--action); }
    .panel { position: fixed; width: min(380px, calc(100vw - 24px)); padding: 14px; border-radius: 14px; background: var(--surface);
      box-shadow: 0 16px 40px rgba(0,0,0,.22), 0 0 0 1px var(--hair); pointer-events: auto; }
    .preview { margin: 0 0 8px; max-height: 96px; overflow: hidden; color: var(--ink); font: 14px/1.5 ui-serif, Georgia, serif;
      -webkit-mask-image: linear-gradient(#000 70%, transparent); mask-image: linear-gradient(#000 70%, transparent); }
    .meta { font-size: 12px; color: var(--ink2); margin-bottom: 12px; }
    .actions { display: flex; gap: 6px; flex-wrap: wrap; }
    button.btn { all: unset; box-sizing: border-box; display: inline-flex; align-items: center; justify-content: center; height: 30px;
      padding: 0 12px; border-radius: 8px; cursor: pointer; font-weight: 600; font-size: 13px; color: var(--ink);
      box-shadow: inset 0 0 0 1px var(--hair); }
    button.btn:hover { background: var(--soft); }
    button.btn:focus-visible { outline: 2px solid var(--violet); outline-offset: 2px; }
    button.primary { background: var(--action); color: var(--on-action); box-shadow: none; }
    button.primary:hover { background: var(--action); filter: brightness(1.08); }
    .spacer { flex: 1; }
    .toast { position: fixed; right: 16px; bottom: 16px; display: flex; align-items: center; gap: 10px; max-width: min(420px, calc(100vw - 32px));
      padding: 10px 10px 10px 12px; border-radius: 12px; background: var(--surface); color: var(--ink);
      box-shadow: 0 12px 32px rgba(0,0,0,.2), 0 0 0 1px var(--hair); pointer-events: auto; }
    .toast .mark { flex: none; width: 20px; height: 20px; color: var(--violet); }
    .toast.error .mark { color: var(--error); }
    .toast .msg { flex: 1; }
    .toast button { all: unset; cursor: pointer; padding: 2px 6px; border-radius: 6px; color: var(--ink2); font-size: 16px; line-height: 1; }
    .toast button:hover { background: var(--soft); color: var(--ink); }
    @media (prefers-reduced-motion: reduce) { .box { transition: none; } }
  `;

  const MARK = '<svg class="mark" viewBox="0 0 24 24" fill="none"><path d="M15.5 3.5v3.2c0 1-.9 1.6-3 1.6-2.4 0-4.8.9-4.8 3.3 0 2.2 2 2.8 4.3 3.3 2.4.5 4.5 1.3 4.5 3.5 0 2.3-2.2 3.3-4.7 3.3-2 0-3.6-.6-4.6-1.7" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"/></svg>';

  function ensureUi() {
    if (ui) return ui;
    const host = document.createElement('sovirae-overlay');
    host.style.cssText = 'all: initial; position: fixed; inset: 0; z-index: 2147483647; pointer-events: none;';
    const shadow = host.attachShadow({ mode: 'closed' });
    shadow.innerHTML = `<style>${STYLE}</style><div class="root"></div>`;
    (document.body || document.documentElement).appendChild(host);
    ui = { host, root: shadow.querySelector('.root'), frame: null, box: null, chip: null, banner: null, hold: null, panel: null, toast: null, toastTimer: 0 };
    return ui;
  }

  function el(tag, cls, html) {
    const node = document.createElement(tag);
    if (cls) node.className = cls;
    if (html != null) node.innerHTML = html;
    return node;
  }

  function remove(key) {
    if (ui && ui[key]) {
      ui[key].remove();
      ui[key] = null;
    }
  }

  function isOurs(node) {
    return ui && (node === ui.host || ui.host.contains(node));
  }

  function eventFromUi(e) {
    return ui && e.composedPath().includes(ui.host);
  }

  function showToast(message, tone = 'info', ms = 6000) {
    const u = ensureUi();
    remove('toast');
    const t = el('div', `toast ${tone === 'error' ? 'error' : ''}`);
    t.setAttribute('role', tone === 'error' ? 'alert' : 'status');
    t.innerHTML = `${MARK}<span class="msg"></span><button aria-label="Dismiss">×</button>`;
    t.querySelector('.msg').textContent = message;
    t.querySelector('button').addEventListener('click', () => remove('toast'));
    u.root.appendChild(t);
    u.toast = t;
    clearTimeout(u.toastTimer);
    if (ms) u.toastTimer = setTimeout(() => remove('toast'), ms);
  }

  // ---- Candidate discovery -------------------------------------------------------------

  function hidden(node) {
    if (node.closest('[aria-hidden="true"], [inert], [hidden]')) return true;
    const style = getComputedStyle(node);
    return style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0';
  }

  function editable(node) {
    return !!node?.closest?.('input, textarea, select, [contenteditable=""], [contenteditable="true"]');
  }

  function readable(node) {
    if (!(node instanceof Element) || SKIP_TAGS.has(node.tagName) || isOurs(node)) return false;
    if (node === document.body || node === document.documentElement) return false;
    if (editable(node) || hidden(node)) return false;
    const text = node.innerText || '';
    return text.trim().length > 0;
  }

  function isBlock(node) {
    if (BLOCK_TAGS.has(node.tagName)) return true;
    const d = getComputedStyle(node).display;
    return d === 'block' || d === 'list-item' || d === 'flex' || d === 'grid' || d === 'table' || d === 'table-cell';
  }

  /** The smallest readable block under the pointer. */
  function blockAt(x, y) {
    let node = document.elementFromPoint(x, y);
    while (node && isOurs(node)) node = null;
    while (node && node !== document.body) {
      if (node instanceof Element && isBlock(node) && readable(node)) return node;
      node = node.parentElement;
    }
    return null;
  }

  function widerThan(node) {
    let p = node?.parentElement;
    while (p && p !== document.body) {
      if (isBlock(p) && readable(p) && (p.innerText || '').trim().length > (node.innerText || '').trim().length) return p;
      p = p.parentElement;
    }
    return null;
  }

  function extract(node) {
    // Live innerText follows rendering: hidden nodes are skipped and blocks
    // are separated, unlike a detached clone (spec §12.6).
    return T.tidy(node.innerText || '');
  }

  // ---- Highlight -----------------------------------------------------------------------

  function describe(node) {
    const tag = node.tagName.toLowerCase();
    const names = { p: 'Paragraph', li: 'List item', blockquote: 'Quote', pre: 'Code', td: 'Cell', th: 'Cell', article: 'Article', section: 'Section', main: 'Main content', ul: 'List', ol: 'List', table: 'Table', figcaption: 'Caption' };
    const name = /^h[1-6]$/.test(tag) ? 'Heading' : names[tag] || 'Block';
    const words = T.wordCount(node.innerText || '');
    return `${name}, ${words.toLocaleString()} ${words === 1 ? 'word' : 'words'}`;
  }

  function paint() {
    frameRequest = 0;
    if (mode === 'idle' || !candidate || !candidate.isConnected) {
      remove('box');
      remove('chip');
      return;
    }
    const u = ensureUi();
    const r = candidate.getBoundingClientRect();
    if (!u.box) {
      u.box = el('div', 'box');
      u.root.appendChild(u.box);
    }
    Object.assign(u.box.style, { top: `${r.top - 3}px`, left: `${r.left - 3}px`, width: `${r.width + 6}px`, height: `${r.height + 6}px` });
    if (mode === 'picking') {
      if (!u.chip) {
        u.chip = el('div', 'chip');
        u.chip.innerHTML = '<span class="label"></span><button data-act="wider" title="Wider (↑)">↑</button><button data-act="narrower" title="Narrower (↓)">↓</button>';
        u.chip.addEventListener('click', (e) => {
          const act = e.target?.dataset?.act;
          if (act === 'wider') widen();
          if (act === 'narrower') narrow();
        });
        u.root.appendChild(u.chip);
      }
      u.chip.querySelector('.label').textContent = describe(candidate);
      const top = r.top > 34 ? r.top - 32 : Math.min(r.bottom + 6, innerHeight - 30);
      Object.assign(u.chip.style, { top: `${top}px`, left: `${Math.max(8, Math.min(r.left, innerWidth - 260))}px` });
    } else {
      remove('chip');
    }
  }

  function schedulePaint() {
    if (!frameRequest) frameRequest = requestAnimationFrame(paint);
  }

  function setCandidate(node, keepHistory = false) {
    if (node === candidate) return;
    if (!keepHistory) history = [];
    candidate = node;
    schedulePaint();
  }

  function widen() {
    const wider = candidate && widerThan(candidate);
    if (wider) {
      history.push(candidate);
      setCandidate(wider, true);
    }
    touch();
  }

  function narrow() {
    const previous = history.pop();
    if (previous) setCandidate(previous, true);
    touch();
  }

  // ---- Picker lifecycle -------------------------------------------------------------------

  function touch() {
    clearTimeout(idleTimer);
    if (mode === 'picking') idleTimer = setTimeout(exitPicker, INACTIVITY_MS);
  }

  function arm() {
    if (!settings.activeHere) return;
    trigger.arm();
    mode = 'picking';
    const u = ensureUi();
    remove('hold');
    if (!u.frame) {
      u.frame = el('div', 'frame');
      u.root.appendChild(u.frame);
    }
    remove('banner');
    u.banner = el('div', 'banner');
    u.banner.innerHTML = `${MARK}<span><b>Pick text to read.</b> <span class="muted">Click a block. <span class="kbd">↑</span> <span class="kbd">↓</span> resize, <span class="kbd">esc</span> exits.</span></span><button class="btn" data-act="close">Done</button>`;
    u.banner.querySelector('[data-act=close]').addEventListener('click', exitPicker);
    u.root.appendChild(u.banner);
    send({ type: 'sovirae:picker-armed' });
    if (pointer.x >= 0) setCandidate(blockAt(pointer.x, pointer.y));
    touch();
  }

  function exitPicker() {
    clearTimeout(holdTimer);
    clearTimeout(idleTimer);
    trigger.disarm();
    mode = 'idle';
    candidate = null;
    history = [];
    for (const key of ['frame', 'box', 'chip', 'banner', 'hold', 'panel']) remove(key);
  }

  function startHold() {
    const u = ensureUi();
    remove('hold');
    u.hold = el('div', 'hold', '<span>Keep holding to pick text</span><div class="bar"><span></span></div>');
    u.root.appendChild(u.hold);
    const bar = u.hold.querySelector('.bar span');
    bar.animate([{ width: '0%' }, { width: '100%' }], { duration: trigger.holdMs, easing: 'linear', fill: 'forwards' });
    const poll = () => {
      if (trigger.tick() === 'armed') arm();
      else if (trigger.state === 'holding') holdTimer = setTimeout(poll, 30);
    };
    holdTimer = setTimeout(poll, 30);
  }

  function cancelHold() {
    clearTimeout(holdTimer);
    remove('hold');
  }

  // ---- Confirmation -----------------------------------------------------------------------

  function confirmBlock(node) {
    mode = 'confirming';
    clearTimeout(idleTimer);
    candidate = node;
    paint();
    const u = ensureUi();
    remove('banner');
    remove('panel');
    const text = extract(node);
    const tooLong = text.length > T.MAX_TEXT;
    const words = T.wordCount(text);
    const panel = el('div', 'panel');
    panel.setAttribute('role', 'dialog');
    panel.setAttribute('aria-label', 'Read this text with Sovirae');
    panel.innerHTML = `<p class="preview"></p><div class="meta"></div><div class="actions">
      <button class="btn primary" data-act="listen"></button><button class="btn" data-act="copy">Copy text</button>
      <span class="spacer"></span><button class="btn" data-act="back">Back</button><button class="btn" data-act="close">Close</button></div>`;
    panel.querySelector('.preview').textContent = text.slice(0, 400);
    panel.querySelector('.meta').textContent = tooLong
      ? `${words.toLocaleString()} words. Longer than Sovirae's limit, so it reads the first 200,000 characters.`
      : `${words.toLocaleString()} ${words === 1 ? 'word' : 'words'}, ${T.estimate(words).toLowerCase() || 'under 1 min'}`;
    panel.querySelector('[data-act=listen]').textContent = tooLong ? 'Read first part' : 'Listen';
    // Snapshot now: later page changes do not alter what was previewed (spec §12.5).
    const snapshot = tooLong ? T.truncate(text) : text;
    panel.addEventListener('click', async (e) => {
      const act = e.target?.closest?.('[data-act]')?.dataset.act;
      if (act === 'listen') {
        exitPicker();
        readText(snapshot);
      } else if (act === 'copy') {
        copy(text, e.target);
      } else if (act === 'back') {
        remove('panel');
        mode = 'idle';
        arm();
        setCandidate(node);
      } else if (act === 'close') {
        exitPicker();
      }
    });
    u.root.appendChild(panel);
    u.panel = panel;
    placePanel(node);
    panel.querySelector('[data-act=listen]').focus({ preventScroll: true });
  }

  function placePanel(node) {
    const p = ui?.panel;
    if (!p) return;
    const r = node.getBoundingClientRect();
    const h = p.offsetHeight || 180;
    const w = p.offsetWidth || 380;
    let top = r.bottom + 10;
    if (top + h > innerHeight - 8) top = r.top - h - 10;
    if (top < 8) top = Math.min(innerHeight - h - 8, Math.max(8, r.top + 8));
    const left = Math.max(8, Math.min(r.left, innerWidth - w - 8));
    Object.assign(p.style, { top: `${top}px`, left: `${left}px` });
  }

  async function copy(text, button) {
    let ok = false;
    try {
      await navigator.clipboard.writeText(text);
      ok = true;
    } catch {
      // Fallback for pages that block the async clipboard.
      const area = document.createElement('textarea');
      area.value = text;
      area.style.cssText = 'position:fixed;opacity:0;pointer-events:none;';
      ensureUi().root.appendChild(area);
      area.select();
      ok = document.execCommand('copy');
      area.remove();
    }
    if (button) {
      button.textContent = ok ? 'Copied' : 'Copy failed';
      setTimeout(() => (button.textContent = 'Copy text'), 1600);
    }
    if (!ok) showToast('The page blocked copying. Select the preview text and copy it manually.', 'error');
  }

  // ---- Reading -------------------------------------------------------------------------------

  const MESSAGES = {
    HOST_NOT_FOUND: 'Sovirae is not connected to Chrome yet. Open Sovirae once, then restart Chrome.',
    HOST_FORBIDDEN: "Sovirae's connection does not allow this extension. Open Sovirae › Extension to fix it.",
    HOST_EXITED: "Sovirae's connection stopped unexpectedly. Use Test connection in Sovirae › Extension.",
    APP_UNAVAILABLE: null,
    PAIRING_DECLINED: 'Chrome is not allowed to use Sovirae. Allow it in Sovirae › Extension.',
    NOT_AUTHORIZED: 'Allow Chrome in the Sovirae window, then try again.',
    NO_VOICE: 'Choose or download a voice in Sovirae first.',
    NO_TEXT: 'There is no readable text there.',
    TEXT_TOO_LONG: 'That text is longer than 200,000 characters.',
  };

  async function readText(text) {
    const clean = T.tidy(text);
    if (!clean) {
      showToast('There is no readable text there.', 'error');
      return;
    }
    const payload = clean.length > T.MAX_TEXT ? T.truncate(clean) : clean;
    showToast('Sending to Sovirae…', 'info', 0);
    const reply = await send({
      type: 'sovirae:speak',
      text: payload,
      origin: location.origin,
      title: document.title,
      lang: T.pageLanguage(document),
    });
    if (reply?.ok) {
      showToast(reply.voice ? `Reading with ${reply.voice}.` : 'Reading in Sovirae.', 'info', 2500);
      return;
    }
    const message = (reply?.code && MESSAGES[reply.code]) || reply?.message || 'Sovirae could not read that.';
    showToast(message, 'error', 10000);
  }

  // ---- Events ----------------------------------------------------------------------------------

  function onKeyDown(e) {
    if (!settings.activeHere) return;
    if (mode === 'picking' || mode === 'confirming') {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopImmediatePropagation();
        exitPicker();
        return;
      }
      if (mode === 'picking') {
        if (e.key === 'ArrowUp') (e.preventDefault(), e.stopImmediatePropagation(), widen());
        else if (e.key === 'ArrowDown') (e.preventDefault(), e.stopImmediatePropagation(), narrow());
        else if (e.key === 'Enter' && candidate) (e.preventDefault(), e.stopImmediatePropagation(), confirmBlock(candidate));
      }
      return;
    }
    const result = trigger.keydown(
      { key: e.key, repeat: e.repeat, isComposing: e.isComposing, altGraph: e.getModifierState?.('AltGraph'), shiftKey: e.shiftKey, metaKey: e.metaKey, ctrlKey: e.ctrlKey, altKey: e.altKey },
      { editable: editable(document.activeElement) },
    );
    if (result === 'hold-start') startHold();
    else if (result === 'cancel') cancelHold();
  }

  function onKeyUp(e) {
    if (trigger.keyup({ key: e.key }) === 'cancel') cancelHold();
  }

  function onPointerMove(e) {
    pointer = { x: e.clientX, y: e.clientY };
    if (mode !== 'picking' || eventFromUi(e)) return;
    touch();
    const node = blockAt(e.clientX, e.clientY);
    if (node && node !== candidate && !history.includes(node)) setCandidate(node);
  }

  /** While picking, page clicks choose a block instead of following links. */
  function onPointerEvent(e) {
    if (trigger.state === 'holding' && e.type === 'pointerdown') {
      trigger.pointerdown();
      cancelHold();
      return;
    }
    if (mode === 'idle' || eventFromUi(e)) return;
    if (e.type === 'contextmenu' || (e.type === 'pointerdown' && e.button === 2)) {
      e.preventDefault();
      e.stopImmediatePropagation();
      exitPicker();
      return;
    }
    e.preventDefault();
    e.stopImmediatePropagation();
    if (e.type === 'click') {
      const node = blockAt(e.clientX, e.clientY) || candidate;
      if (node) confirmBlock(node);
    }
  }

  function onWheel(e) {
    // Option/Alt + wheel resizes; plain scrolling keeps working.
    if (mode !== 'picking' || !e.altKey) return;
    e.preventDefault();
    if (e.deltaY < 0) widen();
    else if (e.deltaY > 0) narrow();
  }

  const opts = { capture: true };
  window.addEventListener('keydown', onKeyDown, opts);
  window.addEventListener('keyup', onKeyUp, opts);
  window.addEventListener('pointermove', onPointerMove, { capture: true, passive: true });
  for (const type of ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click', 'auxclick', 'dblclick', 'contextmenu']) {
    window.addEventListener(type, onPointerEvent, opts);
  }
  window.addEventListener('wheel', onWheel, { capture: true, passive: false });
  window.addEventListener('scroll', () => (mode !== 'idle' ? (schedulePaint(), candidate && ui?.panel && placePanel(candidate)) : null), { capture: true, passive: true });
  window.addEventListener('resize', schedulePaint, { passive: true });
  window.addEventListener('blur', () => {
    cancelHold();
    trigger.disarm();
    if (mode === 'picking') exitPicker();
  });
  document.addEventListener('visibilitychange', () => document.hidden && exitPicker());
  window.addEventListener('pagehide', exitPicker);

  chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    switch (message?.type) {
      case 'sovirae:arm':
        arm();
        break;
      case 'sovirae:disarm':
        // Another frame of this tab took over the picker.
        exitPicker();
        break;
      case 'sovirae:read-text':
        readText(message.text);
        break;
      case 'sovirae:toast':
        showToast(message.message, message.tone === 'error' ? 'error' : 'info', message.tone === 'wait' ? 0 : 7000);
        break;
      default:
        break;
    }
    sendResponse({ ok: true });
  });

  if (alive()) {
    chrome.storage.onChanged.addListener((_changes, area) => area === 'local' && loadSettings());
  }
  loadSettings();
})();
