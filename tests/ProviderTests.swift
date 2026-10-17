import Foundation
@main struct ProviderTests {
 static func main() throws {
  var count=0
  func check(_ value:Bool,_ name:String){if !value{fatalError("FAILED: "+name)};count+=1}
  let old=try JSONDecoder().decode(MailCredentials.self,from:Data(#"{"email":"old@icloud.com","password":"fixture"}"#.utf8))
  check(old.kind == .icloud,"existing iCloud credentials migrate")
  let gmail=MailCredentials(email:"me@example.com",password:"abcdefghijklmnop",provider:.gmail)
  check(gmail.kind.imapHost == "imap.gmail.com" && gmail.kind.smtpHost == "smtp.gmail.com","Google endpoints")
  check(old.kind.imapHost == "imap.mail.me.com","Apple endpoint unchanged")
  check(try JSONDecoder().decode(MailCredentials.self,from:JSONEncoder().encode(gmail)).kind == .gmail,"provider persists")
  check(String(decoding:MailHTTP.form(["token":"a+b&= ?","scope":"Mail.Read Mail.Send"]),as:UTF8.self)=="scope=Mail.Read%20Mail.Send&token=a%2Bb%26%3D%20%3F","OAuth form escapes tokens")
  let c=try OutlookMail.credentials(["access_token":"fixture-token","refresh_token":"fixture-refresh","expires_in":3600],email:"me@outlook.com",clientID:"fixture-client")
  check(c.kind == .outlook && c.refreshToken == "fixture-refresh","token stored with provider")
  do{_=try MailTransport().perform(credentials:c);fatalError("Outlook token sent to IMAP")}catch{count+=1}
  do{_=try ICloudMail().sync(c);fatalError("Outlook token sent to IMAP")}catch{count+=1}
  check(c.expiresAt!.timeIntervalSinceNow>3500,"token expiry recorded")
  let renewed=try OutlookMail.credentials(["access_token":"new-token"],email:c.email,clientID:c.clientID!,oldRefresh:c.refreshToken)
  check(renewed.refreshToken == c.refreshToken,"refresh token retained when not rotated")
  let rotated=try OutlookMail.credentials(["access_token":"new-token","refresh_token":"new-refresh"],email:c.email,clientID:c.clientID!,oldRefresh:c.refreshToken)
  check(rotated.refreshToken == "new-refresh","refresh token rotation")
  do{_=try OutlookMail.credentials(["error":"authorization_declined"],email:"",clientID:"");fatalError("denied auth accepted")}catch{count+=1}
  do{_=try MailHTTP.request(URL(string:"https://example.com/")!,token:"must-not-leak");fatalError("untrusted endpoint accepted")}catch{count+=1}
  let input: [String:Any] = ["value":[["id":"immutable","from":["emailAddress":["name":"Jamie","address":"jamie@example.com"]],"subject":"A message","body":["contentType":"html","content":"<p>Hello &amp; welcome</p><script>bad()</script>"],"isRead":false,"hasAttachments":true,"flag":["flagStatus":"flagged"],"internetMessageId":"<one@example.com>","internetMessageHeaders":[["name":"In-Reply-To","value":"<original@example.com>"],["name":"List-Unsubscribe","value":"<https://example.com>"]]]]]
  let records=try OutlookMail.records(JSONSerialization.data(withJSONObject:input));let m=records[0]
  check(m.id == "outlook:immutable" && m.email == "jamie@example.com","Graph identity and sender")
  check(m.body == "Hello & welcome","Graph HTML converted to safe text")
  check(m.unread && m.star && m.newsletter && m.attachmentCount>0,"Graph metadata")
  check(m.inReplyTo == "<original@example.com>","Graph reply threading")
  do{_=try OutlookMail.records(Data(#"{"value":[{}]}"#.utf8));fatalError("missing identity accepted")}catch{count+=1}
  let cancelled=SignInCancellation();check(!cancelled.cancelled,"sign-in starts active");cancelled.cancel();check(cancelled.cancelled,"sign-in cancellation")
  print("Passed \(count) provider and OAuth checks.")
 }
}
