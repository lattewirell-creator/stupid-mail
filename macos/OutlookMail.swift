import Foundation

// Requests run on the mail worker queue. Tokens never cross the web-view bridge.
final class MailHTTP: NSObject, URLSessionTaskDelegate {
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) { completionHandler(nil) }
    static func request(_ url: URL, token: String? = nil, body: Data? = nil, contentType: String = "application/json") throws -> (Int, Data) {
        guard url.scheme == "https", ["login.microsoftonline.com", "graph.microsoft.com"].contains(url.host ?? "") else {throw MailFailure(message:"Invalid Microsoft endpoint.")}
        var request=URLRequest(url:url);request.timeoutInterval=30
        if let token {request.setValue("Bearer " + token,forHTTPHeaderField:"Authorization")}
        request.setValue("outlook.body-content-type=\"text\", IdType=\"ImmutableId\"",forHTTPHeaderField:"Prefer")
        if let body {request.httpMethod="POST";request.httpBody=body;request.setValue(contentType,forHTTPHeaderField:"Content-Type")}
        let config=URLSessionConfiguration.ephemeral;config.urlCache=nil;config.httpCookieStorage=nil
        let session=URLSession(configuration:config,delegate:MailHTTP(),delegateQueue:nil)
        defer {session.invalidateAndCancel()}
        let done=DispatchSemaphore(value:0);var result: (Int,Data)?
        session.dataTask(with:request){data,response,error in
            if error == nil,let response=response as? HTTPURLResponse,let data,data.count<=16*1024*1024 {result=(response.statusCode,data)}
            done.signal()
        }.resume()
        guard done.wait(timeout:.now()+35) == .success,let result else {throw MailFailure(message:"Microsoft could not be reached or returned too much data. Check your connection and refresh. If sending, check Sent before retrying.")}
        return result
    }
    static func form(_ fields:[String:String]) -> Data {
        let safe=CharacterSet(charactersIn:"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~")
        return Data(fields.sorted{$0.key<$1.key}.map{($0.key.addingPercentEncoding(withAllowedCharacters:safe) ?? "")+"="+($0.value.addingPercentEncoding(withAllowedCharacters:safe) ?? "")}.joined(separator:"&").utf8)
    }
}
final class SignInCancellation {
    private let lock=NSLock();private var value=false
    var cancelled: Bool {lock.lock();defer{lock.unlock()};return value}
    func cancel(){lock.lock();value=true;lock.unlock()}
}
enum OutlookMail {
    static let scope="offline_access https://graph.microsoft.com/User.Read https://graph.microsoft.com/Mail.Read https://graph.microsoft.com/Mail.Send"
    static func auth(_ path:String,_ fields:[String:String]) throws -> [String:Any] {
        let (status,data)=try MailHTTP.request(URL(string:"https://login.microsoftonline.com/common/oauth2/v2.0/"+path)!,body:MailHTTP.form(fields),contentType:"application/x-www-form-urlencoded")
        guard let object=try JSONSerialization.jsonObject(with:data) as? [String:Any] else {throw MailFailure(message:"Microsoft returned an unexpected sign-in response.")}
        guard status == 200 || object["error"] != nil else {throw MailFailure(message:"Microsoft sign-in is unavailable. Try again later.")}
        return object
    }
    static func credentials(_ object:[String:Any],email:String,clientID:String,oldRefresh:String? = nil) throws -> MailCredentials {
        guard let token=object["access_token"] as? String,!token.isEmpty,let refresh=object["refresh_token"] as? String ?? oldRefresh,!refresh.isEmpty else {throw MailFailure(message:"Microsoft sign-in expired or was denied. Reconnect Outlook. Check that your app registration allows public client sign-in and the requested mail permissions.")}
        return MailCredentials(email:email,password:token,provider:.outlook,clientID:clientID,refreshToken:refresh,expiresAt:Date().addingTimeInterval(Double(object["expires_in"] as? Int ?? 3600)))
    }
    static func refreshed(_ c:MailCredentials) throws -> MailCredentials {
        if let expiry=c.expiresAt,expiry.timeIntervalSinceNow>120{return c}
        guard let client=c.clientID,let refresh=c.refreshToken else {throw MailFailure(message:"Reconnect Outlook to renew sign-in.")}
        let next=try credentials(auth("token",["grant_type":"refresh_token","client_id":client,"refresh_token":refresh,"scope":scope]),email:c.email,clientID:client,oldRefresh:refresh)
        try MailKeychain.save(next);return next
    }
    static func graph(_ path:String,token:String,body:Data? = nil,contentType:String = "application/json") throws -> Data {
        let (status,data)=try MailHTTP.request(URL(string:"https://graph.microsoft.com/v1.0/"+path)!,token:token,body:body,contentType:contentType)
        guard (200...299).contains(status) else {
            if status == 401 {throw MailFailure(message:"Microsoft sign-in has expired. Reconnect Outlook.")}
            if status == 403 {throw MailFailure(message:"Microsoft blocked mail access. Check the app’s Mail.Read and Mail.Send permissions or contact your work/school administrator.")}
            if status == 429 {throw MailFailure(message:"Microsoft needs a short break. Wait a minute before refreshing or retrying.")}
            throw MailFailure(message:body == nil ? "Microsoft could not load the inbox. Try refreshing later." : "Sending was not confirmed. Check Outlook Sent before retrying to avoid a duplicate.")
        }
        return data
    }
    static func records(_ data:Data) throws -> [MailRecord] {
        guard let object=try JSONSerialization.jsonObject(with:data) as? [String:Any],let list=object["value"] as? [[String:Any]],list.count<=40 else {throw MailFailure(message:"Microsoft returned an unexpected mailbox response.")}
        return try list.map {m in
            guard let id=m["id"] as? String,!id.isEmpty else {throw MailFailure(message:"Microsoft returned a message without an identifier.")}
            let sender=(m["from"] as? [String:Any])?["emailAddress"] as? [String:Any] ?? [:]
            let body=m["body"] as? [String:Any] ?? [:],content=body["content"] as? String ?? ""
            let headers=m["internetMessageHeaders"] as? [[String:String]] ?? []
            func header(_ name:String)->String {headers.first{($0["name"] ?? "").lowercased()==name}?["value"] ?? ""}
            return MailRecord(id:"outlook:"+id,from:sender["name"] as? String ?? "Unknown sender",email:sender["address"] as? String ?? "",subject:m["subject"] as? String ?? "(No subject)",body:content.utf8.count>1024*1024 ? "This message is too large. Open Outlook to read it." : (body["contentType"] as? String ?? "").lowercased()=="html" ? MIMEParser.plainHTML(content) : content,time:m["receivedDateTime"] as? String ?? "",unread:!(m["isRead"] as? Bool ?? false),star:(m["flag"] as? [String:Any])?["flagStatus"] as? String == "flagged",newsletter:!header("list-unsubscribe").isEmpty || !header("list-id").isEmpty,attachmentCount:m["hasAttachments"] as? Bool == true ? 1 : 0,messageID:m["internetMessageId"] as? String ?? "",inReplyTo:header("in-reply-to"))
        }
    }
    static func sync(_ c:MailCredentials) throws -> [MailRecord] {
        let path="me/mailFolders/inbox/messages?$top=40&$orderby=receivedDateTime%20desc&$select=id,from,subject,body,receivedDateTime,isRead,flag,hasAttachments,internetMessageId,internetMessageHeaders"
        return try records(graph(path,token:c.password))
    }
    static func send(_ c:MailCredentials,to:String,subject:String,body:String,replyTo:String?) throws -> String {
        let mime=try ICloudMail.outgoing(from:c.email,to:to,subject:subject,body:body,replyTo:replyTo)
        _=try graph("me/sendMail",token:c.password,body:Data(mime.base64EncodedString().utf8),contentType:"text/plain")
        return MIMEParser.split(mime).0["message-id"] ?? ""
    }
}
