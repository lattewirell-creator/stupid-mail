# Connect a mail account

Use **Connect mail** (or **Change account**) and choose a provider. One account is active at a time. Connecting another replaces the saved credentials; local drafts, reminders, and sorting remain separate by email address. A failed connection leaves the current account connected.

## iCloud

Use your primary `@icloud.com`, `@me.com`, or `@mac.com` address and an [Apple app-specific password](https://account.apple.com/). Apple requires two-factor authentication. Never enter your main Apple Account password.

## Gmail / Google Workspace

1. Enable 2-Step Verification on your Google account.
2. Create a [Google app password](https://myaccount.google.com/apppasswords) for The Stupid Mail.
3. Choose Gmail, enter your full email address, and paste the 16-letter app password. Spaces are removed automatically.

This release uses app passwords, not Google OAuth sign-in. App passwords may be unavailable with Advanced Protection, security-key-only two-step verification, or an organization’s policy. If unavailable, this release cannot connect that account; do not use your normal Google password or weaken your security settings. Changing your Google password can revoke app passwords; reconnect with a newly generated one.

Servers are fixed to `imap.gmail.com:993` (TLS) and `smtp.gmail.com:587` (STARTTLS). Workspace accounts require the organization to allow this access.

Google references: [Connect a mail client](https://support.google.com/mail/answer/7126229), [App passwords](https://support.google.com/accounts/answer/185833).

## Outlook / Hotmail / Microsoft 365

Outlook uses Microsoft OAuth and Microsoft Graph. **This build does not ship with a registered Microsoft application ID.** A developer or administrator must register The Stupid Mail once; the app then accepts its public client ID. A client ID is a public identifier, not a password.

1. Open [Microsoft Entra app registrations](https://entra.microsoft.com/) and register an application named The Stupid Mail.
2. Choose accounts in any organizational directory **and personal Microsoft accounts**.
3. In Authentication → Advanced settings, enable **Allow public client flows**. This release uses device-code sign-in, so no client secret is needed. Do not create or paste one.
4. Under API permissions, add Microsoft Graph **delegated** permissions: `User.Read`, `Mail.Read`, and `Mail.Send`. The app also requests `offline_access` to renew its sign-in. Do not choose application permissions.
5. Copy the Application (client) ID from Overview. Choose Outlook in The Stupid Mail and enter it.
6. The app opens Microsoft’s device sign-in page. Enter the displayed code, check that you are approving your The Stupid Mail registration, and consent. Finish within two minutes, or reconnect to start again.

Your Microsoft password stays on Microsoft’s website. Access and refresh tokens are saved in macOS Keychain. Work/school administrators may require consent or block device-code sign-in. No password fallback is provided. If a token is revoked or your administrator changes policy, reconnect.

Disconnect removes the local saved credentials; it does not revoke the app’s Microsoft consent. To revoke consent, use your Microsoft account’s application permissions settings.

Microsoft references: [Register an app](https://learn.microsoft.com/en-us/entra/identity-platform/quickstart-register-app), [Device authorization](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-device-code), [Read messages](https://learn.microsoft.com/en-us/graph/api/user-list-messages?view=graph-rest-1.0), [Send mail](https://learn.microsoft.com/en-us/graph/api/user-sendmail?view=graph-rest-1.0).

## Current limits

- Latest 40 inbox messages; one active account; no unified inbox or account switcher with multiple saved credentials.
- Plain-text reading, no attachment downloading or sending. Large messages may require the provider’s own app.
- Sorting, unread overrides, stars, snooze, and reminders stay local; server folders are not edited.
- The app confirms before sending. Server acceptance does not guarantee delivery. If confirmation fails, check the provider’s Sent folder before retrying.
- Outlook requests a sent copy through Graph; the app’s Sent here section remains local. iCloud/Gmail sending uses SMTP and does not explicitly upload a separate Sent copy.
- Provider connectivity is implemented but must be verified with your own account. Automated fixtures do not prove successful live authentication.
