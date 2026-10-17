# The Stupid Mail 2.1

A local Apple-silicon Mac app with Quiet Inbox, Follow-up Radar, and the animated Buddy from The Stupid Browser.

## Open

Unzip `The Stupid Mail — Mac.zip`, drag `The Stupid Mail.app` into Applications, and open it. Quit an older version before replacing it. This is a locally built, ad-hoc-signed app, not a notarized App Store release.

## iCloud

Click **Connect iCloud** inside the app. On https://account.apple.com, choose Sign-In and Security → App-Specific Passwords and generate one named “The Stupid Mail”. Enter your primary @icloud.com, @me.com, or @mac.com address and that password in the native app window. Do not use or share your main Apple Account password. Apple requires two-factor authentication for app-specific passwords.

The password stays in macOS Keychain. The inbox uses a certificate-validated TLS connection to imap.mail.me.com:993. Sending uses authenticated SMTP with mandatory STARTTLS to smtp.mail.me.com:587. Passwords are never exposed to the web interface or passed in process arguments.

The app loads the latest 40 inbox messages when opened or refreshed. It does not download messages larger than 1 MB, and limits body downloads to 10 MB per refresh. Email is displayed as text, without loading tracking images or scripts. Attachments, custom-domain sending identities, and other server folders are not supported yet. Sent here shows mail sent from this app; it is not a sync of iCloud’s Sent folder. SMTP acceptance is confirmed before a draft is marked sent. Automatic retry is intentionally disabled to avoid duplicate messages.

Drafts, stars, read state, sorting, and reminders are stored locally per account in private Application Support files. Earlier browser-storage data is migrated when available. Put away and Hidden do not modify or delete messages on iCloud. Disconnect removes the Keychain password and returns to the demo mailbox; local drafts and cached mail remain available on reconnect. The app’s local cache is not additionally encrypted by the app.

## What makes it different

- **Quiet Inbox:** a suggested Needs a reply / Read later / Done grouping and five-message sessions with a deliberate stopping point. Suggestions use local rules, not AI.
- **Follow-up Radar:** assign a date to a conversation; see overdue reminders and possible replies in the latest synced inbox. Possible replies are suggestions based on sender and thread references or matching subject/date. Review them before following up. Reminders update while the app is open; there are no background notifications while it is closed.
- **Buddy:** the same bundled 3D character as the browser, with encouragement, email summaries, and draft help. Its answer never sends an email automatically.

## AI setup

Open Buddy and select the gear icon. Choose Ollama, OpenAI, or Claude and enter the exact model identifier from your provider. Cloud APIs require their own API key and may bill for use. The browser’s existing credentials are not imported. Keys are kept in Keychain.

Ollama uses http://127.0.0.1:11434 and needs Ollama running with a locally installed model. For cloud providers, your question goes directly to the chosen provider. An email is included only when you enable **Include this email**. Each question is independent; only the selected message and current question are supplied. Review any answer before using it in a reply.

## Build and verification

`python3 scripts/build-mac.py` builds the app into `release-2.1/The Stupid Mail.app` and creates the ZIP alongside it. Requires Apple Command Line Tools. Source assets remain in dist/ and macos/.

- Swift tests cover MIME decoding, safe text rendering, header/config injection, IMAP literal framing, and outgoing email encoding.
- JavaScript tests cover grouping, follow-up matching, and preserving local decisions during sync.
- SMTP is tested against a disposable localhost STARTTLS server, never with real mail.
- An authenticated iCloud sync and real-provider Buddy response must be verified after the owner enters their credentials. These are not claimed as completed by fixture tests.

Apple setup reference: https://support.apple.com/en-us/102654
Apple server reference: https://support.apple.com/en-us/102525

## Version 2.1

- Snooze a message until tomorrow at 9, wake it early, and undo the latest local change.
- Follow-up Radar separates due, upcoming, and possible replies, with a button to open a matched reply.
- Hidden and put-away messages can be restored; read status can be changed back to unread.
- Native mailbox files remove the browser-storage quota limit. Writes are atomic and happen off the UI thread. Sending waits for a successful draft save, and quitting waits for pending saves.
- Refresh keeps the open message in sync and preserves local sorting decisions.
- Buddy supports a custom name and color, a fresh-chat button, Enter to ask, and Escape to close. Answers arriving after an account switch or chat reset are discarded.
- UI flow tests use simulated DOM/storage fixtures; they are not visual browser tests. Live iCloud and AI checks still require account setup in the app.
