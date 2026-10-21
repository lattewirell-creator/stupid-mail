# The Stupid Mail 2.7

A cream-and-terracotta mail app for Mac, Windows and Android with Quiet Inbox, Follow-up Radar, and the animated AI Buddy from The Stupid Browser.

[Website](https://stupidmail.vercel.app) · [Download for Mac, Windows or Android](https://github.com/lattewirell-creator/stupid-mail/releases/latest) · [Account setup](docs/accounts.md)

## Open

**Mac:** unzip the download, quit any older version, and drag The Stupid Mail.app into Applications. One app for Apple chip and Intel Macs, macOS 13 or later. This is an ad-hoc-signed build, not a notarized App Store release.

**Windows:** run `StupidMail-<version>-windows-setup.exe`. Windows 10 or 11; it installs for your user only (no administrator prompt) and uses Windows' built-in Edge WebView2. Windows SmartScreen may warn because the installer isn't code-signed: choose **More info → Run anyway**. See [Windows notes](docs/windows.md).

**Android:** download `StupidMail-<version>-android.apk` on your phone and open it. Android 8 or later. Your phone asks once to allow installs from your browser; Play Protect may ask you to confirm because the app isn't from the Play Store. Passwords and keys are encrypted with a key in the phone's secure hardware, and nothing is included in phone backups. Proton Mail isn't offered on Android because Proton Mail Bridge only runs on computers. Calendar events open in your phone's calendar app for you to save. The app checks GitHub once a day and tells you when a newer APK is out.

## Connect your mail

Choose **Connect mail** and select:

| Provider | Connection method |
| --- | --- |
| iCloud | Primary iCloud address and Apple app-specific password |
| Gmail / Workspace | Full address and Google app password; requires 2-Step Verification and account policy support |
| Outlook / Hotmail / Microsoft 365 | Microsoft browser sign-in; this build requires your own Microsoft application client ID |
| Yahoo / AOL | Full address and a 16-letter app password from the account's security page |
| Fastmail | Full address and an app password with IMAP/SMTP access |
| Zoho Mail (zoho.com) | Full address and an app-specific password; turn on IMAP in Zoho Mail |
| GMX | Full address; turn on IMAP in GMX settings (app password if two-factor is on) |
| Proton Mail | Through Proton Mail Bridge (paid Proton plans): set Bridge's IMAP and SMTP to SSL and use the Bridge password. Mac and Windows only |
| Any other service | Choose **Other mail service** and enter its IMAP (SSL/TLS) and SMTP (SSL/TLS or STARTTLS) servers |

Follow the [setup guide](docs/accounts.md), including Outlook app-registration steps. One account is active at a time. Connecting another replaces its saved sign-in; drafts and sorting remain separate by address. Normal account passwords should never be entered in the app.

## Automatic app updates

Install 2.5 once to enable signed in-app updates. Future packaged releases download automatically, with hourly checks while open and installation on quit. Use **The Stupid Mail → Check for Updates…** to check immediately or toggle **Automatic Updates**. Uploading source code alone does not publish an update. [Update setup and release steps](docs/updates.md).

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

Passwords and AI keys are in macOS Keychain. Microsoft access and refresh tokens also stay in Keychain. Tokens and passwords never cross into the web interface. Provider endpoints are fixed for every listed provider; with **Other mail service** the app connects only to the servers you enter, always encrypted. TLS certificates are validated.

Drafts, cached mail, and local decisions are stored per address in private Application Support files, with atomic writes. The cache is not additionally encrypted by the app. Disconnect removes saved credentials, while local drafts and mail remain for reconnecting. Disconnect does not revoke Microsoft consent.

The app reads the latest 40 inbox messages and displays text without external images or tracking scripts. Attachments and other server folders are not supported. IMAP skips bodies over 1 MB and limits full body downloads to 10 MB per refresh. Outlook bodies over 1 MB show a placeholder. Sorting, stars, read overrides, and reminders stay local. Reminders and snooze update while the app is open, with no background notifications after closing.

Sending requires native confirmation. Provider acceptance is not a delivery guarantee. If confirmation fails, check the provider’s Sent folder before retrying. **Sent here** is local, not a server-folder sync. Outlook requests a sent copy; SMTP sending does not explicitly upload one to iCloud/Gmail Sent.

## Build and test

Install Apple Command Line Tools, Python 3, and Node.js. The build downloads a checksum-verified Sparkle framework for native updates.

```sh
python3 scripts/test.py
python3 scripts/build-mac.py
```

The build creates `release-<version>/app.noindex/The Stupid Mail.app` (".noindex" keeps Launchpad from listing it) and its ZIP. Sources are in `macos/` and `dist/`.

Tests cover MIME/IMAP framing and injection resistance, provider migration, OAuth token parsing, safe Outlook message conversion, frontend draft handling, snooze/undo, and Buddy context isolation. An optional localhost SMTP transport fixture is available in `tests/smtp_transport.py`.

Live account authentication, Microsoft consent, real-provider AI responses, and visual UI behavior still need testing with the owner’s setup. Fixture tests do not establish live connectivity.

## Version 2.4

Buddy uses the unchanged 3D model/animation source from The Stupid Browser (`9234994`), at its normal 116 × 128 size, with the browser’s full appearance catalog: seven shapes, ten body/outfit colours plus custom colours, six eye glows plus custom colours, five eye and five mouth styles, nine hats, six glasses styles, backpack/scarf toggles, name, and a locally cropped face picture. The Hiker is the normal default. Customize has a spin-and-poke preview and **Reset to browser Buddy**.

This version starts with the normal browser appearance instead of restoring the older Mail robot look. The old appearance key is retained as a backup. New appearance choices save separately. Mail-specific chat context and permission handling remain separate from the browser; browser credentials and browsing history are not imported. Reduced-motion mode still uses the static fallback.

## Version 2.3

- Browser-inspired modern frame, compact rail, rounded mailbox panel, pill search, and serif heading, adapted from The Stupid Browser source at `9234994`.
- Updated browser Buddy character: pick up and toss, bounce and landing reactions, return-home control, character and hat choices, and selected-email-text questions prepared for explicit review before sending.
- Optional locally synthesized ambient music, with play/pause and volume. No autoplay or external music service. Volume is remembered; playback is not.
- Slack and Teams web-client windows plus email-excerpt sharing through configured incoming webhooks/Workflows. See [workspace setup](docs/workspaces.md).
- Calendar view and on-device meeting suggestions from loaded email or selected/pasted chat text. Connect calendars already synced in macOS, review suggestions, and confirm before adding events. See [calendar setup](docs/calendar.md).
- Customize theme, accent, frame, density, corners, and sidebar; add/remove/reorder checklist, calendar, music, clock, and Action Desk widgets. Checklist tasks survive widget removal and appearance resets.
- **Action Desk:** turn an email into a local task with an optional due date and a link back to its source. Refresh preserves emails with linked tasks beyond the latest 40 messages. No background task notifications.
- Reduced-motion users get a static Buddy and no throw animation.

The visual browser preview was blocked by an administrative browser-policy check. Automated interaction checks and the signed Mac build pass; visual layout, embedded workspace sign-in, real webhook delivery, and calendar permission/saving need live testing.

## Version 2.2

- Gmail connection through Google app passwords.
- Outlook connection through Microsoft device sign-in and Graph, with refresh-token renewal and cancellation.
- Provider picker, provider-aware status and send copy, clearer errors, and account-switch protections.
- Compose, search, and refresh shortcuts; corrected About version.
- Setup guide, font license notices, and repeatable test runner.

## Credits

The ring styling and Buddy character originate in the owner’s The Stupid Browser project. Three.js and helper modules include their [MIT license](dist/vendor/THREE-LICENSE). Inter and Newsreader include their [respective](dist/fonts/Inter-OFL.txt) [SIL Open Font Licenses](dist/fonts/Newsreader-OFL.txt). No license for the original app source is granted by these third-party notices.
