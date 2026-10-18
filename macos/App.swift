import AppKit
import EventKit
import Sparkle
import UniformTypeIdentifiers
@preconcurrency import WebKit

final class AppDelegate: NSObject, NSApplicationDelegate, WKNavigationDelegate, WKUIDelegate, NSMenuItemValidation {
    let cacheQueue = DispatchQueue(label: "StupidMail.LocalStorage", qos: .userInitiated)
    let eventStore=EKEventStore()
    var workspaceWindows:[String:WorkspaceWindow] = [:]
    lazy var updaterController = SPUStandardUpdaterController(startingUpdater: false, updaterDelegate: nil, userDriverDelegate: nil)
    var mailBusy = false
    var buddyBusy = false
    var window: NSWindow!
    var webView: WKWebView!

    func applicationDidFinishLaunching(_ notification: Notification) {
        let appMenu = NSMenu()
        let appItem = NSMenuItem()
        appMenu.addItem(appItem)
        let applicationMenu = NSMenu()
        applicationMenu.addItem(withTitle: "About The Stupid Mail", action: #selector(showAbout), keyEquivalent: "")
        let updateItem = applicationMenu.addItem(withTitle: "Check for Updates…", action: #selector(SPUStandardUpdaterController.checkForUpdates(_:)), keyEquivalent: "")
        updateItem.target = updaterController
        applicationMenu.addItem(withTitle: "Automatic Updates", action: #selector(toggleAutomaticUpdates), keyEquivalent: "")
        applicationMenu.addItem(.separator())
        applicationMenu.addItem(withTitle: "Hide The Stupid Mail", action: #selector(NSApplication.hide(_:)), keyEquivalent: "h")
        applicationMenu.addItem(.separator())
        applicationMenu.addItem(withTitle: "Quit The Stupid Mail", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        appItem.submenu = applicationMenu
        let fileItem = NSMenuItem(title: "File", action: nil, keyEquivalent: "")
        let fileMenu = NSMenu(title: "File")
        fileMenu.addItem(withTitle: "Connect Mail Account…", action: #selector(accountMenu), keyEquivalent: ",")
        fileMenu.addItem(withTitle: "New Message", action: #selector(newMessage), keyEquivalent: "n")
        fileMenu.addItem(withTitle: "Close Window", action: #selector(NSWindow.performClose(_:)), keyEquivalent: "w")
        fileItem.submenu = fileMenu
        appMenu.addItem(fileItem)
        let editItem = NSMenuItem(title: "Edit", action: nil, keyEquivalent: "")
        let editMenu = NSMenu(title: "Edit")
        for (name, action, key) in [("Undo", "undo:", "z"), ("Redo", "redo:", "Z"), ("Cut", "cut:", "x"), ("Copy", "copy:", "c"), ("Paste", "paste:", "v"), ("Select All", "selectAll:", "a")] {
            editMenu.addItem(withTitle: name, action: Selector(action), keyEquivalent: key)
        }
        editMenu.addItem(.separator())
        editMenu.addItem(withTitle: "Search Mail", action: #selector(searchMail), keyEquivalent: "f")
        editItem.submenu = editMenu
        appMenu.addItem(editItem)
        let windowItem = NSMenuItem(title: "Window", action: nil, keyEquivalent: "")
        let windowMenu = NSMenu(title: "Window")
        windowMenu.addItem(withTitle: "Minimize", action: #selector(NSWindow.performMiniaturize(_:)), keyEquivalent: "m")
        windowMenu.addItem(withTitle: "Zoom", action: #selector(NSWindow.performZoom(_:)), keyEquivalent: "")
        windowItem.submenu = windowMenu
        appMenu.addItem(windowItem)
        NSApp.mainMenu = appMenu
        NSApp.windowsMenu = windowMenu

        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .default()
        configuration.userContentController.add(self, name: "mail")
        let nativeStyle = "const s=document.createElement('style');s.textContent='.window{min-height:100vh}';document.head.appendChild(s);"
        configuration.userContentController.addUserScript(WKUserScript(source: nativeStyle, injectionTime: .atDocumentEnd, forMainFrameOnly: true))
        webView = WKWebView(frame: .zero, configuration: configuration)
        webView.navigationDelegate = self
        webView.uiDelegate = self
        webView.setValue(false, forKey: "drawsBackground")
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1180, height: 820), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        window.title = "The Stupid Mail"
        window.minSize = NSSize(width: 420, height: 550)
        window.isReleasedWhenClosed = false
        window.backgroundColor = .windowBackgroundColor
        window.contentView = webView
        window.setFrameAutosaveName("StupidMailMainWindow")
        if !window.setFrameUsingName("StupidMailMainWindow") { window.center() }
        guard let resources = Bundle.main.resourceURL else { NSApp.terminate(nil); return }
        let website = resources.appendingPathComponent("Web")
        webView.loadFileURL(website.appendingPathComponent("index.html"), allowingReadAccessTo: website)
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        updaterController.startUpdater()
    }

    @objc func toggleAutomaticUpdates() {
        let updater = updaterController.updater
        let enabled = !updater.automaticallyChecksForUpdates
        updater.automaticallyDownloadsUpdates = enabled
        updater.automaticallyChecksForUpdates = enabled
    }
    func validateMenuItem(_ menuItem: NSMenuItem) -> Bool {
        if menuItem.action == #selector(toggleAutomaticUpdates) {
            menuItem.state = updaterController.updater.automaticallyChecksForUpdates ? .on : .off
        }
        return true
    }

    @objc func newMessage() { webView.evaluateJavaScript("if(!document.querySelector('dialog[open]'))document.getElementById('compose').click()", completionHandler: nil) }
    @objc func searchMail() { webView.evaluateJavaScript("if(!document.querySelector('dialog[open]'))document.getElementById('search').focus()", completionHandler: nil) }
    @objc func showAbout() {
        NSApp.orderFrontStandardAboutPanel(options: [.applicationName: "The Stupid Mail", .applicationVersion: Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "2.2", .credits: NSAttributedString(string: "Quiet Inbox • Follow-up Radar • AI Buddy\n\niCloud, Gmail, and Outlook inbox and sending. Sorting, follow-up reminders, and drafts stay on this Mac. Latest 40 inbox messages. Attachments are not supported yet.")])
    }
    func webView(_ webView: WKWebView, runOpenPanelWith parameters: WKOpenPanelParameters, initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping ([URL]?) -> Void) {
        guard frame.isMainFrame, frame.request.url?.standardizedFileURL == Bundle.main.resourceURL?.appendingPathComponent("Web/index.html").standardizedFileURL, window.attachedSheet == nil else { completionHandler(nil); return }
        let picker = NSOpenPanel()
        picker.canChooseDirectories = false; picker.canChooseFiles = true; picker.allowsMultipleSelection = false
        picker.allowedContentTypes = [.png, .jpeg, .webP]
        picker.beginSheetModal(for: window) { result in completionHandler(result == .OK ? picker.urls : nil) }
    }
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        if mailBusy || buddyBusy {
            nativeNotice("Please wait for the current mail or Buddy request to finish before quitting. This avoids interrupting an email that may be sending.")
            return .terminateCancel
        }
        webView.callAsyncJavaScript("return window.flushDraft ? await window.flushDraft() : true",arguments:[:],in:nil,in:.page) { result in
            switch result {
            case .success(let value):
                let saved = value as? Bool ?? false
                sender.reply(toApplicationShouldTerminate:saved)
                if !saved {self.nativeNotice("Your latest draft could not be saved. Keep the app open and check free disk space.")}
            case .failure:
                sender.reply(toApplicationShouldTerminate:false)
                self.nativeNotice("Could not confirm that your draft was saved. Please save it before quitting.")
            }
        }
        return .terminateLater
    }
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if !flag { window.makeKeyAndOrderFront(nil) }; return true
    }
    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
        guard let url = navigationAction.request.url else { decisionHandler(.cancel); return }
        if url.standardizedFileURL == Bundle.main.resourceURL?.appendingPathComponent("Web/index.html").standardizedFileURL || url.absoluteString == "about:blank" { decisionHandler(.allow) }
        else { decisionHandler(.cancel) }
    }
}

@main
struct StupidMailApp {
    static func main() {
        let app = NSApplication.shared
        let delegate = AppDelegate()
        app.delegate = delegate
        app.setActivationPolicy(.regular)
        app.run()
    }
}
