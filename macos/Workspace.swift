import AppKit
@preconcurrency import WebKit
import Security

final class WorkspaceWindow: NSObject,WKNavigationDelegate,WKUIDelegate {
    let window:NSWindow;let web:WKWebView;let url:URL
    var onSelection:((String)->Void)?
    init(provider:String) {
        url=URL(string:provider=="slack" ? "https://app.slack.com/client/" : "https://teams.microsoft.com/v2/")!
        let config=WKWebViewConfiguration();config.websiteDataStore = .default()
        web=WKWebView(frame:.zero,configuration:config)
        window=NSWindow(contentRect:NSRect(x:0,y:0,width:1100,height:780),styleMask:[.titled,.closable,.miniaturizable,.resizable],backing:.buffered,defer:false)
        super.init();window.title=provider=="slack" ? "Slack — The Stupid Mail" : "Teams — The Stupid Mail";window.isReleasedWhenClosed=false
        let container=NSView(),button=NSButton(title:"Open in browser if sign-in is unavailable here",target:self,action:#selector(external))
        let meeting=NSButton(title:"Find meeting in selected chat text",target:self,action:#selector(selectedMeeting))
        container.addSubview(meeting);meeting.translatesAutoresizingMaskIntoConstraints=false
        container.addSubview(button);container.addSubview(web);button.translatesAutoresizingMaskIntoConstraints=false;web.translatesAutoresizingMaskIntoConstraints=false
        NSLayoutConstraint.activate([meeting.centerYAnchor.constraint(equalTo:button.centerYAnchor),meeting.trailingAnchor.constraint(equalTo:container.trailingAnchor,constant:-12),button.topAnchor.constraint(equalTo:container.topAnchor,constant:8),button.leadingAnchor.constraint(equalTo:container.leadingAnchor,constant:12),web.topAnchor.constraint(equalTo:button.bottomAnchor,constant:8),web.leadingAnchor.constraint(equalTo:container.leadingAnchor),web.trailingAnchor.constraint(equalTo:container.trailingAnchor),web.bottomAnchor.constraint(equalTo:container.bottomAnchor)])
        window.contentView=container;web.navigationDelegate=self;web.uiDelegate=self;window.center();web.load(URLRequest(url:url))
    }
    @objc func selectedMeeting(){web.evaluateJavaScript("window.getSelection().toString().slice(0,6000)"){value,_ in
        guard let text=value as? String,!text.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty else {let alert=NSAlert();alert.messageText="Select a message first";alert.informativeText="Highlight the chat text containing a meeting time, then try again. You can also copy chat text into Mail’s calendar.";alert.beginSheetModal(for:self.window,completionHandler:nil);return}
        self.onSelection?(text)
    }}
    @objc func external(){NSWorkspace.shared.open(url)}
    func webView(_ webView:WKWebView,decidePolicyFor navigationAction:WKNavigationAction,decisionHandler:@escaping(WKNavigationActionPolicy)->Void){decisionHandler(navigationAction.request.url?.scheme=="https" || navigationAction.request.url?.absoluteString=="about:blank" ? .allow : .cancel)}
    func webView(_ webView:WKWebView,createWebViewWith configuration:WKWebViewConfiguration,for navigationAction:WKNavigationAction,windowFeatures:WKWindowFeatures)->WKWebView? {if let url=navigationAction.request.url,url.scheme=="https"{NSWorkspace.shared.open(url)};return nil}
}
extension AppDelegate {
    func workspaceAction(_ id:String,action:String,request:[String:Any]) {
        guard let provider=request["provider"] as? String,["slack","teams"].contains(provider) else {respond(id,error:"Choose Slack or Teams.");return}
        if action=="workspaceOpen" {if workspaceWindows[provider]==nil {workspaceWindows[provider]=WorkspaceWindow(provider:provider);workspaceWindows[provider]?.onSelection={text in
            self.window.makeKeyAndOrderFront(nil)
            self.webView.callAsyncJavaScript("window.receiveMeetingText(text)",arguments:["text":text],in:nil,in:.page,completionHandler:nil)
        }};workspaceWindows[provider]?.window.makeKeyAndOrderFront(nil);respond(id);return}
        if action=="workspaceSettings" {configureWorkspace(id,provider:provider);return}
        if action=="workspaceShare" {shareWorkspace(id,provider:provider,text:request["text"] as? String ?? "");return}
    }
    func configureWorkspace(_ id:String,provider:String) {
        guard window.attachedSheet==nil else {respond(id,error:"Close the current dialog first.");return}
        let alert=NSAlert();alert.messageText="Share emails to \(provider.capitalized)"
        alert.informativeText=provider=="slack" ? "Create a Slack Incoming Webhook for your chosen channel. Give it a recognizable destination label and paste the secret webhook URL below. The URL stays in Keychain. Chat sign-in is separate." : "Create a Teams Workflow using ‘When a Teams webhook request is received’ and a post-card action. Use a signed URL that accepts unauthenticated webhook calls. Tenant-authenticated workflows are not supported. The URL stays in Keychain. Chat sign-in is separate."
        let form=NSView(frame:NSRect(x:0,y:0,width:440,height:76)),label=NSTextField(frame:NSRect(x:0,y:44,width:440,height:26)),secret=NSSecureTextField(frame:NSRect(x:0,y:4,width:440,height:26));label.placeholderString="Destination label, e.g. Design team / #inbox";secret.placeholderString="Secret webhook URL";form.addSubview(label);form.addSubview(secret);alert.accessoryView=form
        do {label.stringValue=try WorkspaceStore.load(provider)?.label ?? ""}catch{respond(id,error:error.localizedDescription);return}
        alert.addButton(withTitle:"Save");alert.addButton(withTitle:"Cancel");alert.addButton(withTitle:"Remove connection")
        alert.beginSheetModal(for:window){response in
            do {
                if response == .alertThirdButtonReturn {try WorkspaceStore.remove(provider);self.respond(id);return}
                guard response == .alertFirstButtonReturn else {self.respond(id,error:"Cancelled.");return}
                let name=label.stringValue.trimmingCharacters(in:.whitespacesAndNewlines),url=secret.stringValue.trimmingCharacters(in:.whitespacesAndNewlines);secret.stringValue=""
                guard !name.isEmpty,name.count<=100,WorkspaceStore.valid(url,provider:provider) else {self.respond(id,error:"Enter a destination label and a valid \(provider.capitalized) webhook URL. See docs/workspaces.md for setup.");return}
                try WorkspaceStore.save(WorkspaceDestination(provider:provider,label:name,webhook:url));self.respond(id)
            }catch{self.respond(id,error:"Could not save the workspace connection in Keychain.")}
        }
    }
    func shareWorkspace(_ id:String,provider:String,text:String) {
        guard !mailBusy,window.attachedSheet==nil else {respond(id,error:"Finish the current operation first.");return}
        do {
            guard let destination=try WorkspaceStore.load(provider),WorkspaceStore.valid(destination.webhook,provider:provider) else {throw MailFailure(message:"Set up this workspace’s sharing connection first.")}
            let payload=try WorkspaceStore.payload(text,provider:provider)
            let alert=NSAlert();alert.messageText="Share to \(provider.capitalized)?";alert.informativeText="Destination label: \(destination.label)\nThe webhook determines the actual channel/chat. This text will be shared with everyone who can access it. No attachments are included."
            let scroll=NSScrollView(frame:NSRect(x:0,y:0,width:460,height:200));scroll.hasVerticalScroller=true;let preview=NSTextView(frame:scroll.bounds);preview.string=text;preview.isEditable=false;preview.isVerticallyResizable=true;scroll.documentView=preview;alert.accessoryView=scroll;alert.addButton(withTitle:"Share now");alert.addButton(withTitle:"Cancel")
            alert.beginSheetModal(for:window){response in
                guard response == .alertFirstButtonReturn else {self.respond(id,error:"Cancelled.");return}
                self.mailBusy=true
                Task {
                    defer{self.mailBusy=false}
                    var request=URLRequest(url:URL(string:destination.webhook)!);request.httpMethod="POST";request.httpBody=payload;request.setValue("application/json",forHTTPHeaderField:"Content-Type");request.timeoutInterval=30
                    let config=URLSessionConfiguration.ephemeral;config.httpCookieStorage=nil;config.urlCache=nil
                    let session=URLSession(configuration:config,delegate:NoRedirect(),delegateQueue:nil);defer{session.invalidateAndCancel()}
                    do {let (_,response)=try await session.data(for:request);guard let response=response as? HTTPURLResponse,(200...299).contains(response.statusCode) else {throw MailFailure(message:"The workspace did not accept the share.")};self.respond(id,data:["accepted":true])}
                    catch {self.respond(id,error:"Sharing was not confirmed. Check the destination before retrying to avoid a duplicate. The connection may need updating.")}
                }
            }
        }catch{respond(id,error:error.localizedDescription)}
    }
}
