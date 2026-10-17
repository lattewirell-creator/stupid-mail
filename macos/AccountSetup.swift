import AppKit

extension AppDelegate {
    func connectAccount(_ id:String) {
        guard !mailBusy,window.attachedSheet == nil else {respond(id,error:"Finish the current operation first.");return}
        let alert=NSAlert();alert.messageText="Connect your mail"
        alert.informativeText="Choose your provider. One account is active at a time; connecting another replaces the saved sign-in. Drafts and sorting stay separate for each email address."
        let provider=NSPopUpButton(frame:NSRect(x:0,y:0,width:400,height:30));provider.addItems(withTitles:["iCloud — Apple app password","Gmail — Google app password","Outlook — Microsoft sign-in"]);alert.accessoryView=provider
        alert.addButton(withTitle:"Continue");alert.addButton(withTitle:"Cancel")
        alert.beginSheetModal(for:window){response in
            guard response == .alertFirstButtonReturn else {self.respond(id,error:"Cancelled.");return}
            DispatchQueue.main.async {
                let selected:[MailProvider]=[.icloud,.gmail,.outlook]
                if selected[provider.indexOfSelectedItem] == .outlook {self.connectOutlook(id)}
                else {self.connectPasswordAccount(id,provider:selected[provider.indexOfSelectedItem])}
            }
        }
    }
    func connectOutlook(_ id:String) {
        let alert=NSAlert();alert.messageText="Connect Outlook"
        alert.informativeText="Sign in with Microsoft in your browser. This build needs an application client ID from a Microsoft app registration. No client secret or Microsoft password goes here.\n\nRegistration: personal + work/school accounts, public client flows enabled, and delegated User.Read, Mail.Read, Mail.Send permissions. See the setup guide for steps."
        let client=NSTextField(frame:NSRect(x:0,y:0,width:420,height:26));client.placeholderString="Microsoft application (client) ID";client.stringValue=UserDefaults.standard.string(forKey:"microsoftClientID") ?? "";alert.accessoryView=client
        alert.addButton(withTitle:"Sign in");alert.addButton(withTitle:"Cancel");alert.addButton(withTitle:"Setup guide")
        alert.beginSheetModal(for:window){response in
            if response == .alertThirdButtonReturn {NSWorkspace.shared.open(URL(string:"https://github.com/lattewirell-creator/stupid-mail/blob/main/docs/accounts.md")!);self.respond(id,error:"Follow the Outlook setup guide, then connect again.");return}
            guard response == .alertFirstButtonReturn else {self.respond(id,error:"Cancelled.");return}
            let clientID=client.stringValue.trimmingCharacters(in:.whitespacesAndNewlines)
            guard UUID(uuidString:clientID) != nil else {self.respond(id,error:"Enter the application client ID from your Microsoft app registration.");return}
            UserDefaults.standard.set(clientID,forKey:"microsoftClientID");self.mailBusy=true
            DispatchQueue.global(qos:.userInitiated).async {
                do {
                    let device=try OutlookMail.auth("devicecode",["client_id":clientID,"scope":OutlookMail.scope])
                    guard let code=device["user_code"] as? String,let secret=device["device_code"] as? String else {throw MailFailure(message:"Microsoft could not start sign-in. Check the client ID and enable public client flows in the registration.")}
                    let cancellation=SignInCancellation()
                    DispatchQueue.main.async {
                        let prompt=NSAlert();prompt.messageText="Finish Microsoft sign-in"
                        prompt.informativeText="Open microsoft.com/devicelogin and enter this code:\n\n\(code)\n\nApprove access only for your The Stupid Mail registration. This window closes when sign-in finishes."
                        let field=NSTextField(frame:NSRect(x:0,y:0,width:380,height:26));field.stringValue=code;field.isEditable=false;field.isSelectable=true;prompt.accessoryView=field
                        prompt.addButton(withTitle:"Cancel sign-in")
                        prompt.beginSheetModal(for:self.window){_ in cancellation.cancel()}
                        NSWorkspace.shared.open(URL(string:"https://microsoft.com/devicelogin")!)
                        DispatchQueue.global(qos:.userInitiated).async {
                            do {
                                let deadline=Date().addingTimeInterval(Double(min(device["expires_in"] as? Int ?? 120,120)))
                                var interval=max(device["interval"] as? Int ?? 5,5)
                                var credentials:MailCredentials?
                                while Date()<deadline && !cancellation.cancelled {
                                    Thread.sleep(forTimeInterval:Double(interval))
                                    if cancellation.cancelled {break}
                                    let token=try OutlookMail.auth("token",["client_id":clientID,"device_code":secret,"grant_type":"urn:ietf:params:oauth:grant-type:device_code"])
                                    if token["error"] as? String == "authorization_pending" {continue}
                                    if token["error"] as? String == "slow_down" {interval+=5;continue}
                                    credentials=try OutlookMail.credentials(token,email:"",clientID:clientID);break
                                }
                                guard let provisional=credentials,!cancellation.cancelled else {throw MailFailure(message:"Microsoft sign-in was cancelled or timed out. Connect again when ready.")}
                                let profile=try JSONSerialization.jsonObject(with:OutlookMail.graph("me?$select=mail,userPrincipalName",token:provisional.password)) as? [String:Any] ?? [:]
                                let email=((profile["mail"] as? String) ?? (profile["userPrincipalName"] as? String) ?? "").lowercased()
                                guard MailValidation.address(email) else {throw MailFailure(message:"Microsoft did not return a usable mailbox address.")}
                                let c=MailCredentials(email:email,password:provisional.password,provider:.outlook,clientID:clientID,refreshToken:provisional.refreshToken,expiresAt:provisional.expiresAt)
                                let messages=try self.jsonRecords(OutlookMail.sync(c))
                                DispatchQueue.main.async {
                                    // Serialize cancellation and committing the new account on the UI queue.
                                    do {
                                        guard !cancellation.cancelled else {throw MailFailure(message:"Cancelled.")}
                                        try MailKeychain.save(c)
                                        UserDefaults.standard.set(email,forKey:"icloudEmail");UserDefaults.standard.set("outlook",forKey:"mailProvider")
                                        self.window.endSheet(prompt.window);self.mailBusy=false
                                        self.respond(id,data:["email":email,"provider":"outlook","messages":messages])
                                    } catch {
                                        if self.window.attachedSheet === prompt.window {self.window.endSheet(prompt.window)}
                                        self.mailBusy=false;self.respond(id,error:error.localizedDescription)
                                    }
                                }
                            } catch {
                                let text=error.localizedDescription
                                DispatchQueue.main.async {if self.window.attachedSheet === prompt.window {self.window.endSheet(prompt.window)};self.mailBusy=false;self.respond(id,error:text)}
                            }
                        }
                    }
                } catch {let text=error.localizedDescription;DispatchQueue.main.async{self.mailBusy=false;self.respond(id,error:text)}}
            }
        }
    }
}
