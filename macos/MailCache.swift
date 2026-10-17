import Foundation
import CryptoKit

// Private local app storage, separate from the browser's small localStorage quota.
// Passwords and API keys are never written here.
enum MailCache {
    static func location(account: String) throws -> URL {
        guard account.isEmpty || MailValidation.iCloudAddress(account) else {throw MailFailure(message:"Invalid local account.")}
        let base=try FileManager.default.url(for:.applicationSupportDirectory,in:.userDomainMask,appropriateFor:nil,create:true).appendingPathComponent("The Stupid Mail/Accounts",isDirectory:true)
        try FileManager.default.createDirectory(at:base,withIntermediateDirectories:true,attributes:[.posixPermissions:0o700])
        let hash=SHA256.hash(data:Data((account.isEmpty ? "demo" : account.lowercased()).utf8)).map{String(format:"%02x",$0)}.joined()
        return base.appendingPathComponent(hash+".json")
    }
    static func read(account: String) throws -> Any {
        let url=try location(account:account)
        guard FileManager.default.fileExists(atPath:url.path) else {return NSNull()}
        return try decode(Data(contentsOf:url))
    }
    static func decode(_ data:Data) throws -> [[String:Any]] {
        guard data.count<=80*1024*1024,let records=try JSONSerialization.jsonObject(with:data) as? [[String:Any]] else {throw MailFailure(message:"Invalid local mailbox data.")}
        return records
    }
    static func write(account: String, contents: String) throws {
        let data=Data(contents.utf8)
        _ = try decode(data)
        let url=try location(account:account)
        try data.write(to:url,options:.atomic)
        try FileManager.default.setAttributes([.posixPermissions:0o600],ofItemAtPath:url.path)
    }
}
