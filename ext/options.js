// Options: trigger, hold time, and sites turned off (spec §12.2).

const $ = (id) => document.getElementById(id);
let isMac = false;

async function defaults() {
  isMac = (await chrome.runtime.getPlatformInfo()).os === 'mac';
  return { enabled: true, trigger: isMac ? 'alt' : 'control', holdMs: 450, disabledOrigins: [] };
}

function flashSaved() {
  const s = $('saved');
  s.classList.add('show');
  setTimeout(() => s.classList.remove('show'), 1200);
}

async function save(patch) {
  await chrome.storage.local.set(patch);
  flashSaved();
}

function siteList(target, origins, onRemove, empty) {
  const ul = $(target);
  ul.innerHTML = '';
  if (origins.length === 0) {
    ul.innerHTML = `<li class="empty">${empty}</li>`;
    return;
  }
  for (const o of origins) {
    const li = document.createElement('li');
    const name = document.createElement('span');
    name.textContent = o.replace(/\/\*$/, '');
    const btn = document.createElement('button');
    btn.className = 'btn ghost';
    btn.textContent = 'Remove';
    btn.addEventListener('click', () => onRemove(o));
    li.append(name, btn);
    ul.append(li);
  }
}

async function render() {
  const base = await defaults();
  const s = { ...base, ...(await chrome.storage.local.get(Object.keys(base))) };
  $('alt-name').textContent = isMac ? 'Long press Option' : 'Long press Alt';
  document.querySelectorAll('input[name=trigger]').forEach((r) => (r.checked = r.value === s.trigger));
  $('hold').value = s.holdMs;
  $('hold-value').textContent = `${(s.holdMs / 1000).toFixed(2)} s`;
  $('hold').disabled = s.trigger === 'shortcut';

  siteList('off', s.disabledOrigins, async (o) => {
    await save({ disabledOrigins: s.disabledOrigins.filter((x) => x !== o) });
    render();
  }, 'Sovirae is on for every site.');
}

document.querySelectorAll('input[name=trigger]').forEach((r) =>
  r.addEventListener('change', async () => {
    await save({ trigger: r.value });
    render();
  }),
);
$('hold').addEventListener('input', (e) => ($('hold-value').textContent = `${(e.target.value / 1000).toFixed(2)} s`));
$('hold').addEventListener('change', (e) => save({ holdMs: Number(e.target.value) }));
$('shortcuts').addEventListener('click', () => chrome.tabs.create({ url: 'chrome://extensions/shortcuts' }));
$('reset').addEventListener('click', async () => {
  await chrome.storage.local.set(await defaults());
  render();
  flashSaved();
});

render();
