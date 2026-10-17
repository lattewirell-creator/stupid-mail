# The Stupid Mail 2.2

A cream-and-terracotta Mac mail app with Quiet Inbox, Follow-up Radar, and the animated AI Buddy from The Stupid Browser.

[Download the latest Mac release](https://github.com/lattewirell-creator/stupid-mail/releases/latest) · [Account setup](docs/accounts.md)

## Open

Unzip the Mac download, quit any older version, and drag The Stupid Mail.app into Applications. Requires Apple silicon and macOS 13 or later. This is an ad-hoc-signed local build, not a notarized App Store release.

## Connect your mail

Choose **Connect mail** and select:

| Provider | Connection method |
| --- | --- |
| iCloud | Primary iCloud address and Apple app-specific password |
| Gmail / Workspace | Full address and Google app password; requires 2-Step Verification and account policy support |
| Outlook / Hotmail / Microsoft 365 | Microsoft browser sign-in; this build requires your own Microsoft application client ID |

Follow the [setup guide](docs/accounts.md), including Outlook app-registration steps. One account is active at a time. Connecting another replaces its saved sign-in; drafts and sorting remain separate by address. Normal account passwords should never be entered in the app.

## A calmer inbox

- **Quiet Inbox:** suggested Needs a reply / Read later / Done groups and five-message sessions. Grouping uses local rules, not AI.
- **Follow-up Radar:** assign a date, filter due and upcoming reminders, and check possible replies. Matching uses thread references or sender/subject/date; review suggestions before following up.
- **Snooze and undo:** rest a message until tomorrow at 9, wake it early, undo the latest local change, and restore hidden or put-away messages.
- **Buddy:** a customizable little 3D robot that can summarize a selected email or help draft a reply. Its answers never send mail automatically.
- **Keyboard shortcuts:** ⌘N to compose, ⌘F or `/` to search, and ⌘R to refresh. Drafts save while writing.

## AI setup

Open Buddy’s gear icon. Choose Ollama, OpenAI, or Claude and enter a model identifier. Cloud APIs require your own API key and may bill for usage. Browser credentials are not imported. Ollama needs a locally installed model and the service running at `http://127.0.0.1:11434`.

Questions go to your selected provider. Email content is included only when **Include this email** is enabled. Each question stands alone. Review suggested replies before sending.

## Storage and limits

Passwords and AI keys are in macOS Keychain. Microsoft access and refresh tokens also stay in Keychain. Tokens and passwords never cross into the web interface. Provider endpoints are fixed; HTTPS/TLS certificates are validated.

Drafts, cached mail, and local decisions are stored per address in private Application Support files, with atomic writes. The cache is not additionally encrypted by the app. Disconnect removes saved credentials, while local drafts and mail remain for reconnecting. Disconnect does not revoke Microsoft consent.

The app reads the latest 40 inbox messages and displays text without external images or tracking scripts. Attachments and other server folders are not supported. IMAP skips bodies over 1 MB and limits full body downloads to 10 MB per refresh. Outlook bodies over 1 MB show a placeholder. Sorting, stars, read overrides, and reminders stay local. Reminders and snooze update while the app is open, with no background notifications after closing.

Sending requires native confirmation. Provider acceptance is not a delivery guarantee. If confirmation fails, check the provider’s Sent folder before retrying. **Sent here** is local, not a server-folder sync. Outlook requests a sent copy; SMTP sending does not explicitly upload one to iCloud/Gmail Sent.

## Build and test

Install Apple Command Line Tools, Python 3, and Node.js. No package install is needed.

```sh
python3 scripts/test.py
python3 scripts/build-mac.py
```

The build creates `release-2.2/The Stupid Mail.app` and its ZIP. Sources are in `macos/` and `dist/`.

Tests cover MIME/IMAP framing and injection resistance, provider migration, OAuth token parsing, safe Outlook message conversion, frontend draft handling, snooze/undo, and Buddy context isolation. An optional localhost SMTP transport fixture is available in `tests/smtp_transport.py`.

Live account authentication, Microsoft consent, real-provider AI responses, and visual UI behavior still need testing with the owner’s setup. Fixture tests do not establish live connectivity.

## Version 2.2

- Gmail connection through Google app passwords.
- Outlook connection through Microsoft device sign-in and Graph, with refresh-token renewal and cancellation.
- Provider picker, provider-aware status and send copy, clearer errors, and account-switch protections.
- Compose, search, and refresh shortcuts; corrected About version.
- Setup guide, font license notices, and repeatable test runner.

## Credits

The ring styling and Buddy character originate in the owner’s The Stupid Browser project. Three.js and helper modules include their [MIT license](dist/vendor/THREE-LICENSE). Inter and Newsreader include their [respective](dist/fonts/Inter-OFL.txt) [SIL Open Font Licenses](dist/fonts/Newsreader-OFL.txt). No license for the original app source is granted by these third-party notices.
