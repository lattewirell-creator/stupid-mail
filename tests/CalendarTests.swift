import Foundation
@main struct CalendarTests {
 static func main()throws{
  let now=ISO8601DateFormatter().date(from:"2030-01-01T00:00:00Z")!
  let found=try MeetingDetector.suggestions(text:"Project meeting June 4, 2030 at 10:00 AM.",title:"Project review",now:now)
  precondition(!found.isEmpty,"Explicit future meeting should be found")
  precondition(found[0]["title"] == "Project review")
  precondition(found[0]["phrase"]?.contains("2030") == true)
  let start=ISO8601DateFormatter().date(from:found[0]["start"]!)!,end=ISO8601DateFormatter().date(from:found[0]["end"]!)!
  precondition(end>start,"Suggestion needs an editable end time")
  let past=try MeetingDetector.suggestions(text:"June 4, 2020 at 10:00 AM",title:"Old meeting",now:now)
  precondition(past.isEmpty)
  let empty=try MeetingDetector.suggestions(text:"Thank you for the update.",title:"No meeting",now:now)
  precondition(empty.isEmpty)
  print("Passed future meeting detection, review fields, default duration, past-date rejection, and no-date checks.")
 }
}
