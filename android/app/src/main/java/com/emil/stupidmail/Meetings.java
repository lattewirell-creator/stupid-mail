package com.emil.stupidmail;

import org.json.JSONException;
import org.json.JSONObject;

import java.time.DayOfWeek;
import java.time.Instant;
import java.time.LocalDate;
import java.time.LocalTime;
import java.time.ZoneId;
import java.time.ZonedDateTime;
import java.time.format.DateTimeFormatter;
import java.time.temporal.ChronoUnit;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** Finds possible meeting times in mail or chat text, on this phone (windows/src/calendar.rs): English and Swedish
 *  phrases such as "Tuesday at 2pm", "Oct 20 15:30", "tomorrow at 9", "imorgon kl 15", "20 oktober". */
final class Meetings {
    private Meetings() {}

    static final Map<String, Integer> MONTHS = new LinkedHashMap<>();
    static final Map<String, DayOfWeek> DAYS = new LinkedHashMap<>();
    static {
        String[][] m = {{"january", "1"}, {"jan", "1"}, {"januari", "1"}, {"february", "2"}, {"feb", "2"}, {"februari", "2"}, {"march", "3"}, {"mar", "3"}, {"mars", "3"}, {"april", "4"}, {"apr", "4"},
            {"may", "5"}, {"maj", "5"}, {"june", "6"}, {"jun", "6"}, {"juni", "6"}, {"july", "7"}, {"jul", "7"}, {"juli", "7"}, {"august", "8"}, {"aug", "8"}, {"augusti", "8"},
            {"september", "9"}, {"sep", "9"}, {"sept", "9"}, {"october", "10"}, {"oct", "10"}, {"oktober", "10"}, {"november", "11"}, {"december", "12"}};
        for (String[] e : m) MONTHS.put(e[0], Integer.parseInt(e[1]));
        Object[][] d = {{"monday", DayOfWeek.MONDAY}, {"mon", DayOfWeek.MONDAY}, {"måndag", DayOfWeek.MONDAY}, {"tuesday", DayOfWeek.TUESDAY}, {"tue", DayOfWeek.TUESDAY}, {"tisdag", DayOfWeek.TUESDAY},
            {"wednesday", DayOfWeek.WEDNESDAY}, {"wed", DayOfWeek.WEDNESDAY}, {"onsdag", DayOfWeek.WEDNESDAY}, {"thursday", DayOfWeek.THURSDAY}, {"thu", DayOfWeek.THURSDAY}, {"torsdag", DayOfWeek.THURSDAY},
            {"friday", DayOfWeek.FRIDAY}, {"fri", DayOfWeek.FRIDAY}, {"fredag", DayOfWeek.FRIDAY}, {"saturday", DayOfWeek.SATURDAY}, {"sat", DayOfWeek.SATURDAY}, {"lördag", DayOfWeek.SATURDAY},
            {"sunday", DayOfWeek.SUNDAY}, {"sun", DayOfWeek.SUNDAY}, {"söndag", DayOfWeek.SUNDAY}};
        for (Object[] e : d) DAYS.put((String) e[0], (DayOfWeek) e[1]);
    }
    private static final Pattern PATTERN;
    static {
        List<String> names = new ArrayList<>(MONTHS.keySet());
        names.sort((a, b) -> b.length() - a.length());
        String mon = String.join("|", names);
        PATTERN = Pattern.compile("(?iu)\\b(?:(\\d{4})-(\\d{2})-(\\d{2})(?:[T ]?)|(today|tonight|tomorrow|idag|ikväll|imorgon|i morgon)|(?:next\\s+|nästa\\s+|on\\s+|på\\s+)?(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tue|wed|thu|fri|sat|sun|måndag|tisdag|onsdag|torsdag|fredag|lördag|söndag)\\b,?\\s*(?:(\\d{1,2})(?:st|nd|rd|th)?\\s+(" + mon + ")\\b\\.?)?|(" + mon + ")\\b\\.?\\s+(\\d{1,2})(?:st|nd|rd|th)?\\b(?:,?\\s*(\\d{4}))?|(\\d{1,2})(?:st|nd|rd|th)?\\s+(?:of\\s+)?(" + mon + ")\\b\\.?(?:\\s+(\\d{4}))?)");
    }
    private static final Pattern TIME = Pattern.compile("(?i)^\\s*(?:,\\s*)?(at|@|kl\\.?|klockan)?\\s*([0-2]?\\d)(?:[:.]([0-5]\\d))?\\s*(am|pm|a\\.m\\.|p\\.m\\.)?\\b");

    /** "3pm", "3:30 pm", "15:00", "15.30", "kl 15", "kl. 9.30": the time and how many characters it used. */
    static Object[] timeAfter(String text) {
        Matcher m = TIME.matcher(text);
        if (!m.find()) return null;
        boolean marked = m.group(1) != null || m.group(3) != null || m.group(4) != null;
        if (!marked) return null;
        int h = Integer.parseInt(m.group(2)), min = m.group(3) == null ? 0 : Integer.parseInt(m.group(3));
        if (m.group(4) != null) { boolean pm = m.group(4).toLowerCase(Locale.ROOT).startsWith("p"); if (h == 12) h = 0; if (pm) h += 12; }
        if (h > 23) return null;
        return new Object[]{LocalTime.of(h, min), m.end()};
    }
    private static LocalDate yearFor(LocalDate today, int m, int d, Integer year) {
        try {
            if (year != null) return LocalDate.of(year, m, d);
            LocalDate thisYear = LocalDate.of(today.getYear(), m, d);
            return thisYear.isBefore(today) ? LocalDate.of(today.getYear() + 1, m, d) : thisYear;
        } catch (Exception e) { return null; }
    }
    private static Integer num(String s) { try { return s == null ? null : Integer.parseInt(s); } catch (NumberFormatException e) { return null; } }

    static List<JSONObject> suggestions(String text, String title, ZonedDateTime now) throws JSONException {
        String input = text.length() > 6000 ? text.substring(0, 6000) : text;
        LocalDate today = now.toLocalDate();
        List<JSONObject> out = new ArrayList<>(); Set<String> seen = new HashSet<>();
        Matcher c = PATTERN.matcher(input);
        DateTimeFormatter iso = DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss'Z'").withZone(ZoneId.of("UTC"));
        while (c.find()) {
            LocalDate date = null;
            if (c.group(1) != null) { try { date = LocalDate.of(num(c.group(1)), num(c.group(2)), num(c.group(3))); } catch (Exception ignored) { } }
            else if (c.group(4) != null) date = c.group(4).toLowerCase(Locale.ROOT).contains("mor") ? today.plusDays(1) : today;
            else if (c.group(5) != null && DAYS.containsKey(c.group(5).toLowerCase(Locale.ROOT))) {
                if (c.group(6) != null && c.group(7) != null) date = yearFor(today, MONTHS.get(c.group(7).toLowerCase(Locale.ROOT)), num(c.group(6)), null);
                else { DayOfWeek wd = DAYS.get(c.group(5).toLowerCase(Locale.ROOT)); date = today.plusDays(1); while (date.getDayOfWeek() != wd) date = date.plusDays(1); }
            }
            else if (c.group(8) != null) date = yearFor(today, MONTHS.get(c.group(8).toLowerCase(Locale.ROOT)), num(c.group(9)), num(c.group(10)));
            else if (c.group(11) != null) date = yearFor(today, MONTHS.get(c.group(12).toLowerCase(Locale.ROOT)), num(c.group(11)), num(c.group(13)));
            if (date == null) continue;
            Object[] t = timeAfter(input.substring(c.end()));
            LocalTime time = t == null ? LocalTime.NOON : (LocalTime) t[0];
            ZonedDateTime start = date.atTime(time).atZone(now.getZone());
            if (start.isBefore(now.minusHours(1)) || start.isAfter(now.plusDays(366))) continue;
            String s = iso.format(start.toInstant());
            if (!seen.add(s)) continue;
            String phrase = input.substring(c.start(), c.end() + (t == null ? 0 : (Integer) t[1])).trim();
            out.add(new JSONObject().put("title", title.length() > 200 ? title.substring(0, 200) : title).put("start", s)
                .put("end", iso.format(start.toInstant().plus(1, ChronoUnit.HOURS))).put("phrase", phrase));
            if (out.size() == 8) break;
        }
        return out;
    }
    static Instant parseIso(String text) { try { return Instant.parse(text); } catch (Exception e) { return null; } }
}
