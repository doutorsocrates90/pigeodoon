// Pigeodoon: runs in every page of the main window, before the page's own scripts.
// On the Odoo server's pages it (1) turns Odoo's browser notifications into native ones,
// (2) keeps the window to Discuss, (3) sends links to the system browser, (4) honours the sound setting.
(function () {
  if (window.__pigeodoon) return;
  var internals = window.__TAURI_INTERNALS__;
  var isLocal = location.protocol === 'tauri:' || location.hostname === 'tauri.localhost';
  if (isLocal || !internals) return;
  window.__pigeodoon = true;
  window.__pigeodoon_cfg = window.__pigeodoon_cfg || { sound: true };

  function call(cmd, args) {
    try { return internals.invoke(cmd, args || {}); } catch (e) { return Promise.resolve(); }
  }

  // 1. Native notifications. Odoo already decides when to notify (message not seen, channel not muted, window not
  //    focused) and asks the browser for it; we answer "granted" and hand the text to the operating system.
  class PigeodoonNotification extends EventTarget {
    static get permission() { return 'granted'; }
    static requestPermission(callback) {
      if (typeof callback === 'function') callback('granted');
      return Promise.resolve('granted');
    }
    constructor(title, options) {
      super();
      options = options || {};
      this.title = String(title || '');
      this.body = String(options.body || '');
      this.onclick = null;
      call('show_notification', { title: this.title, body: this.body });
    }
    close() {}
  }
  window.Notification = PigeodoonNotification;
  if (navigator.permissions && navigator.permissions.query) {
    var originalQuery = navigator.permissions.query.bind(navigator.permissions);
    navigator.permissions.query = function (descriptor) {
      if (descriptor && descriptor.name === 'notifications') {
        return Promise.resolve({ state: 'granted', onchange: null, addEventListener: function () {}, removeEventListener: function () {} });
      }
      return originalQuery(descriptor);
    };
  }

  // 2. Discuss only: hide the app switcher, the menus and the other systray items. The user menu stays
  //    (preferences, notification settings, log out).
  var css =
    '.o_main_navbar .o_navbar_apps_menu,' +
    '.o_main_navbar .o_menu_brand,' +
    '.o_main_navbar .o_menu_sections,' +
    '.o_main_navbar .o_menu_systray > :not(.o_user_menu){display:none !important;}';
  function addStyle() {
    if (document.getElementById('pigeodoon-style')) return;
    var style = document.createElement('style');
    style.id = 'pigeodoon-style';
    style.textContent = css;
    (document.head || document.documentElement).appendChild(style);
  }
  addStyle();
  document.addEventListener('DOMContentLoaded', addStyle);

  // 3. Links: same server stays in the app, everything else opens in the system browser.
  function route(href) {
    var url;
    try { url = new URL(href, location.href); } catch (e) { return; }
    if (url.origin === location.origin) { location.href = url.href; return; }
    if (/^(https?:|mailto:)$/.test(url.protocol)) call('open_external', { url: url.href });
  }
  document.addEventListener('click', function (event) {
    var anchor = event.target && event.target.closest ? event.target.closest('a[href]') : null;
    if (!anchor) return;
    var wantsNewWindow = anchor.target === '_blank' || anchor.protocol === 'mailto:';
    if (wantsNewWindow) {
      event.preventDefault();
      route(anchor.href);
    }
  }, true);
  window.open = function (href) { if (href) route(href); return null; };

  // 4. Sound setting (Odoo plays the new-message sound itself).
  var originalPlay = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function () {
    if (window.__pigeodoon_cfg && window.__pigeodoon_cfg.sound === false) return Promise.resolve();
    return originalPlay.apply(this, arguments);
  };

  // 5. Phones have no menu bar: a small button opens the settings page (server, sign out, notifications).
  if (/Android|iPhone|iPad/i.test(navigator.userAgent)) {
    document.addEventListener('DOMContentLoaded', function () {
      if (document.getElementById('pigeodoon-gear')) return;
      var gear = document.createElement('button');
      gear.id = 'pigeodoon-gear';
      gear.textContent = '\u2699';
      gear.setAttribute('aria-label', 'Pigeodoon settings');
      gear.style.cssText = 'position:fixed;left:8px;bottom:70px;z-index:2147483647;width:34px;height:34px;border-radius:17px;border:0;background:rgba(0,0,0,.45);color:#fff;font-size:18px;line-height:34px;padding:0;opacity:.7;';
      gear.addEventListener('click', function () { call('open_settings'); });
      document.body.appendChild(gear);
    });
  }

  // Seeing the window again clears the unread badge.
  window.addEventListener('focus', function () { call('clear_badge'); });
})();
