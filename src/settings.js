const { invoke } = window.__TAURI__.core;
let state = {};
const saved = document.getElementById('saved');

function render() {
  document.getElementById('server').textContent = state.server_url ? 'Connected to ' + state.server_url : 'Not connected';
  document.querySelectorAll('[data-key]').forEach((el) => {
    const value = state[el.dataset.key];
    if (el.type === 'checkbox') el.checked = !!value; else el.value = value || '';
  });
  document.querySelectorAll('[data-needs]').forEach((row) => {
    row.classList.toggle('disabled', !state[row.dataset.needs]);
  });
}

async function save() {
  try {
    await invoke('save_settings', { newSettings: state });
    saved.textContent = 'Saved';
    setTimeout(() => (saved.textContent = ''), 1200);
  } catch (error) {
    saved.textContent = String(error);
  }
}

document.querySelectorAll('[data-key]').forEach((el) => {
  el.addEventListener('change', () => {
    state[el.dataset.key] = el.type === 'checkbox' ? el.checked : el.value;
    render();
    save();
  });
});
if (/Android|iPhone|iPad/i.test(navigator.userAgent)) {
  const back = document.getElementById('back');
  back.style.display = 'block';
  back.addEventListener('click', () => invoke('back_to_chat'));
}
document.getElementById('signout').addEventListener('click', () => invoke('sign_out'));
document.getElementById('change').addEventListener('click', () => invoke('disconnect'));

invoke('get_settings').then((s) => { state = s; render(); });
