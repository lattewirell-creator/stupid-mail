# The Stupid Mail on Windows

The Windows version shows the same interface as the Mac app (the files in `dist/`, built into the program) in Windows' own web engine, Microsoft Edge WebView2. Its native side is written in Rust (`windows/`): it connects to your mail, keeps secrets, and shows the confirmation dialogs.

## What's the same
- iCloud and Gmail with app passwords, Outlook with Microsoft sign-in (your own application client ID), the latest 40 inbox messages, sending with confirmation, drafts and local sorting, Quiet Inbox, Follow-up Radar, snooze and undo, Buddy (Ollama, OpenAI or Claude), Slack and Teams windows and sharing, finding meeting times in mail and chat text.
- The same wording, checks and limits as on the Mac.

## What's different
- **Secrets:** passwords, Microsoft tokens, AI keys and webhook URLs are encrypted with Windows' per-user data protection (DPAPI) in `%APPDATA%\The Stupid Mail\secrets`, readable only by your Windows account. Like on the Mac, they never reach the web interface: dialogs are separate windows.
- **Mail data:** drafts, cached mail and local decisions are in `%APPDATA%\The Stupid Mail\Accounts` (not additionally encrypted).
- **Calendar:** Windows has no shared calendar store for regular apps like macOS's. Meeting suggestions work the same (English and Swedish dates and times). **Add event** opens the event in your calendar app (Outlook, Windows Calendar…) as an `.ics` file for you to save there; upcoming events aren't listed.
- **Updates:** at most once a day at start, and from **Help → Check for Updates…**, the app checks GitHub Releases. It downloads the new installer and runs it only if its Ed25519 signature matches the key built into the app (the same key as the Mac updates).
- Uninstalling keeps your mail data and saved sign-ins for a reinstall. Delete `%APPDATA%\The Stupid Mail` to remove them.

## Build
`cd windows && cargo test && cargo build --release` on Windows (Rust stable). `.github/workflows/windows.yml` builds, tests and packages the installer (NSIS) on GitHub's Windows machines on every push. The mail code and its tests also run on a Mac (`cargo test`), which is how most of it was developed.

## Publish a Windows update
Download the `windows-installer` artifact from the workflow run, then on the publishing Mac sign it with the update key (`python3 scripts/sign-release.py` signs every `release-<version>/*-windows-setup.exe` it finds and writes the `.sig` next to it). Upload both the installer and its `.sig` to the GitHub release.
