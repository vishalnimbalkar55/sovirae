// Sovirae extension service worker (spec §12–13).
//
// Chrome stops this worker when idle, so it keeps no authoritative state:
// the native port is reopened on demand and playback state is re-queried
// from the app. All reading happens in the desktop app.

const HOST = 'com.sovirae.bridge';
const REPLY_TIMEOUT_MS = 10000;
const HELLO_TIMEOUT_MS = 8000;
const PAIRING_WAIT_MS = 90000;
const CONTENT_FILES = ['lib/text.js', 'lib/trigger.js', 'content.js'];
const LEGACY_ALWAYS_READY_ID = 'sovirae-always-ready';

let port = null;
let helloAck = null;
let helloWaiters = [];
let lastState = null;
/** Chrome's own words about the last connection failure. */
let lastError = null;
const pending = new Map();

// ---- Native connection ---------------------------------------------------------

function connect() {
  if (port) return port;
  lastError = null;
  port = chrome.runtime.connectNative(HOST);
  port.onMessage.addListener(onNativeMessage);
  port.onDisconnect.addListener(() => {
    const reason = chrome.runtime.lastError?.message || lastError || 'Sovirae closed the connection.';
    lastError = reason;
    port = null;
    helloAck = null;
    const failure = Object.assign(new Error(reason), { code: classify(reason), disconnected: true });
    for (const [, waiter] of pending) waiter.reject(failure);
    pending.clear();
    for (const w of helloWaiters) w.reject(failure);
    helloWaiters = [];
    setBadge(null);
    notifyPopup();
  });
  port.postMessage({ t: 'hello', v: 1, ext: chrome.runtime.getManifest().version });
  return port;
}

function onNativeMessage(msg) {
  if (!msg || typeof msg.t !== 'string') return;
  switch (msg.t) {
    case 'hello.ack': {
      helloAck = msg;
      const waiters = helloWaiters;
      helloWaiters = [];
      for (const w of waiters) w.resolve(msg);
      if (msg.pairing === 'paired') port?.postMessage({ t: 'state.get', v: 1 });
      notifyPopup();
      break;
    }
    case 'speak.accepted':
    case 'speak.rejected': {
      const waiter = pending.get(msg.id);
      if (waiter) {
        pending.delete(msg.id);
        waiter.resolve(msg);
      }
      break;
    }
    case 'state':
      lastState = msg;
      chrome.storage.session.set({ state: msg }).catch(() => {});
      setBadge(msg);
      notifyPopup();
      break;
    case 'error': {
      // Errors without an id (app unavailable, bad frame) settle everything waiting.
      lastError = msg.message;
      const failure = Object.assign(new Error(msg.message || 'Sovirae refused that request.'), { code: msg.code });
      if (msg.code === 'APP_UNAVAILABLE' || msg.code === 'PAYLOAD_TOO_LARGE' || msg.code === 'INVALID_REQUEST') {
        for (const [, waiter] of pending) waiter.reject(failure);
        pending.clear();
        for (const w of helloWaiters) w.reject(failure);
        helloWaiters = [];
      }
      notifyPopup();
      break;
    }
    default:
      break;
  }
}

/** Distinguishes the causes Chrome collapses into short messages. */
function classify(text) {
  const t = String(text || '');
  if (/not found/i.test(t)) return 'HOST_NOT_FOUND';
  if (/forbidden/i.test(t)) return 'HOST_FORBIDDEN';
  if (/exited|communicating/i.test(t)) return 'HOST_EXITED';
  return 'BRIDGE_ERROR';
}

function waitForHello(timeoutMs) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      helloWaiters = helloWaiters.filter((w) => w.resolve !== done);
      reject(Object.assign(new Error('Sovirae did not answer.'), { code: 'NO_REPLY' }));
    }, timeoutMs);
    const done = (ack) => {
      clearTimeout(timer);
      resolve(ack);
    };
    helloWaiters.push({ resolve: done, reject: (e) => (clearTimeout(timer), reject(e)) });
  });
}

/**
 * Connects and returns the app's hello.ack. With `onPending`, waits while the
 * user answers Sovirae's allow prompt instead of failing straight away.
 */
async function ready({ onPending } = {}) {
  connect();
  let ack = helloAck || (await waitForHello(HELLO_TIMEOUT_MS));
  if (ack.pairing === 'pending' && onPending) {
    onPending();
    ack = await waitForHello(PAIRING_WAIT_MS);
  }
  return ack;
}

function sendAndWait(payload) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(payload.id);
      reject(Object.assign(new Error('Sovirae did not answer in time.'), { code: 'NO_REPLY' }));
    }, REPLY_TIMEOUT_MS);
    pending.set(payload.id, {
      resolve: (m) => (clearTimeout(timer), resolve(m)),
      reject: (e) => (clearTimeout(timer), reject(e)),
    });
    try {
      connect().postMessage(payload);
    } catch (error) {
      clearTimeout(timer);
      pending.delete(payload.id);
      reject(error);
    }
  });
}

async function speak({ text, origin, title, lang }, onPending) {
  const ack = await ready({ onPending });
  if (ack.pairing !== 'paired') {
    throw Object.assign(new Error('This browser is not allowed to use Sovirae. Allow it in Sovirae › Extension.'), {
      code: 'PAIRING_DECLINED',
    });
  }
  const payload = {
    t: 'speak',
    v: 1,
    id: crypto.randomUUID(),
    text,
    source: { origin: origin || null, title: title ? String(title).slice(0, 300) : null },
    languageHint: lang || null,
  };
  let reply;
  try {
    reply = await sendAndWait(payload);
  } catch (error) {
    // A worker restart or reconnect lost the port: resend once with the same
    // id; the app returns the original result instead of starting twice.
    if (!error.disconnected) throw error;
    await ready({});
    reply = await sendAndWait(payload);
  }
  if (reply.t === 'speak.rejected') {
    throw Object.assign(new Error(reply.message), { code: reply.code });
  }
  return { reply, voice: ack.voice };
}

// ---- Badge and popup ---------------------------------------------------------------

function setBadge(state) {
  const active = state && ['playing', 'buffering', 'preparing'].includes(state.status);
  chrome.action.setBadgeText({ text: active ? '▶' : '' }).catch(() => {});
  chrome.action.setBadgeBackgroundColor({ color: '#6242D6' }).catch(() => {});
}

function notifyPopup() {
  chrome.runtime.sendMessage({ type: 'sovirae:status-changed', status: statusSnapshot() }).catch(() => {});
}

function statusSnapshot() {
  return {
    connected: !!port && !!helloAck,
    pairing: helloAck?.pairing ?? null,
    voice: helloAck?.voice ?? null,
    state: lastState,
    error: lastError,
    errorCode: lastError ? classify(lastError) : null,
    extensionId: chrome.runtime.id,
  };
}

// ---- Settings --------------------------------------------------------------------------

async function defaults() {
  // Option on macOS, Alt elsewhere; the content script keeps Windows from
  // treating the Alt release as "focus the browser menu" (spec §12.3).
  return { enabled: true, trigger: 'alt', holdMs: 450, disabledOrigins: [] };
}

async function getSettings() {
  const base = await defaults();
  return { ...base, ...(await chrome.storage.local.get(Object.keys(base))) };
}

function originOf(url) {
  try {
    const u = new URL(url);
    return /^https?:$/.test(u.protocol) ? u.origin : null;
  } catch {
    return null;
  }
}

async function allowedOn(url) {
  const s = await getSettings();
  const origin = originOf(url);
  return s.enabled && !!origin && !s.disabledOrigins.includes(origin);
}

// ---- Content script in open tabs --------------------------------------------------

/**
 * The manifest loads the picker into every page from now on, but Chrome does
 * not add it to tabs that were already open when the extension was installed,
 * updated, or reloaded. Load it there so a long press works without a reload.
 */
async function injectOpenTabs() {
  const tabs = await chrome.tabs.query({ url: ['http://*/*', 'https://*/*'] });
  await Promise.all(tabs.map((t) => (t.id != null ? inject(t.id).catch(() => {}) : null)));
}

/** Earlier builds registered the picker per site; drop that so it does not load twice. */
async function dropLegacyRegistration() {
  try {
    await chrome.scripting.unregisterContentScripts({ ids: [LEGACY_ALWAYS_READY_ID] });
  } catch {
    // Never registered.
  }
}

// ---- Page actions -----------------------------------------------------------------

/** Injects the content script where the user just gave access (activeTab). */
async function inject(tabId, frameIds) {
  const target = frameIds ? { tabId, frameIds } : { tabId, allFrames: true };
  try {
    await chrome.scripting.executeScript({ target, files: CONTENT_FILES });
  } catch (error) {
    if (frameIds) throw error;
    // Cross-origin frames may be off limits; the top frame is enough.
    await chrome.scripting.executeScript({ target: { tabId, frameIds: [0] }, files: CONTENT_FILES });
  }
}

function unsupportedPage(error) {
  return /cannot be scripted|cannot access|chrome:\/\/|extensions gallery|Missing host permission/i.test(String(error?.message));
}

async function flagProblem(message) {
  lastError = message;
  chrome.action.setBadgeText({ text: '!' }).catch(() => {});
  chrome.action.setBadgeBackgroundColor({ color: '#B63229' }).catch(() => {});
  notifyPopup();
}

async function toast(tabId, frameId, message, tone = 'info') {
  try {
    await chrome.tabs.sendMessage(tabId, { type: 'sovirae:toast', message, tone }, { frameId });
  } catch {
    await flagProblem(message);
  }
}

/** Reads the selection from whichever frame holds it. */
async function readSelection(tab, onlyFrame) {
  if (!tab?.id) return;
  if (!(await allowedOn(tab.url))) {
    await flagProblem('Sovirae is turned off for this site.');
    return;
  }
  let results;
  try {
    results = await chrome.scripting.executeScript({
      target: onlyFrame != null ? { tabId: tab.id, frameIds: [onlyFrame] } : { tabId: tab.id, allFrames: true },
      func: () => ({ text: String(getSelection() || ''), focused: document.hasFocus() }),
    });
  } catch (error) {
    await flagProblem(
      unsupportedPage(error)
        ? "Sovirae can't read this page. Copy the text and use Speak clipboard in Sovirae."
        : `Sovirae could not read the selection: ${error.message}`,
    );
    return;
  }
  const withText = results.filter((r) => r.result && r.result.text.trim());
  const chosen = withText.find((r) => r.result.focused) || withText[0];
  const frameId = chosen ? chosen.frameId : onlyFrame ?? 0;
  try {
    await inject(tab.id, [frameId]);
  } catch {
    // The toast falls back to the badge.
  }
  if (!chosen) {
    await toast(tab.id, frameId, 'Select some text first, or use Pick text.', 'error');
    return;
  }
  await chrome.tabs.sendMessage(tab.id, { type: 'sovirae:read-text', text: chosen.result.text }, { frameId }).catch(async () => {
    // No content script could load; speak without on-page feedback.
    try {
      await speak({ text: chosen.result.text, origin: originOf(tab.url), title: tab.title });
    } catch (error) {
      await flagProblem(error.message);
    }
  });
}

async function startPicker(tab) {
  if (!tab?.id) return;
  if (!(await allowedOn(tab.url))) {
    await flagProblem('Sovirae is turned off for this site.');
    return;
  }
  try {
    await inject(tab.id);
    await chrome.tabs.sendMessage(tab.id, { type: 'sovirae:arm' }, { frameId: 0 });
  } catch (error) {
    await flagProblem(
      unsupportedPage(error) ? "Sovirae can't pick text on this page. Copy the text instead." : error.message,
    );
  }
}

// ---- Messages from content scripts and the popup -------------------------------

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  (async () => {
    switch (message?.type) {
      case 'sovirae:settings': {
        const s = await getSettings();
        const origin = originOf(sender.tab?.url || sender.url || '');
        sendResponse({ ...s, activeHere: s.enabled && (!origin || !s.disabledOrigins.includes(origin)) });
        break;
      }
      case 'sovirae:speak': {
        const tabId = sender.tab?.id;
        const frameId = sender.frameId;
        try {
          const { reply, voice } = await speak(message, () => {
            if (tabId != null) {
              toast(tabId, frameId, 'Allow Chrome in the Sovirae window to start reading.', 'wait');
            }
          });
          sendResponse({ ok: true, sessionId: reply.sessionId, voice });
        } catch (error) {
          const code = error.code || classify(error.message);
          sendResponse({ ok: false, code, message: error.message, detail: lastError });
        }
        break;
      }
      case 'sovirae:status': {
        if (message.probe) {
          try {
            await ready({});
          } catch {
            // Reported through the snapshot.
          }
        }
        sendResponse(statusSnapshot());
        break;
      }
      case 'sovirae:control': {
        try {
          const ack = await ready({});
          if (ack.pairing !== 'paired') throw new Error('Allow this browser in Sovirae first.');
          connect().postMessage({
            t: 'control',
            v: 1,
            sessionId: lastState?.sessionId || undefined,
            action: message.action,
            deltaMs: message.deltaMs,
            rate: message.rate,
          });
          sendResponse({ ok: true });
        } catch (error) {
          sendResponse({ ok: false, message: error.message });
        }
        break;
      }
      case 'sovirae:picker-armed': {
        // One picker per tab: disarm the other frames.
        if (sender.tab?.id != null) {
          chrome.tabs.sendMessage(sender.tab.id, { type: 'sovirae:disarm', except: message.token }).catch(() => {});
        }
        sendResponse({ ok: true });
        break;
      }
      case 'sovirae:popup-read-selection': {
        await readSelection(await chrome.tabs.get(message.tabId));
        sendResponse({ ok: true });
        break;
      }
      case 'sovirae:popup-pick': {
        await startPicker(await chrome.tabs.get(message.tabId));
        sendResponse({ ok: true });
        break;
      }
      case 'sovirae:clear-problem': {
        lastError = null;
        setBadge(lastState);
        sendResponse({ ok: true });
        break;
      }
      default:
        sendResponse({ ok: false, message: 'Unknown request.' });
    }
  })();
  return true;
});

// ---- Menus and keyboard commands --------------------------------------------------

chrome.runtime.onInstalled.addListener(async () => {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({ id: 'sovirae-read', title: 'Read with Sovirae', contexts: ['selection'] });
    chrome.contextMenus.create({ id: 'sovirae-pick', title: 'Pick text to read with Sovirae', contexts: ['page'] });
  });
  await dropLegacyRegistration();
  await injectOpenTabs();
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === 'sovirae-read') readSelection(tab, info.frameId);
  if (info.menuItemId === 'sovirae-pick') startPicker(tab);
});

chrome.commands.onCommand.addListener(async (command, tab) => {
  tab = tab || (await chrome.tabs.query({ active: true, currentWindow: true }))[0];
  if (command === 'read-selection') readSelection(tab);
  if (command === 'pick-text') startPicker(tab);
});
