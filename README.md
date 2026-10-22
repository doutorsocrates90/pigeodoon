<p align="center"><img src="src/logo.svg" width="110" alt="Pigeodoon logo"></p>

# Pigeodoon

**A lightweight desktop (and soon mobile) client for the chat of any Odoo server.**

Pigeodoon opens only **Discuss**, Odoo's chat, in its own small window, with real native notifications,
a menu-bar / tray icon and an unread badge, so you can keep the chat open all day without keeping a
browser tab and the rest of Odoo in the way.

* Enter your Odoo server address once, then sign in on **your server's own login page** (passwords,
  two-factor authentication, Google/Microsoft/SAML single sign-on: all work as usual).
* **Native notifications in real time**, settings like a messaging app: on/off, message preview,
  sound, unread badge, quiet hours.
* Stays in the **menu bar / system tray** when you close the window, so notifications keep coming.
* **Small**: about 6 MB installed (the app uses the web view that is already part of your system).
* Works with **Odoo 17, 18 and 19**, Community and Enterprise, Odoo Online and self-hosted.
* Free software, **GPL-3.0-or-later**.

> Pigeodoon is an independent project. It is not made by, affiliated with or endorsed by Odoo S.A.
> "Odoo" is a trademark of Odoo S.A.

## Status

| Platform | Status |
|---|---|
| macOS (Apple Silicon, Intel) | working, tested on Apple Silicon (version 0.1) |
| Windows (.msi, .exe) | built automatically by GitHub Actions, **not tested yet** |
| Linux (AppImage, .deb, .rpm) | built automatically by GitHub Actions, **not tested yet** |
| Android (.apk) | built automatically by GitHub Actions, **not tested yet**; no background notifications yet |
| iPhone / iPad | not available as a download; build from source (see "Mobile" below) |

## Install

Download the file for your system from the **[Releases](../../releases)** page.

### macOS

1. Open the `.dmg` and drag **Pigeodoon** to **Applications**.
2. The app is not signed with a paid Apple developer certificate yet, so macOS shows *"Pigeodoon can't be
   opened"* the first time. Open **System Settings → Privacy & Security**, scroll down and click
   **Open Anyway**. (Or in Terminal: `xattr -dr com.apple.quarantine /Applications/Pigeodoon.app`.)
3. Allow notifications when macOS asks (or later in **System Settings → Notifications → Pigeodoon**).

### Windows

Run the `.msi` (or the `-setup.exe`). If SmartScreen says *"Windows protected your PC"*, click **More
info → Run anyway** (the installer is not signed with a paid certificate yet).

### Linux

* **AppImage**: `chmod +x Pigeodoon_*.AppImage` and run it.
* **Debian / Ubuntu**: `sudo apt install ./pigeodoon_*.deb`
* **Fedora / openSUSE**: `sudo dnf install ./pigeodoon-*.rpm`

You need a system tray (on GNOME, the *AppIndicator* extension) to keep Pigeodoon running in the
background.

### Android

Download `Pigeodoon_*_android.apk` on your phone and open it. Android asks you to allow installing apps
from this source (your browser or file manager): allow it for this one install. Allow notifications
when asked. The phone has no menu bar: use the small gear button on the chat to open the settings,
change server or sign out. Use an `https://` server address (Android blocks plain `http://` in
release apps).

Version 0.1 on Android only receives messages while the app is open or recently used. A background
service for real-time notifications is planned.

## Use

1. Start Pigeodoon and type your server address (`mycompany.odoo.com`, or `https://odoo.example.org`).
2. Pigeodoon checks that it is an Odoo server, then shows the server's login page. Sign in.
3. Chat. Close the window to keep Pigeodoon in the menu bar / tray; click its icon to bring it back.
   **Quit** from the menu to stop receiving notifications.

Menu: **Pigeodoon → Settings** (`Cmd/Ctrl + ,`), **Change server…**, **Sign out**; **View → Back to
Discuss** (`Cmd/Ctrl + D`).

### How notifications work

Odoo's own chat already decides *when* to notify you (a new message in a conversation you are not
looking at, respecting each channel's "mute" and "mentions only" settings: change those from the
channel's bell icon inside Discuss). Pigeodoon turns those into **native operating system
notifications**, counts them on the app icon, and applies your own rules (quiet hours, no preview).

Because the chat keeps its live connection while Pigeodoon runs, messages arrive in real time on
desktop, also when the window is hidden. **If you quit Pigeodoon, or your computer sleeps, nothing can
arrive**: Odoo Community has no push service that could wake an app up.

### Limits of version 0.1

* Voice and video calls are not supported yet.
* The unread badge counts notifications received while the window was not in focus; it resets when you
  look at the window.
* Clicking a notification brings Pigeodoon forward; it does not jump to that exact conversation yet.
* Mobile: Android can keep a live connection with a foreground service (planned). **iOS does not allow
  apps to stay connected in the background**; real push on iPhone needs a push server and an Odoo
  add-on, which are outside version 0.1.

## Build from source

Requirements: [Node.js](https://nodejs.org) 20+, [Rust](https://rustup.rs) (stable), and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) for your system.

```bash
git clone https://github.com/<your-user>/pigeodoon.git
cd pigeodoon
npm install
npm run tauri dev        # run in development
npm run tauri build      # build installers into src-tauri/target/release/bundle/
cd src-tauri && cargo test    # unit tests
```

### Mobile (experimental)

```bash
npm run tauri android init && npm run tauri android build   # needs Android Studio + NDK
npm run tauri ios init && npm run tauri ios build           # needs a Mac with Xcode
```

An iPhone build can only be installed through Xcode, TestFlight or the App Store with an Apple
developer account, so no ready-made iOS binary is attached to the releases.

## Privacy and security

* Pigeodoon talks **only to the server you typed**. No analytics, no tracking, no other servers.
* Your password is typed on **your Odoo server's own page**; Pigeodoon never sees it.
* Pages from the server may use only three app functions (show a notification, clear the badge, open
  a link in your browser). Links to other sites open in your normal browser, never inside Pigeodoon.
* Settings are stored in a small file in your user configuration folder; the session cookie is kept
  by the system web view.

## How it is built

[Tauri 2](https://tauri.app) (Rust + the system web view) with a tiny plain-JavaScript front end.
See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE). Contributions are welcome under the same license.
