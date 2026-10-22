# Architecture

```
src/                 plain HTML/CSS/JS shown for Pigeodoon's own screens (connect, settings)
src-tauri/src/
  lib.rs             app: window, menus, tray, commands, navigation rules, downloads
  odoo.rs            checks that an address is an Odoo server and finds its version
  settings.rs        user settings (JSON file) and the quiet-hours rule
  inject.js          script injected in the Odoo pages (see below)
src-tauri/capabilities/
  local.json         what Pigeodoon's own pages may call
  remote.json        what the Odoo server's pages may call (3 commands only)
```

## The idea

Odoo's web client already is a good chat client. Pigeodoon does not re-implement it: it loads
`<server>/odoo/discuss` (or `/web#action=mail.action_discuss` before Odoo 18) in a native window and
adds what a browser tab cannot do. That keeps it small, and it works with any Odoo version, with every
login method and with each server's own customisations.

## What `inject.js` does in the Odoo page

1. **Notifications.** Odoo's `mail.out_of_focus` service calls `new Notification(title, {body})` when
   a message arrives and the window is not focused (and the channel is not muted). Pigeodoon replaces
   `window.Notification` with a shim that reports permission "granted" and calls the Rust command
   `show_notification`. Rust applies the user's rules (on/off, preview, quiet hours), shows the native
   notification and updates the badge.
2. **Discuss only.** CSS hides the app switcher, menus and other systray items (the user menu stays).
3. **Links.** `target=_blank` links and `window.open` stay in the app for the same server and open
   in the system browser for anything else.
4. **Sound.** `HTMLMediaElement.play` is silenced when the user turned the sound off.

## Navigation and login

`on_navigation` allows the connect/settings pages and the chosen server. Anything else opens in the
system browser, except while the user is logging in (paths such as `/web/login`, `/auth_oauth/...`):
then the main window may go through an identity provider (Google, Microsoft, SAML) and come back. After
login, Odoo's home page (`/odoo`) is redirected to Discuss.

## Security model

The remote capability grants only `show_notification`, `clear_badge` and `open_external`. Text from
notifications is truncated; `open_external` accepts only http, https and mailto. Settings can never
change the server (only `connect` can, and it re-validates the address).

## Real time

Desktop: the Odoo page keeps its own live connection while the app runs (window hidden or not).
Android: planned foreground service. iOS: not possible without a push relay.
