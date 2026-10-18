// Calendar on Windows (macos/CalendarCore.swift + CalendarBridge.swift). Finding dates in mail or chat text
// runs on this computer (English and Swedish phrases such as "Tuesday at 2pm", "Oct 20 15:30", "tomorrow at 9",
// "imorgon kl 15", "20 oktober"). Windows has no shared calendar store for regular apps like macOS's, so a
// confirmed event is handed to your calendar app (Outlook, Windows Calendar…) as an .ics file to add there.
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use regex::Regex;
use serde_json::{json, Value};

const MONTHS: [(&str, u32); 30] = [("january", 1), ("jan", 1), ("januari", 1), ("february", 2), ("feb", 2), ("februari", 2), ("march", 3), ("mar", 3), ("mars", 3),
    ("april", 4), ("apr", 4), ("may", 5), ("maj", 5), ("june", 6), ("jun", 6), ("juni", 6), ("july", 7), ("jul", 7), ("juli", 7), ("august", 8), ("aug", 8), ("augusti", 8),
    ("september", 9), ("sep", 9), ("sept", 9), ("october", 10), ("oct", 10), ("oktober", 10), ("november", 11), ("december", 12)];
const DAYS: [(&str, Weekday); 21] = [("monday", Weekday::Mon), ("mon", Weekday::Mon), ("måndag", Weekday::Mon), ("tuesday", Weekday::Tue), ("tue", Weekday::Tue), ("tisdag", Weekday::Tue),
    ("wednesday", Weekday::Wed), ("wed", Weekday::Wed), ("onsdag", Weekday::Wed), ("thursday", Weekday::Thu), ("thu", Weekday::Thu), ("torsdag", Weekday::Thu),
    ("friday", Weekday::Fri), ("fri", Weekday::Fri), ("fredag", Weekday::Fri), ("saturday", Weekday::Sat), ("lördag", Weekday::Sat), ("sunday", Weekday::Sun), ("söndag", Weekday::Sun), ("sat", Weekday::Sat), ("sun", Weekday::Sun)];

fn month(name: &str) -> Option<u32> { let n = name.to_lowercase(); MONTHS.iter().find(|(m, _)| *m == n.trim_end_matches('.')).map(|x| x.1) }
fn weekday(name: &str) -> Option<Weekday> { let n = name.to_lowercase(); DAYS.iter().find(|(d, _)| *d == n).map(|x| x.1) }

/// "3pm", "3:30 pm", "15:00", "15.30", "kl 15", "kl. 9.30"
fn time_after(text: &str) -> Option<(NaiveTime, usize)> {
    let re = Regex::new(r"(?i)^\s*(?:,\s*)?(?:at|@|kl\.?|klockan)?\s*([0-2]?\d)(?:[:.]([0-5]\d))?\s*(am|pm|a\.m\.|p\.m\.)?\b").unwrap();
    let c = re.captures(text)?;
    let has_marker = c.get(2).is_some() || c.get(3).is_some() || c[0].to_lowercase().contains("kl") || c[0].to_lowercase().contains("at") || c[0].contains('@');
    if !has_marker { return None; }
    let mut h: u32 = c[1].parse().ok()?;
    let m: u32 = c.get(2).map(|x| x.as_str().parse().unwrap_or(0)).unwrap_or(0);
    if let Some(ap) = c.get(3) { let pm = ap.as_str().to_lowercase().starts_with('p'); if h == 12 { h = 0; } if pm { h += 12; } }
    Some((NaiveTime::from_hms_opt(h, m, 0)?, c[0].len()))
}

pub fn suggestions(text: &str, title: &str, now: DateTime<Local>) -> Vec<Value> {
    let input: String = text.chars().take(6000).collect();
    let today = now.date_naive();
    // Month names are listed explicitly (longest first) so ordinary words are never taken for months.
    let mut names: Vec<&str> = MONTHS.iter().map(|m| m.0).collect();
    names.sort_by_key(|n| std::cmp::Reverse(n.len()));
    let mon = names.join("|");
    let pattern = Regex::new(&format!(r"(?i)\b(?:(\d{{4}})-(\d{{2}})-(\d{{2}})(?:[T ]?)|(today|tonight|tomorrow|idag|ikväll|imorgon|i morgon)|(?:next\s+|nästa\s+|on\s+|på\s+)?(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tue|wed|thu|fri|sat|sun|måndag|tisdag|onsdag|torsdag|fredag|lördag|söndag)\b,?\s*(?:(\d{{1,2}})(?:st|nd|rd|th)?\s+({mon})\b\.?)?|({mon})\b\.?\s+(\d{{1,2}})(?:st|nd|rd|th)?\b(?:,?\s*(\d{{4}}))?|(\d{{1,2}})(?:st|nd|rd|th)?\s+(?:of\s+)?({mon})\b\.?(?:\s+(\d{{4}}))?)")).unwrap();
    let (mut out, mut seen) = (Vec::new(), std::collections::HashSet::new());
    for c in pattern.captures_iter(&input) {
        let whole = c.get(0).unwrap();
        let date: Option<NaiveDate> = if let Some(y) = c.get(1) {
            NaiveDate::from_ymd_opt(y.as_str().parse().ok().unwrap_or(0), c[2].parse().unwrap_or(0), c[3].parse().unwrap_or(0))
        } else if let Some(rel) = c.get(4) {
            let r = rel.as_str().to_lowercase();
            Some(if r.contains("mor") { today + Duration::days(1) } else { today })
        } else if let Some(wd) = c.get(5).and_then(|d| weekday(d.as_str())) {
            match (c.get(6), c.get(7).and_then(|m| month(m.as_str()))) {
                (Some(d), Some(m)) => year_for(today, m, d.as_str().parse().unwrap_or(0), None),
                _ => { let mut d = today + Duration::days(1); while d.weekday() != wd { d += Duration::days(1); } Some(d) }
            }
        } else if let (Some(m), Some(d)) = (c.get(8).and_then(|m| month(m.as_str())), c.get(9)) {
            year_for(today, m, d.as_str().parse().unwrap_or(0), c.get(10).and_then(|y| y.as_str().parse().ok()))
        } else if let (Some(d), Some(m)) = (c.get(11), c.get(12).and_then(|m| month(m.as_str()))) {
            year_for(today, m, d.as_str().parse().unwrap_or(0), c.get(13).and_then(|y| y.as_str().parse().ok()))
        } else { None };
        let Some(date) = date else { continue };
        let rest = &input[whole.end()..];
        let (time, extra) = time_after(rest).unwrap_or((NaiveTime::from_hms_opt(12, 0, 0).unwrap(), 0));
        let Some(start) = Local.from_local_datetime(&date.and_time(time)).single() else { continue };
        if start < now - Duration::hours(1) || start > now + Duration::days(366) { continue; }
        let iso = start.with_timezone(&Utc).format("%Y-%m-%dT%H:%M:%SZ").to_string();
        if !seen.insert(iso.clone()) { continue; }
        let end = (start + Duration::hours(1)).with_timezone(&Utc).format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let phrase = input[whole.start()..whole.end() + extra].trim().to_string();
        out.push(json!({"title": title.chars().take(200).collect::<String>(), "start": iso, "end": end, "phrase": phrase}));
        if out.len() == 8 { break; }
    }
    out
}
/// A date without a year means the next time it comes round.
fn year_for(today: NaiveDate, m: u32, d: u32, year: Option<i32>) -> Option<NaiveDate> {
    if let Some(y) = year { return NaiveDate::from_ymd_opt(y, m, d); }
    let this = NaiveDate::from_ymd_opt(today.year(), m, d)?;
    if this >= today { Some(this) } else { NaiveDate::from_ymd_opt(today.year() + 1, m, d) }
}

pub fn parse_iso(text: &str) -> Option<DateTime<Utc>> { DateTime::parse_from_rfc3339(text).ok().map(|d| d.with_timezone(&Utc)) }

/// An .ics file with one event (no attendees, no email text), for the user's calendar app.
pub fn ics(title: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    let esc = title.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace(['\r', '\n'], " ");
    let f = |d: DateTime<Utc>| d.format("%Y%m%dT%H%M%SZ").to_string();
    format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//The Stupid Mail//EN\r\nBEGIN:VEVENT\r\nUID:{}@stupidmail.local\r\nDTSTAMP:{}\r\nDTSTART:{}\r\nDTEND:{}\r\nSUMMARY:{}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        uuid::Uuid::new_v4(), f(Utc::now()), f(start), f(end), esc)
}
