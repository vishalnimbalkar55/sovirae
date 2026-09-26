// Toolbar popup: connection status, playback controls, page actions, and
// per-site on/off (spec §12.2).

const $ = (id) => document.getElementById(id);
const PROBLEMS = {
  HOST_NOT_FOUND: 'Sovirae is not connected to Chrome yet. Open Sovirae once, then restart Chrome.',
  HOST_FORBIDDEN: "Sovirae's connection does not list this extension. Open Sovirae › Extension and use Test connection.",
  HOST_EXITED: "Sovirae's connection stopped unexpectedly. Use Test connection in Sovirae › Extension.",
};
const PAUSE = '<svg viewBox="0 0 24 24"><rect x="6.5" y="5.5" width="3.8" height="13" rx="1" fill="currentColor"/><rect x="13.7" y="5.5" width="3.8" height="13" rx="1" fill="currentColor"/></svg>';
const PLAY = '<svg viewBox="0 0 24 24"><path d="M8 5.5v13l10.5-6.5z" fill="currentColor"/></svg>';

let tab = null;
let origin = null;

function time(ms) {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

function send(message) {
  return chrome.runtime.sendMessage(message).catch((e) => ({ ok: false, message: e.message }));
}

function renderStatus(s) {
  const pill = $('status');
  const problem = $('problem');
  problem.hidden = true;
  if (s.connected && s.pairing === 'paired') {
    pill.className = 'pill ok';
    pill.innerHTML = '<span class="led"></span>Connected';
  } else if (s.connected && s.pairing === 'pending') {
    pill.className = 'pill warn';
    pill.textContent = 'Waiting for you';
    problem.hidden = false;
    problem.innerHTML = '<b>Allow Chrome in Sovirae.</b> A prompt is open in the Sovirae window.';
  } else if (s.connected) {
    pill.className = 'pill warn';
    pill.textContent = 'Not allowed';
    problem.hidden = false;
    problem.innerHTML = '<b>Chrome is not allowed.</b> Allow it in Sovirae › Extension.';
  } else {
    pill.className = 'pill warn';
    pill.textContent = 'Not connected';
    if (s.error) {
      problem.hidden = false;
      problem.textContent = PROBLEMS[s.errorCode] || s.error;
    }
  }

  const st = s.state;
  const active = st && st.sessionId && ['preparing', 'buffering', 'playing', 'paused', 'loadingVoice', 'recovering'].includes(st.status);
  $('now').hidden = !active;
  if (active) {
    const paused = st.status === 'paused';
    $('now-title').textContent = paused ? 'Paused' : st.status === 'playing' ? 'Reading' : 'Getting ready…';
    $('now-voice').textContent = st.voice || '';
    $('now-bar').style.width = `${st.durationMs ? Math.min(100, (st.positionMs / st.durationMs) * 100) : 0}%`;
    $('now-time').textContent = `${time(st.positionMs)} / ${st.durationIsFinal ? '' : '~'}${time(st.durationMs)}`;
    $('toggle').innerHTML = paused ? PLAY : PAUSE;
    $('toggle').setAttribute('aria-label', paused ? 'Resume' : 'Pause');
  }
}

async function renderSite() {
  const settings = await send({ type: 'sovirae:settings' });
  $('enabled').checked = !!settings.enabled;
  const supported = !!origin;
  $('site').hidden = !supported;
  $('read').disabled = $('pick').disabled = !supported || !settings.enabled || settings.disabledOrigins?.includes(origin);
  if (!supported) {
    const p = $('problem');
    p.hidden = false;
    p.innerHTML = "<b>Chrome doesn't let extensions read this page.</b> Copy the text and use Speak clipboard in Sovirae.";
    return;
  }
  $('site-on').checked = !settings.disabledOrigins?.includes(origin);
}

async function renderShortcuts() {
  const commands = await chrome.commands.getAll();
  for (const [name, target] of [['read-selection', 'read-keys'], ['pick-text', 'pick-keys']]) {
    const c = commands.find((x) => x.name === name);
    $(target).innerHTML = c?.shortcut
      ? [...c.shortcut.replace(/\+/g, '')].map((k) => `<kbd>${k}</kbd>`).join('')
      : '<span>No shortcut</span>';
  }
}

async function init() {
  [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  try {
    const u = new URL(tab?.url || '');
    origin = /^https?:$/.test(u.protocol) ? u.origin : null;
  } catch {
    origin = null;
  }
  await Promise.all([renderSite(), renderShortcuts()]);
  renderStatus(await send({ type: 'sovirae:status', probe: true }));
  send({ type: 'sovirae:clear-problem' });
}

chrome.runtime.onMessage.addListener((m) => {
  if (m?.type === 'sovirae:status-changed') renderStatus(m.status);
});

$('read').addEventListener('click', async () => {
  await send({ type: 'sovirae:popup-read-selection', tabId: tab.id });
  window.close();
});

$('pick').addEventListener('click', async () => {
  await send({ type: 'sovirae:popup-pick', tabId: tab.id });
  window.close();
});

document.querySelectorAll('[data-control]').forEach((b) =>
  b.addEventListener('click', () => {
    const c = b.dataset.control;
    const msg = c === 'skip-back' ? { action: 'skip', deltaMs: -10000 } : c === 'skip-forward' ? { action: 'skip', deltaMs: 10000 } : { action: c };
    send({ type: 'sovirae:control', ...msg });
  }),
);

$('site-on').addEventListener('change', async (e) => {
  const { disabledOrigins = [] } = await chrome.storage.local.get('disabledOrigins');
  const next = e.target.checked ? disabledOrigins.filter((o) => o !== origin) : [...new Set([...disabledOrigins, origin])];
  await chrome.storage.local.set({ disabledOrigins: next });
  renderSite();
});

$('enabled').addEventListener('change', async (e) => {
  await chrome.storage.local.set({ enabled: e.target.checked });
  renderSite();
});

$('options').addEventListener('click', () => chrome.runtime.openOptionsPage());

init();
