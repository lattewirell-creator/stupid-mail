import AppKit
@preconcurrency import WebKit

extension AppDelegate: WKScriptMessageHandler {
    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        guard message.frameInfo.isMainFrame,
              message.frameInfo.request.url?.standardizedFileURL == Bundle.main.resourceURL?.appendingPathComponent("Web/index.html").standardizedFileURL,
              let request=message.body as? [String:Any],let id=request["id"] as? String,let action=request["action"] as? String else {return}
        switch action {
        case "status":
            let email=UserDefaults.standard.string(forKey:"icloudEmail") ?? ""
            respond(id,data:["email":email,"connected": !email.isEmpty,"provider":UserDefaults.standard.string(forKey:"mailProvider") ?? "icloud"])
        case "readCache":
            let account=request["account"] as? String ?? ""
            cacheQueue.async {
                do {let records=try MailCache.read(account:account);DispatchQueue.main.async{self.respond(id,data:["messages":records])}}
                catch {DispatchQueue.main.async{self.respond(id,error:"Could not read the local mailbox. Your saved files have not been changed.")}}
            }
        case "writeCache":
            guard let contents=request["contents"] as? String else {respond(id,error:"Missing mailbox data.");return}
            let account=request["account"] as? String ?? ""
            cacheQueue.async {
                do {try MailCache.write(account:account,contents:contents);DispatchQueue.main.async{self.respond(id)}}
                catch {DispatchQueue.main.async{self.respond(id,error:"Could not save your mailbox on this Mac. Check free disk space before closing the app.")}}
            }
        case "connect": connectAccount(id)
        case "disconnect": disconnectAccount(id)
        case "sync": runMail(id) { credentials in ["email":credentials.email,"provider":credentials.kind.rawValue,"messages":try self.jsonRecords(credentials.kind == .outlook ? OutlookMail.sync(credentials) : ICloudMail().sync(credentials))] }
        case "send": confirmSend(id,request)
        case "buddyStatus": respond(id,data:buddyStatus())
        case "buddySettings": configureBuddy(id)
        case "buddyAsk": askBuddy(id,request)
        default: respond(id,error:"Unknown action.")
        }
    }
    func respond(_ id: String,data: Any = [:],error: String? = nil) {
        var payload: [String:Any]=["id":id,"ok":error == nil,"data":data]
        if let error {payload["error"]=error}
        webView.callAsyncJavaScript("window.nativeResponse(payload)",arguments:["payload":payload],in:nil,in:.page,completionHandler:nil)
    }
    func jsonRecords(_ records:[MailRecord]) throws -> Any {try JSONSerialization.jsonObject(with:JSONEncoder().encode(records))}
    func nativeNotice(_ text:String){let alert=NSAlert();alert.messageText="The Stupid Mail";alert.informativeText=text;alert.beginSheetModal(for:window,completionHandler:nil)}
    func runMail(_ id:String,work:@escaping(MailCredentials)throws->Any){
        guard !mailBusy else {respond(id,error:"A mail operation is already running. Please wait.");return}
        mailBusy=true
        DispatchQueue.global(qos:.userInitiated).async {
            do{
                guard var c=try MailKeychain.load() else {throw MailFailure(message:"Connect your mail account first.")}
                if c.kind == .outlook { c=try OutlookMail.refreshed(c) }
                let result=try work(c)
                DispatchQueue.main.async {self.mailBusy=false;self.respond(id,data:result)}
            }catch{let text=error.localizedDescription;DispatchQueue.main.async{self.mailBusy=false;self.respond(id,error:text)}}
        }
    }
    @objc func accountMenu(){webView.evaluateJavaScript("document.getElementById('connect-account').click()",completionHandler:nil)}
    func connectPasswordAccount(_ id:String,provider:MailProvider){
        guard !mailBusy,window.attachedSheet == nil else {respond(id,error:"Finish the current operation first.");return}
        let alert=NSAlert();alert.messageText="Connect \(provider.title) Mail"
        alert.informativeText=provider == .icloud ? "Use your primary iCloud address and an Apple app-specific password from account.apple.com. Your password is stored in macOS Keychain. Latest 40 inbox messages; local sorting and drafts." : "Use your full Gmail or Google Workspace address and a 16-letter Google app password. Enable 2-Step Verification, then create one at myaccount.google.com/apppasswords. Some work/school or protected accounts do not allow app passwords. Your password is stored in macOS Keychain."
        let form=NSView(frame:NSRect(x:0,y:0,width:400,height:112))
        let email=NSTextField(frame:NSRect(x:0,y:75,width:400,height:26));email.placeholderString=provider == .icloud ? "you@icloud.com" : "you@gmail.com";email.stringValue=UserDefaults.standard.string(forKey:"icloudEmail") ?? ""
        let password=NSSecureTextField(frame:NSRect(x:0,y:32,width:400,height:26));password.placeholderString="App-specific password (xxxx-xxxx-xxxx-xxxx)"
        let note=NSTextField(labelWithString:"Use an app password, not your normal account password.");note.frame=NSRect(x:0,y:0,width:400,height:20);note.font = .systemFont(ofSize:11)
        form.addSubview(email);form.addSubview(password);form.addSubview(note);alert.accessoryView=form
        alert.addButton(withTitle:"Connect");alert.addButton(withTitle:"Cancel");alert.addButton(withTitle:"Create app password")
        alert.beginSheetModal(for:window){response in
            if response == .alertThirdButtonReturn {NSWorkspace.shared.open(URL(string:provider == .icloud ? "https://account.apple.com/" : "https://myaccount.google.com/apppasswords")!);self.respond(id,error:"Create an app-specific password, then connect again.");return}
            guard response == .alertFirstButtonReturn else {self.respond(id,error:"Cancelled.");return}
            let address=email.stringValue.trimmingCharacters(in:.whitespacesAndNewlines).lowercased(),secret=password.stringValue.trimmingCharacters(in:.whitespacesAndNewlines).replacingOccurrences(of:" ",with:"")
            password.stringValue=""
            guard (provider == .icloud ? MailValidation.iCloudAddress(address) : MailValidation.address(address)),MailValidation.appPassword(secret) else {self.respond(id,error:"Check your address and use a 16-letter app password from your provider.");return}
            self.mailBusy=true
            DispatchQueue.global(qos:.userInitiated).async{
                do{
                    let c=MailCredentials(email:address,password:secret,provider:provider)
                    let messages=try self.jsonRecords(ICloudMail().sync(c));try MailKeychain.save(c)
                    UserDefaults.standard.set(address,forKey:"icloudEmail");UserDefaults.standard.set(provider.rawValue,forKey:"mailProvider")
                    DispatchQueue.main.async{self.mailBusy=false;self.respond(id,data:["email":address,"provider":provider.rawValue,"messages":messages])}
                }catch{let text=error.localizedDescription;DispatchQueue.main.async{self.mailBusy=false;self.respond(id,error:text)}}
            }
        }
    }
    func disconnectAccount(_ id:String){
        guard !mailBusy,window.attachedSheet == nil else {respond(id,error:"Wait for the current operation to finish.");return}
        let alert=NSAlert();alert.messageText="Disconnect this account?";alert.informativeText="This removes the saved sign-in from this Mac. Your mail on the server is unchanged. Local drafts and sorting are kept for when you reconnect.";alert.addButton(withTitle:"Disconnect");alert.addButton(withTitle:"Cancel")
        alert.beginSheetModal(for:window){result in
            guard result == .alertFirstButtonReturn else {self.respond(id,error:"Cancelled.");return}
            do{try MailKeychain.remove();UserDefaults.standard.removeObject(forKey:"icloudEmail");UserDefaults.standard.removeObject(forKey:"mailProvider");self.respond(id)}catch{self.respond(id,error:error.localizedDescription)}
        }
    }
    func confirmSend(_ id:String,_ request:[String:Any]){
        guard !mailBusy,window.attachedSheet == nil else {respond(id,error:"Finish the current operation first.");return}
        guard let to=request["to"] as? String,let subject=request["subject"] as? String,let body=request["body"] as? String,MailValidation.address(to),subject.utf8.count<=900,body.utf8.count<=1024*1024 else {respond(id,error:"Check the recipient and message length.");return}
        let alert=NSAlert();alert.messageText="Send this email?";alert.informativeText="To: \(to)\nSubject: \(subject)\n\nThis will send a real email from your connected mail account.";alert.addButton(withTitle:"Send");alert.addButton(withTitle:"Cancel")
        alert.beginSheetModal(for:window){result in
            guard result == .alertFirstButtonReturn else {self.respond(id,error:"Cancelled.");return}
            self.runMail(id){ c in let messageID = try c.kind == .outlook ? OutlookMail.send(c,to:to,subject:subject,body:body,replyTo:request["replyTo"] as? String) : ICloudMail().send(c,to:to,subject:subject,body:body,replyTo:request["replyTo"] as? String);return ["sent":true,"messageID":messageID] }
        }
    }
    func buddyStatus()->[String:Any]{
        guard let settings=BuddyStore.load() else {return ["ready":false,"provider":"","model":""]}
        return ["ready":true,"provider":settings.provider,"model":settings.model]
    }
    func configureBuddy(_ id:String){
        guard window.attachedSheet == nil else {respond(id,error:"Close the current dialog first.");return}
        let current=BuddyStore.load()
        let alert=NSAlert();alert.messageText="Wake up Buddy";alert.informativeText="Choose an AI and enter its model name. Cloud providers receive your questions, plus the selected email only when you enable ‘Include this email’. API usage may be billed by that provider. Ollama runs locally; use a locally installed model.\n\nKeys are saved in macOS Keychain. Existing browser keys are not imported."
        let form=NSView(frame:NSRect(x:0,y:0,width:420,height:120))
        let provider=NSPopUpButton(frame:NSRect(x:0,y:87,width:420,height:28));provider.addItems(withTitles:["Ollama — local","OpenAI","Claude — Anthropic"])
        provider.selectItem(at:["ollama","openai","anthropic"].firstIndex(of:current?.provider ?? "ollama") ?? 0)
        let model=NSTextField(frame:NSRect(x:0,y:47,width:420,height:26));model.placeholderString="Exact model name from your provider";model.stringValue=current?.model ?? ""
        let key=NSSecureTextField(frame:NSRect(x:0,y:7,width:420,height:26));key.placeholderString="API key (not needed for Ollama)"
        form.addSubview(provider);form.addSubview(model);form.addSubview(key);alert.accessoryView=form;alert.addButton(withTitle:"Save");alert.addButton(withTitle:"Cancel")
        alert.beginSheetModal(for:window){result in
            guard result == .alertFirstButtonReturn else {self.respond(id,error:"Cancelled.");return}
            let p=["ollama","openai","anthropic"][provider.indexOfSelectedItem],m=model.stringValue.trimmingCharacters(in:.whitespacesAndNewlines)
            let secret=key.stringValue.isEmpty && current?.provider == p ? current?.key ?? "" : key.stringValue.trimmingCharacters(in:.whitespacesAndNewlines);key.stringValue=""
            guard !m.isEmpty,m.count<=160,m.range(of:#"^[A-Za-z0-9._:/-]+$"#,options:.regularExpression) != nil,(p=="ollama" || (!secret.isEmpty && secret.utf8.count<=512 && !secret.contains("\n"))) else {self.respond(id,error:"Enter a model name and, for cloud providers, an API key.");return}
            do{try BuddyStore.save(BuddySettings(provider:p,model:m,key:secret));self.respond(id,data:self.buddyStatus())}catch{self.respond(id,error:error.localizedDescription)}
        }
    }
    func askBuddy(_ id:String,_ request:[String:Any]){
        guard !buddyBusy else {respond(id,error:"Buddy is already thinking.");return}
        guard let settings=BuddyStore.load(),let question=request["question"] as? String,!question.isEmpty else {respond(id,error:"Open Buddy settings and choose an AI first.");return}
        let context=request["context"] as? String ?? ""
        buddyBusy=true
        Task {
            do{let answer=try await BuddyAI.ask(settings:settings,question:question,context:context);self.buddyBusy=false;self.respond(id,data:["answer":answer])}
            catch{self.buddyBusy=false;self.respond(id,error:error is MailFailure ? error.localizedDescription : "Buddy could not reach the AI. Check your connection, settings, or local Ollama app.")}
        }
    }
}
