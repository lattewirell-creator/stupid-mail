import Foundation
import Security

struct WorkspaceDestination: Codable {let provider:String;let label:String;let webhook:String}
enum WorkspaceStore {
    static let service="local.emil.thestupidmail.workspaces"
    static func query(_ provider:String)->[String:Any] {[kSecClass as String:kSecClassGenericPassword,kSecAttrService as String:service,kSecAttrAccount as String:provider]}
    static func load(_ provider:String)throws->WorkspaceDestination? {
        var q=query(provider);q[kSecReturnData as String]=true;var result:CFTypeRef?
        let status=SecItemCopyMatching(q as CFDictionary,&result)
        if status == errSecItemNotFound {return nil}
        guard status == errSecSuccess,let data=result as? Data else {throw MailFailure(message:"Unlock Keychain to access your workspace connection.")}
        return try JSONDecoder().decode(WorkspaceDestination.self,from:data)
    }
    static func save(_ destination:WorkspaceDestination)throws {
        let q=query(destination.provider),data=try JSONEncoder().encode(destination)
        let status=SecItemUpdate(q as CFDictionary,[kSecValueData as String:data] as CFDictionary)
        if status == errSecItemNotFound {var add=q;add[kSecValueData as String]=data;add[kSecAttrAccessible as String]=kSecAttrAccessibleWhenUnlockedThisDeviceOnly;guard SecItemAdd(add as CFDictionary,nil)==errSecSuccess else {throw MailFailure(message:"Could not save workspace settings.")}}
        else if status != errSecSuccess {throw MailFailure(message:"Could not update workspace settings.")}
    }
    static func remove(_ provider:String)throws {let status=SecItemDelete(query(provider) as CFDictionary);guard status == errSecSuccess || status == errSecItemNotFound else {throw MailFailure(message:"Could not remove workspace settings.")}}
    static func valid(_ text:String,provider:String)->Bool {
        guard let url=URL(string:text),url.scheme=="https",url.user==nil,url.password==nil,url.fragment==nil,url.port==nil,let host=url.host else {return false}
        if provider=="slack" {return host=="hooks.slack.com" && url.path.hasPrefix("/services/") && url.path.split(separator:"/").count==4}
        if provider=="teams" {return (host.hasSuffix(".logic.azure.com") || host.hasSuffix(".environment.api.powerplatform.com")) && url.path.contains("/workflows/") && URLComponents(url:url,resolvingAgainstBaseURL:false)?.queryItems?.contains(where:{$0.name=="sig" && !($0.value ?? "").isEmpty}) == true}
        return false
    }
    static func payload(_ text:String,provider:String)throws->Data {
        guard !text.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty,text.count<=2500 else {throw MailFailure(message:"Share between 1 and 2,500 characters.")}
        let payload:[String:Any]
        if provider=="slack" {payload=["text":"Email shared from The Stupid Mail","unfurl_links":false,"unfurl_media":false,"blocks":[["type":"section","text":["type":"plain_text","text":text,"emoji":false]]]]}
        else if provider=="teams" {payload=["type":"message","attachments":[["contentType":"application/vnd.microsoft.card.adaptive","contentUrl":NSNull(),"content":["type":"AdaptiveCard","version":"1.2","body":[["type":"TextBlock","text":text,"wrap":true]]]]]]}
        else {throw MailFailure(message:"Choose Slack or Teams.")}
        return try JSONSerialization.data(withJSONObject:payload)
    }
}
