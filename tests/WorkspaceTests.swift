import Foundation
@main struct WorkspaceTests {
 static func main()throws{
  var checks=0
  func check(_ value:Bool,_ label:String){if !value{fatalError(label)};checks+=1}
  check(WorkspaceStore.valid("https://hooks.slack.com/services/T/B/fixture",provider:"slack"),"Slack URL accepted")
  for url in ["http://hooks.slack.com/services/T/B/fixture","https://hooks.slack.com.evil.test/services/T/B/fixture","https://user@hooks.slack.com/services/T/B/fixture","https://hooks.slack.com:444/services/T/B/fixture","https://example.com/services/T/B/fixture"] {check(!WorkspaceStore.valid(url,provider:"slack"),"Unsafe Slack URL rejected")}
  check(WorkspaceStore.valid("https://prod-1.westeurope.logic.azure.com/workflows/fixture/triggers/manual/paths/invoke?sig=fixture",provider:"teams"),"Teams legacy workflow host")
  check(WorkspaceStore.valid("https://fixture.environment.api.powerplatform.com/powerautomate/automations/direct/workflows/fixture/triggers/manual/paths/invoke?sig=fixture",provider:"teams"),"Teams current workflow host")
  check(!WorkspaceStore.valid("https://evil.test/workflows/fixture?sig=fixture",provider:"teams"),"Foreign workflow rejected")
  check(!WorkspaceStore.valid("https://fixture.environment.api.powerplatform.com/workflows/fixture",provider:"teams"),"Unsigned workflow rejected")
  let text="From: fixture@example.com\n<!channel> <https://example.com> & hello"
  let slack=try JSONSerialization.jsonObject(with:WorkspaceStore.payload(text,provider:"slack")) as! [String:Any]
  let block=(slack["blocks"] as! [[String:Any]])[0]["text"] as! [String:Any]
  check(block["type"] as? String == "plain_text" && block["text"] as? String == text,"Email cannot trigger Slack mentions")
  check(slack["unfurl_links"] as? Bool == false,"No automatic link previews")
  let teams=try JSONSerialization.jsonObject(with:WorkspaceStore.payload(text,provider:"teams")) as! [String:Any]
  check(teams["type"] as? String == "message","Teams card envelope")
  for value in ["",String(repeating:"x",count:2501)] {do{_=try WorkspaceStore.payload(value,provider:"slack");fatalError("Invalid size accepted")}catch{checks+=1}}
  print("Passed \(checks) workspace payload and endpoint checks.")
 }
}
