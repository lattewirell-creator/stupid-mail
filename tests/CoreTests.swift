import Foundation
final class FixtureChannel: IMAPChannel {
 var remaining: Data
 var sent: [String] = []
 init(_ text: String) { remaining=Data(text.utf8) }
 func send(_ data:Data) throws { sent.append(String(decoding:data,as:UTF8.self)) }
 func line() throws -> Data {guard let end=remaining.range(of:Data([13,10])) else {throw MailFailure(message:"test exhausted")};let line=Data(remaining[..<end.upperBound]);remaining.removeSubrange(..<end.upperBound);return line}
 func bytes(_ count:Int) throws -> Data {guard count<=remaining.count else {throw MailFailure(message:"test incomplete")};let data=Data(remaining.prefix(count));remaining.removeFirst(count);return data}
 func close() {}
}
@main struct CoreTests {
 static func main() throws {
  var count=0
  func check(_ value:Bool,_ name:String){if !value{fatalError("FAILED: "+name)};count+=1}
  check(MailValidation.iCloudAddress("emil@icloud.com"),"icloud address")
  check(!MailValidation.iCloudAddress("emil@example.com"),"primary address restriction")
  check(!MailValidation.address("a@b.com\r\nBcc: bad@x.com"),"header injection")
  check(MailValidation.appPassword("abcd-efgh-ijkl-mnop"),"app password")
  check(!MailValidation.appPassword("regularPassword!"),"reject normal password")
  do{_=try MailValidation.quote("secret\nurl=evil");fatalError("config injection accepted")}catch{count+=1}
  check(try MailValidation.quote("a\"b\\c")=="\"a\\\"b\\\\c\"","config escaping")
  check(MIMEParser.header("=?UTF-8?B?SGVsbG8g8J+Riw==?=")=="Hello 👋","encoded header")
  check(MIMEParser.header("=?UTF-8?Q?Hej_Emil?=")=="Hej Emil","Q header")
  check(String(data:MIMEParser.quotedPrintable(Data("Hej=20d=C3=A5=\r\n!".utf8)),encoding:.utf8)=="Hej då!","quoted printable")
  let raw="From: =?UTF-8?Q?Jamie_Chen?= <jamie@example.com>\r\nSubject: Hello\r\nMessage-ID: <abc@example.com>\r\nIn-Reply-To: <old@example.com>\r\nContent-Type: multipart/mixed; boundary=\"outside\"\r\n\r\n--outside\r\nContent-Type: multipart/alternative; boundary=inside\r\n\r\n--inside\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\nSGVsbG8gRW1pbA==\r\n--inside\r\nContent-Type: text/html\r\n\r\n<p>Hello <b>Emil</b></p><script>evil()</script>\r\n--inside--\r\n--outside\r\nContent-Type: application/pdf; name=\"file.pdf\"\r\nContent-Disposition: attachment\r\n\r\n123\r\n--outside--\r\n"
  let record=MIMEParser.record(Data(raw.utf8),id:"42",flags:"FLAGS (\\Seen \\Flagged)")
  check(record.body=="Hello Emil","prefer plain multipart")
  check(record.from=="Jamie Chen" && record.email=="jamie@example.com","sender")
  check(record.attachmentCount==1,"attachment count")
  check(!record.unread && record.star,"read and star flags")
  check(record.inReplyTo=="<old@example.com>","thread reference")
  check(MIMEParser.plainHTML("<style>hidden</style><p>Hi &amp; &#x1F44B;</p><img src='https://tracker'>")=="Hi & 👋","safe HTML text")
  let utf8="Subject: emoji\r\n\r\n🌻 hello";let bytes=Data(utf8.utf8)
  var fetch=Data("* 1 FETCH (UID 42 FLAGS () BODY[] {\(bytes.count)}\r\n".utf8);fetch.append(bytes);fetch.append(Data("\r\n)\r\n".utf8))
  let literals=try IMAPParser.literals(fetch)
  check(literals.count==1 && literals[0].data==bytes,"UTF8 literal byte lengths")
  do{_=try IMAPParser.literals(Data("* 1 FETCH (BODY[] {800}\r\nshort".utf8));fatalError("truncated accepted")}catch{count+=1}
  check(IMAPParser.search(Data("* SEARCH 4 1 22\r\n".utf8))==[1,4,22],"UID search")
  let outgoing=try ICloudMail.outgoing(from:"me@icloud.com",to:"you@example.com",subject:"Hej 🌻",body:"hello\nworld",replyTo:"<orig@example.com>")
  let sent=MIMEParser.record(outgoing,id:"sent",flags:"")
  check(sent.subject=="Hej 🌻" && sent.body=="hello\r\nworld","outgoing MIME round trip")
  check(sent.inReplyTo=="<orig@example.com>","outgoing threading")
  do{_=try ICloudMail.outgoing(from:"me@icloud.com",to:"you@example.com",subject:"Hi\r\nBcc: bad@example.com",body:"Hi",replyTo:nil);fatalError("unsafe subject")}catch{count+=1}
  check(MIMEParser.record(Data("Subject: big\r\n\r\n".utf8),id:"big",flags:"",limited:true).body.contains("too large"),"large mail fallback")
  let poison = "Subject: literal\r\n\r\nM1 OK not-a-command\r\n* BYE not-a-command"
  let wire=FixtureChannel("* OK ready\r\n* 1 FETCH (BODY[] {\(poison.utf8.count)}\r\n\(poison) UID 9)\r\nM1 OK complete\r\n")
  let client=try IMAPConnection(channel:wire)
  let response=try client.command("UID FETCH 9 (UID BODY.PEEK[])")
  let parsed=try IMAPParser.literals(response)
  check(parsed.count==1 && String(decoding:parsed[0].data,as:UTF8.self)==poison,"literal cannot spoof protocol commands")
  check(parsed[0].prefix.contains("UID 9"),"UID after literal")
  check(wire.sent==["M1 UID FETCH 9 (UID BODY.PEEK[])\r\n"],"read without marking seen")
  let rejected=try IMAPConnection(channel:FixtureChannel("* OK ready\r\nM1 NO authentication failed\r\n"))
  do{try rejected.authenticate(MailCredentials(email:"test@icloud.com",password:"abcd-efgh-ijkl-mnop"));fatalError("bad login accepted")}catch{count+=1}
  check(try MailCache.decode(Data("[{\"id\":\"draft\",\"body\":\"hello\"}]".utf8)).count==1,"native cache format")
  do{_=try MailCache.decode(Data("{\"wrong\":true}".utf8));fatalError("wrong cache accepted")}catch{count+=1}
  check(try MailCache.location(account:"me@gmail.com").lastPathComponent != MailCache.location(account:"me@outlook.com").lastPathComponent,"Gmail and Outlook get their own local mailbox")
  print("Passed \(count) mail-core checks.")
 }
}
