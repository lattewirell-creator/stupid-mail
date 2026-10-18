import AppKit
import EventKit

extension AppDelegate {
    func calendarAction(_ id:String,action:String,request:[String:Any]) {
        if action=="calendarSuggest" {
            let items=request["items"] as? [[String:String]] ?? []
            do {var suggestions:[[String:String]]=[]
                for item in items.prefix(40) {for var suggestion in try MeetingDetector.suggestions(text:item["text"] ?? "",title:item["title"] ?? "Meeting") {suggestion["source"]=String((item["source"] ?? "").prefix(100));suggestions.append(suggestion);if suggestions.count==30{break}};if suggestions.count==30{break}}
                respond(id,data:["suggestions":suggestions])
            }catch{respond(id,error:"Could not look for dates in this text.")};return
        }
        if action=="calendarConnect" {
            guard window.attachedSheet==nil else {respond(id,error:"Close the current dialog first.");return}
            let completion:(Bool,Error?)->Void={granted,_ in DispatchQueue.main.async {if granted{self.calendarList(id)}else{self.respond(id,error:"Calendar access was not granted. You can enable it in System Settings → Privacy & Security → Calendars.")}}}
            if #available(macOS 14.0,*){eventStore.requestFullAccessToEvents(completion:completion)}else{eventStore.requestAccess(to:.event,completion:completion)}
            return
        }
        guard calendarAllowed else {respond(id,error:"Connect your Mac calendars first.");return}
        if action=="calendarList"{calendarList(id);return}
        if action=="calendarCreate"{calendarCreate(id,request:request)}
    }
    var calendarAllowed:Bool {let status=EKEventStore.authorizationStatus(for:.event);if #available(macOS 14.0,*){return status == .fullAccess};return status == .authorized}
    func calendarList(_ id:String) {
        guard calendarAllowed else{respond(id,error:"Connect your Mac calendars first.");return}
        let calendars=eventStore.calendars(for:.event).filter{$0.allowsContentModifications}
        let start=Date(),end=start.addingTimeInterval(30*86400)
        let events=eventStore.events(matching:eventStore.predicateForEvents(withStart:start,end:end,calendars:nil)).sorted{$0.startDate<$1.startDate}.prefix(100)
        respond(id,data:["calendars":calendars.map{["id":$0.calendarIdentifier,"title":$0.title]},"events":events.map{["title":$0.title ?? "Event","start":ISO8601DateFormatter().string(from:$0.startDate),"end":ISO8601DateFormatter().string(from:$0.endDate),"calendar":$0.calendar.title,"allDay":$0.isAllDay] as [String:Any]}])
    }
    func calendarCreate(_ id:String,request:[String:Any]) {
        guard window.attachedSheet==nil,!mailBusy else{respond(id,error:"Finish the current operation first.");return}
        guard let title=request["title"] as? String,!title.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty,title.count<=200,let startText=request["start"] as? String,let endText=request["end"] as? String,let start=Self.calendarDate(startText),let end=Self.calendarDate(endText),end>start,end.timeIntervalSince(start)<=7*86400,let calendarID=request["calendarID"] as? String,let calendar=eventStore.calendar(withIdentifier:calendarID),calendar.allowsContentModifications else{respond(id,error:"Check the title, times, and destination calendar.");return}
        let alert=NSAlert();alert.messageText="Add this event?";alert.informativeText="\(title)\n\(start.formatted(date:.abbreviated,time:.shortened)) – \(end.formatted(date:.abbreviated,time:.shortened))\nCalendar: \(calendar.title)\n\nPeople with access to this calendar may see the title and time. No invitation or email body is sent.";alert.addButton(withTitle:"Add event");alert.addButton(withTitle:"Cancel")
        alert.beginSheetModal(for:window){response in
            guard response == .alertFirstButtonReturn else{self.respond(id,error:"Cancelled.");return}
            do{let event=EKEvent(eventStore:self.eventStore);event.title=title;event.startDate=start;event.endDate=end;event.calendar=calendar;try self.eventStore.save(event,span:.thisEvent,commit:true);self.respond(id,data:["saved":true])}catch{self.respond(id,error:"The calendar could not save this event. Check the calendar before retrying.")}
        }
    }
    static func calendarDate(_ text:String)->Date? {let formatter=ISO8601DateFormatter();formatter.formatOptions=[.withInternetDateTime,.withFractionalSeconds];return formatter.date(from:text) ?? ISO8601DateFormatter().date(from:text)}
}
