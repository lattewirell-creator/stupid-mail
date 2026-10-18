# Calendar and Buddy’s meeting suggestions

Choose **Calendar** on the left rail.

## Find possible meetings

- **Buddy, find meetings in mail** checks text in up to 40 currently loaded inbox messages.
- Paste a Slack/Teams message into the text box and choose **Find times in this text**.
- In a workspace chat window, highlight the relevant message and choose **Find meeting in selected chat text**. If the web client uses an inaccessible frame or does not expose the selection, copy and paste into the calendar instead.

Detection runs on this Mac with Apple’s date detector, without sending mail or chat text to an AI service. It finds date expressions, not confirmed meeting invitations: deadlines and other dates may appear. It does not continuously monitor mail or read entire Slack/Teams histories. Relative dates and missing years can be ambiguous. Check the title, date, time zone, and duration yourself. A missing duration defaults to one hour.

Choose a suggestion to fill the editable event form, or enter an event manually. Events are never created automatically. There is no meeting invitation/ICS import or attendee scheduling in this version.

## Connect your calendars

Choose **Connect Mac calendars** and grant macOS calendar access. The app shows up to 100 events in the next 30 days. It can save to writable calendars already configured in the Mac’s Calendar app, including supported iCloud, Google, and Outlook accounts.

If a calendar is missing, add its account through macOS Calendar → Add Account (or System Settings → Internet Accounts), enable calendar syncing, then return to Mail and refresh. There is no separate Google or Microsoft Calendar login inside Mail. A mail account connection does not automatically connect its calendar.

Before an event is added, Mail shows the destination calendar, title, and times in a confirmation. Shared-calendar members may see these details. No original email/chat text, attachments, or attendee invitations are included. If saving fails, check the destination calendar before retrying.

If access is denied, enable The Stupid Mail in System Settings → Privacy & Security → Calendars. Revoke it there whenever you want. Reading calendars does not grant Buddy permission to send their contents to an AI provider.

Automated checks cover date detection; calendar permission prompts, account sync, event creation, and chat selections still require live testing on your Mac. No real calendar events were created during development.
