const { invoke } = window.__TAURI__.core;
const form = document.getElementById('form');
const input = document.getElementById('url');
const button = document.getElementById('go');
const status = document.getElementById('status');

function say(text, kind) {
  status.textContent = text || '';
  status.className = 'status' + (kind ? ' ' + kind : '');
}

form.addEventListener('submit', async (event) => {
  event.preventDefault();
  button.disabled = true;
  say('Checking the server…');
  try {
    const info = await invoke('check_server', { url: input.value });
    say('Found Odoo ' + info.version + '. Opening the login page…', 'ok');
    await invoke('connect', { url: info.url, discussPath: info.discuss_path });
  } catch (error) {
    say(String(error), 'err');
    button.disabled = false;
  }
});

invoke('get_settings').then((s) => {
  if (s.server_url) input.value = s.server_url;
});
