# Slack and Teams

Use the **Workspaces** button (▦) on the left rail. Each provider has separate chat and email-sharing controls.

## Read and use your chats

Choose **Open chats** for Slack or Teams. The provider’s website opens in a separate window belonging to The Stupid Mail. Sign in directly with Slack or Microsoft. Mail does not receive your password or read chat content through its mail bridge.

The workspace windows have no native mail/AI message handlers. Sign-in sessions use the app’s persistent WebKit website storage. Closing a chat window keeps that session; use the provider’s Sign out control to end it. Removing an email-sharing connection does not sign out of chats.

Some workplace SSO policies or site features do not support embedded WebKit. Use **Open in browser if sign-in is unavailable here** in the chat window. This opens your normal browser; its login session is separate. Calls, notifications, uploads, SSO popups, and embedded chat compatibility have not been verified with a live workspace. This is a web-client window, not a synced Slack/Teams inbox.

## Share an email to Slack

1. Create or select a Slack app for your workspace and enable **Incoming Webhooks**.
2. Choose **Add New Webhook to Workspace** and approve the specific channel.
3. In Mail → Workspaces → Slack → **Set up email sharing**, enter a recognizable destination label and the webhook URL (`https://hooks.slack.com/services/...`).
4. Open an email and choose **Share to workspace**. Select Slack, review/edit the excerpt, and choose **Review share…**.
5. Check the destination label and complete preview in the native confirmation, then choose **Share now**.

The webhook fixes the actual channel; your label is a reminder, not independent proof of its destination. Mail sends plain-text Slack blocks and disables link/media unfurling. Mention-like email text does not become a Slack mention. Workspace administrators may restrict incoming webhooks.

[Official Slack setup](https://api.slack.com/messaging/webhooks)

## Share an email to Teams

1. Create a Teams **Workflow** using **When a Teams webhook request is received** and a post-card action targeting your chosen channel or chat. Use the incoming-webhook template with Adaptive Card support.
2. This release supports signed webhook URLs that accept an unauthenticated POST. Tenant-authenticated/OAuth-only workflows are not supported. Do not weaken a workplace-required policy; ask your administrator for an approved integration.
3. Copy the Workflow callback URL and save it in Mail → Workspaces → Teams → **Set up email sharing**, along with a recognizable destination label.
4. Open an email, choose **Share to workspace**, select Teams, edit the excerpt, and review the final confirmation.

Supported callback hosts are Microsoft’s `*.logic.azure.com` and `*.environment.api.powerplatform.com`, with a workflow path and signature. Legacy Office connector URLs are not supported. The app sends an Adaptive Card message envelope. A successful HTTP response means the workflow accepted the request; downstream workflow actions may still fail. Check Teams or the workflow’s run history for delivery.

[Microsoft workflow setup](https://support.microsoft.com/en-us/workflows/send-messages-in-teams-using-incoming-webhooks)

## What gets shared

Only the text you review is sent: initially an excerpt containing the sender, subject, and message body, limited to 2,500 characters. Attachments are excluded. Edit or remove any private text before confirming. The share is visible to people with access to the destination channel/chat. No automatic forwarding rules run.

Webhook URLs are secrets and stay in macOS Keychain. They are not exposed to the frontend or committed to GitHub. **Remove connection** deletes the local webhook; revoke it in Slack/Teams if you also want to disable it at the provider.

If sharing is not confirmed, check the destination before retrying to avoid duplicates. Automated tests validate allowed endpoints and request format; no real workspace messages were sent during development.
