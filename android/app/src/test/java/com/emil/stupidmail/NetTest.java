package com.emil.stupidmail;

import static org.junit.Assert.*;

import com.emil.stupidmail.MailCore.Credentials;
import com.emil.stupidmail.MailCore.Provider;

import org.json.JSONObject;
import org.junit.Test;

import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.time.LocalDateTime;
import java.time.ZoneId;
import java.time.ZonedDateTime;
import java.util.List;

/** The Mac app's ProviderTests, WorkspaceTests and CalendarTests (windows/src/tests.rs), against the Android port. */
public class NetTest {
    @Test public void outlookTokensAndRecords() throws Exception {
        Credentials c = Net.credentials(new JSONObject("{\"access_token\":\"fixture-token\",\"refresh_token\":\"fixture-refresh\",\"expires_in\":3600}"), "me@outlook.com", "fixture-client", null);
        assertEquals(Provider.OUTLOOK, c.provider); assertEquals("fixture-refresh", c.refreshToken);
        try { MailCore.sync(c); fail("Outlook token must never go to IMAP"); } catch (MailCore.MailFailure expected) { }
        assertTrue(c.expiresAt - System.currentTimeMillis() / 1000 > 3500);
        assertEquals(Provider.OUTLOOK, Credentials.from(c.json()).provider);
        assertEquals("fixture-refresh", Net.credentials(new JSONObject("{\"access_token\":\"new\"}"), c.email, "fixture-client", c.refreshToken).refreshToken);
        assertEquals("new-refresh", Net.credentials(new JSONObject("{\"access_token\":\"new\",\"refresh_token\":\"new-refresh\"}"), c.email, "fixture-client", c.refreshToken).refreshToken);
        try { Net.credentials(new JSONObject("{\"error\":\"authorization_declined\"}"), "", "", null); fail(); } catch (MailCore.MailFailure expected) { }
        String input = "{\"value\":[{\"id\":\"immutable\",\"from\":{\"emailAddress\":{\"name\":\"Jamie\",\"address\":\"jamie@example.com\"}},\"subject\":\"A message\","
            + "\"body\":{\"contentType\":\"html\",\"content\":\"<p>Hello &amp; welcome</p><script>bad()</script>\"},\"isRead\":false,\"hasAttachments\":true,\"flag\":{\"flagStatus\":\"flagged\"},"
            + "\"internetMessageId\":\"<one@example.com>\",\"internetMessageHeaders\":[{\"name\":\"In-Reply-To\",\"value\":\"<original@example.com>\"},{\"name\":\"List-Unsubscribe\",\"value\":\"<https://example.com>\"}]}]}";
        JSONObject m = Net.records(input).get(0);
        assertEquals("outlook:immutable", m.getString("id")); assertEquals("jamie@example.com", m.getString("email"));
        assertEquals("Hello & welcome", m.getString("body"));
        assertTrue(m.getBoolean("unread") && m.getBoolean("star") && m.getBoolean("newsletter") && m.getInt("attachmentCount") > 0);
        assertEquals("<original@example.com>", m.getString("inReplyTo"));
        try { Net.records("{\"value\":[{}]}"); fail(); } catch (MailCore.MailFailure expected) { }
    }

    @Test public void workspaceEndpointsAndPayloads() throws Exception {
        assertTrue(Net.webhookValid("https://hooks.slack.com/services/T/B/fixture", "slack"));
        for (String url : new String[]{"http://hooks.slack.com/services/T/B/fixture", "https://hooks.slack.com.evil.test/services/T/B/fixture", "https://user@hooks.slack.com/services/T/B/fixture", "https://hooks.slack.com:444/services/T/B/fixture", "https://example.com/services/T/B/fixture"})
            assertFalse(url, Net.webhookValid(url, "slack"));
        assertTrue(Net.webhookValid("https://prod-1.westeurope.logic.azure.com/workflows/fixture/triggers/manual/paths/invoke?sig=fixture", "teams"));
        assertTrue(Net.webhookValid("https://fixture.environment.api.powerplatform.com/powerautomate/automations/direct/workflows/fixture/triggers/manual/paths/invoke?sig=fixture", "teams"));
        assertFalse(Net.webhookValid("https://evil.test/workflows/fixture?sig=fixture", "teams"));
        assertFalse(Net.webhookValid("https://fixture.environment.api.powerplatform.com/workflows/fixture", "teams"));
        String text = "From: fixture@example.com\n<!channel> <https://example.com> & hello";
        JSONObject slack = new JSONObject(new String(Net.webhookPayload(text, "slack"), StandardCharsets.UTF_8));
        assertEquals("plain_text", slack.getJSONArray("blocks").getJSONObject(0).getJSONObject("text").getString("type"));
        assertEquals(text, slack.getJSONArray("blocks").getJSONObject(0).getJSONObject("text").getString("text"));
        assertFalse(slack.getBoolean("unfurl_links"));
        assertEquals("message", new JSONObject(new String(Net.webhookPayload(text, "teams"), StandardCharsets.UTF_8)).getString("type"));
        try { Net.webhookPayload("", "slack"); fail(); } catch (MailCore.MailFailure expected) { }
        try { Net.webhookPayload(new String(new char[2501]).replace('\0', 'x'), "slack"); fail(); } catch (MailCore.MailFailure expected) { }
    }

    @Test public void meetingDetection() throws Exception {
        ZoneId zone = ZoneId.systemDefault();
        ZonedDateTime now = Instant.parse("2030-01-01T00:00:00Z").atZone(zone);
        List<JSONObject> found = Meetings.suggestions("Project meeting June 4, 2030 at 10:00 AM.", "Project review", now);
        assertFalse(found.isEmpty());
        assertEquals("Project review", found.get(0).getString("title"));
        assertTrue(found.get(0).getString("phrase").contains("2030"));
        Instant s = Meetings.parseIso(found.get(0).getString("start")), e = Meetings.parseIso(found.get(0).getString("end"));
        assertTrue(e.isAfter(s));
        assertEquals("10:00", LocalDateTime.ofInstant(s, zone).toLocalTime().toString());
        assertTrue(Meetings.suggestions("June 4, 2020 at 10:00 AM", "Old meeting", now).isEmpty());
        assertTrue(Meetings.suggestions("Thank you for the update.", "No meeting", now).isEmpty());
        List<JSONObject> sv = Meetings.suggestions("Ses vi imorgon kl 15:30? Annars tisdag kl 9.", "Möte", now);
        assertEquals(2, sv.size());
        LocalDateTime first = LocalDateTime.ofInstant(Meetings.parseIso(sv.get(0).getString("start")), zone);
        assertEquals(LocalDateTime.ofInstant(Instant.parse("2030-01-01T00:00:00Z"), zone).toLocalDate().plusDays(1).atTime(15, 30), first);
        assertFalse(Meetings.suggestions("Can we do the 20 oktober at 2pm?", "Möte", now).isEmpty());
        assertTrue("words that aren't dates", Meetings.suggestions("The meeting room is great", "x", now).isEmpty());
    }

    @Test public void updateVersions() {
        assertTrue(MainActivity.newer("2.7", "2.6"));
        assertTrue(MainActivity.newer("2.6.1", "2.6.0"));
        assertFalse(MainActivity.newer("2.5", "2.6.0"));
        assertFalse(MainActivity.newer("2.6", "2.6.0"));
    }
}
