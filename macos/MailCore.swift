import Foundation
import Security

struct MailFailure: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}
enum MailProvider: String, Codable, CaseIterable {
    case icloud, gmail, outlook, yahoo, aol, fastmail, zoho, gmx, proton, custom
    var title: String {
        switch self {
        case .icloud: return "iCloud"; case .gmail: return "Gmail"; case .outlook: return "Outlook"; case .yahoo: return "Yahoo"; case .aol: return "AOL"
        case .fastmail: return "Fastmail"; case .zoho: return "Zoho"; case .gmx: return "GMX"; case .proton: return "Proton Mail"; case .custom: return "Other"
        }
    }
    /// What the connect menu shows.
    var menuTitle: String {
        switch self {
        case .icloud: return "iCloud — Apple app password"; case .gmail: return "Gmail — Google app password"; case .outlook: return "Outlook — Microsoft sign-in"
        case .yahoo: return "Yahoo — app password"; case .aol: return "AOL — app password"; case .fastmail: return "Fastmail — app password"
        case .zoho: return "Zoho — app-specific password"; case .gmx: return "GMX — password or app password"; case .proton: return "Proton Mail — through Proton Mail Bridge"
        case .custom: return "Other mail service — enter its servers"
        }
    }
    var imapHost: String { MailServer.preset(self, email: "")?.imapHost ?? "" }
    var smtpHost: String { MailServer.preset(self, email: "")?.smtpHost ?? "" }
    /// Where to create the app password (or set up the Bridge).
    var passwordPage: String? {
        switch self {
        case .icloud: return "https://account.apple.com/"; case .gmail: return "https://myaccount.google.com/apppasswords"; case .yahoo: return "https://login.yahoo.com/account/security"
        case .aol: return "https://login.aol.com/account/security"; case .fastmail: return "https://app.fastmail.com/settings/security/apps"; case .zoho: return "https://accounts.zoho.com/home#security/app_password"
        case .gmx: return "https://www.gmx.com/"; case .proton: return "https://proton.me/mail/bridge"; default: return nil
        }
    }
    var help: String {
        switch self {
        case .icloud: return "Use your primary iCloud address and an Apple app-specific password from account.apple.com."
        case .gmail: return "Use your full Gmail or Google Workspace address and a 16-letter Google app password. Enable 2-Step Verification, then create one at myaccount.google.com/apppasswords. Some work/school or protected accounts do not allow app passwords."
        case .yahoo: return "Use your Yahoo address and a 16-letter app password: Yahoo Account → Account security → Generate app password."
        case .aol: return "Use your AOL address and a 16-letter app password: AOL Account → Account security → Generate app password."
        case .fastmail: return "Use your Fastmail address and an app password with Mail (IMAP/SMTP) access: Settings → Privacy & Security → App passwords."
        case .zoho: return "Use your Zoho Mail address and an app-specific password (Zoho Accounts → Security → App passwords). IMAP access must be on in Zoho Mail settings. For zoho.eu or other regions, choose Other mail service."
        case .gmx: return "Turn on POP3 & IMAP access in GMX settings first. Use your GMX address and, if two-factor authentication is on, an app password."
        case .proton: return "Proton only lets other apps in through Proton Mail Bridge (paid Proton plans). Install Bridge, sign in, set its IMAP and SMTP connection mode to SSL (Bridge → Settings → Advanced), and use the Bridge password it shows, not your Proton password. Keep Bridge running."
        case .custom: return "Enter your provider's IMAP (incoming) and SMTP (outgoing) servers. Connections always use encryption (TLS); IMAP needs SSL/TLS, usually port 993."
        case .outlook: return ""
        }
    }
    func validPassword(_ value: String) -> Bool {
        switch self {
        case .icloud, .gmail, .yahoo, .aol: return MailValidation.appPassword(value)
        case .fastmail: return value.range(of: #"^[a-z0-9]{16}$"#, options: .regularExpression) != nil
        case .zoho: return value.range(of: #"^[A-Za-z0-9]{12,16}$"#, options: .regularExpression) != nil
        default: return (8...256).contains(value.count) && !value.unicodeScalars.contains(where: { $0.value < 32 || $0.value == 127 })
        }
    }
}
/// Where a password account's mail lives. Presets are fixed per provider; "Other" uses what you entered.
struct MailServer: Codable, Equatable {
    let imapHost: String; let imapPort: Int; let smtpHost: String; let smtpPort: Int
    var smtpImplicitTLS: Bool   // SSL/TLS from the start (465, Proton Bridge in SSL mode); otherwise STARTTLS
    static func preset(_ p: MailProvider, email: String) -> MailServer? {
        let domain = email.components(separatedBy: "@").last?.lowercased() ?? ""
        switch p {
        case .icloud: return MailServer(imapHost: "imap.mail.me.com", imapPort: 993, smtpHost: "smtp.mail.me.com", smtpPort: 587, smtpImplicitTLS: false)
        case .gmail: return MailServer(imapHost: "imap.gmail.com", imapPort: 993, smtpHost: "smtp.gmail.com", smtpPort: 587, smtpImplicitTLS: false)
        case .yahoo: return MailServer(imapHost: "imap.mail.yahoo.com", imapPort: 993, smtpHost: "smtp.mail.yahoo.com", smtpPort: 465, smtpImplicitTLS: true)
        case .aol: return MailServer(imapHost: "imap.aol.com", imapPort: 993, smtpHost: "smtp.aol.com", smtpPort: 465, smtpImplicitTLS: true)
        case .fastmail: return MailServer(imapHost: "imap.fastmail.com", imapPort: 993, smtpHost: "smtp.fastmail.com", smtpPort: 465, smtpImplicitTLS: true)
        case .zoho: return MailServer(imapHost: "imap.zoho.com", imapPort: 993, smtpHost: "smtp.zoho.com", smtpPort: 465, smtpImplicitTLS: true)
        case .gmx:
            let german = ["gmx.net", "gmx.de", "gmx.at", "gmx.ch"].contains(domain)
            return MailServer(imapHost: german ? "imap.gmx.net" : "imap.gmx.com", imapPort: 993, smtpHost: german ? "mail.gmx.net" : "mail.gmx.com", smtpPort: 587, smtpImplicitTLS: false)
        case .proton: return MailServer(imapHost: "127.0.0.1", imapPort: 1143, smtpHost: "127.0.0.1", smtpPort: 1025, smtpImplicitTLS: true)
        case .outlook, .custom: return nil
        }
    }
    /// A server you typed in: a plain host name and a normal port, nothing else.
    static func valid(host: String, port: Int) -> Bool {
        (1...65535).contains(port) && host.utf8.count <= 253 && host.range(of: #"^[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?(\.[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?)+$"#, options: .regularExpression) != nil
    }
}
struct MailCredentials: Codable {
    let email: String
    let password: String
    var provider: MailProvider? = nil
    var clientID: String? = nil
    var refreshToken: String? = nil
    var expiresAt: Date? = nil
    var server: MailServer? = nil   // only for "Other"
    var kind: MailProvider { provider ?? .icloud }
    var mailServer: MailServer? { server ?? MailServer.preset(kind, email: email) }
}
struct MailRecord: Codable {
    let id: String
    let from: String
    let email: String
    let subject: String
    let body: String
    let time: String
    let unread: Bool
    let star: Bool
    let newsletter: Bool
    let attachmentCount: Int
    let messageID: String
    let inReplyTo: String
}

enum MailKeychain {
    static let service = "local.emil.thestupidmail.icloud"
    static func save(_ credentials: MailCredentials) throws {
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service, kSecAttrAccount as String: "primary"]
        let data = try JSONEncoder().encode(credentials)
        let update = SecItemUpdate(query as CFDictionary, [kSecValueData as String: data] as CFDictionary)
        if update == errSecItemNotFound {
            var add = query
            add[kSecValueData as String] = data
            add[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
            guard SecItemAdd(add as CFDictionary, nil) == errSecSuccess else { throw MailFailure(message: "macOS could not save the account in Keychain. No password was saved.") }
        } else if update != errSecSuccess { throw MailFailure(message: "macOS could not update this account in Keychain.") }
    }
    static func load() throws -> MailCredentials? {
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service, kSecAttrAccount as String: "primary", kSecReturnData as String: true, kSecMatchLimit as String: kSecMatchLimitOne]
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess, let data = item as? Data else { throw MailFailure(message: "Unlock your login Keychain to connect your saved account.") }
        return try JSONDecoder().decode(MailCredentials.self, from: data)
    }
    static func remove() throws {
        let status = SecItemDelete([kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service, kSecAttrAccount as String: "primary"] as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else { throw MailFailure(message: "macOS could not remove the saved password. Try again with your Keychain unlocked.") }
    }
}

enum MailValidation {
    static func address(_ value: String) -> Bool {
        value.utf8.count <= 254 && !value.unicodeScalars.contains(where: { $0.value < 33 || $0.value == 127 }) && value.range(of: #"^[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]+@[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?\.[A-Za-z]{2,}$"#, options: .regularExpression) != nil
    }
    static func iCloudAddress(_ value: String) -> Bool {
        address(value) && ["icloud.com", "me.com", "mac.com"].contains(value.components(separatedBy: "@").last!.lowercased())
    }
    static func appPassword(_ value: String) -> Bool { value.range(of: #"^[a-z]{4}(-?[a-z]{4}){3}$"#, options: .regularExpression) != nil }
    static func quote(_ value: String) throws -> String {
        guard !value.unicodeScalars.contains(where: { $0.value < 32 || $0.value == 127 }) else { throw MailFailure(message: "Invalid control character in account or message settings.") }
        return "\"" + value.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\"") + "\""
    }
}

// Only fixed provider endpoints are accepted. Credentials go through an anonymous pipe,
// never through process arguments, environment variables, logs, or temporary files.
final class MailTransport {
    func perform(credentials: MailCredentials, command: String? = nil, mailbox: Bool = true, sending: (recipient: String, file: URL)? = nil) throws -> Data {
        guard credentials.kind != .outlook else {throw MailFailure(message:"Outlook requires Microsoft sign-in.")}
        let task = Process(), input = Pipe(), output = Pipe()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/curl")
        task.arguments = ["--disable", "--config", "-"]
        task.standardInput = input; task.standardOutput = output; task.standardError = FileHandle.nullDevice
        task.environment = ["PATH": "/usr/bin:/bin", "HOME": NSHomeDirectory(), "LANG": "en_US.UTF-8"]
        guard let server = credentials.mailServer else {throw MailFailure(message:"Connect your mail account again.")}
        let url = sending == nil ? "imaps://\(server.imapHost):\(server.imapPort)/" + (mailbox ? "INBOX" : "") : (server.smtpImplicitTLS ? "smtps" : "smtp") + "://\(server.smtpHost):\(server.smtpPort)"
        var config = "silent\nshow-error\nconnect-timeout = 15\nmax-time = 90\nssl-reqd\ntlsv1.2\nproto = \"=imaps,smtp,smtps\"\n"
        config += "url = \(try MailValidation.quote(url))\nuser = \(try MailValidation.quote(credentials.email + ":" + credentials.password))\n"
        if let command { config += "request = \(try MailValidation.quote(command))\n" }
        if let sending {
            config += "mail-from = \(try MailValidation.quote(credentials.email))\nmail-rcpt = \(try MailValidation.quote(sending.recipient))\nupload-file = \(try MailValidation.quote(sending.file.path))\n"
        }
        try task.run()
        try input.fileHandleForWriting.write(contentsOf: Data(config.utf8)); try input.fileHandleForWriting.close()
        var data = Data()
        while let chunk = try output.fileHandleForReading.read(upToCount: 65536), !chunk.isEmpty {
            guard data.count + chunk.count <= 16 * 1024 * 1024 else { task.terminate(); throw MailFailure(message: "The mail response was too large. Please try a smaller inbox.") }
            data.append(chunk)
        }
        task.waitUntilExit()
        guard task.terminationStatus == 0 else {
            if task.terminationStatus == 67 { throw MailFailure(message: "The provider rejected sign-in. Check the email address and generate a new app password.") }
            if sending != nil { throw MailFailure(message: "Sending was not confirmed. Your draft is safe. Check your provider’s Sent folder before retrying, because the server may have accepted it.") }
            throw MailFailure(message: "Could not reach your mail provider securely. Check your internet connection and try again. Your mail has not been changed.")
        }
        return data
    }
}

struct FetchLiteral { let prefix: String; let data: Data }
enum IMAPParser {
    static func literals(_ data: Data) throws -> [FetchLiteral] {
        var cursor = data.startIndex, result: [FetchLiteral] = []
        let crlf = Data([13,10])
        while cursor < data.endIndex, let ending = data.range(of: crlf, in: cursor..<data.endIndex) {
            let line = String(decoding: data[cursor..<ending.lowerBound], as: UTF8.self)
            cursor = ending.upperBound
            guard let range = line.range(of: #"\{[0-9]+\}$"#, options: .regularExpression) else { continue }
            let token = line[range].dropFirst().dropLast()
            guard let count = Int(token), count <= 2 * 1024 * 1024, count <= data.endIndex - cursor else { throw MailFailure(message: "iCloud returned an incomplete message. Please refresh.") }
            let after = cursor + count
            let tailEnd = data.range(of: crlf, in: after..<data.endIndex)?.lowerBound ?? after
            let tail = String(decoding: data[after..<tailEnd], as: UTF8.self)
            result.append(FetchLiteral(prefix: line + " " + tail, data: Data(data[cursor..<(cursor+count)])))
            cursor += count
        }
        return result
    }
    static func capture(_ pattern: String, _ value: String, group: Int = 1) -> String? {
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]), let match = regex.firstMatch(in: value, range: NSRange(value.startIndex..., in: value)), group < match.numberOfRanges, let range = Range(match.range(at: group), in: value) else { return nil }
        return String(value[range])
    }
    static func search(_ data: Data) -> [Int] {
        let text = String(decoding: data, as: UTF8.self)
        guard let line = text.components(separatedBy: .newlines).first(where: { $0.hasPrefix("* SEARCH") }) else { return [] }
        return line.split(separator: " ").dropFirst(2).compactMap { Int($0.trimmingCharacters(in: .whitespacesAndNewlines)) }.filter { $0 > 0 }.sorted()
    }
}

// MIME is converted to inert text; email HTML is never loaded into the web view.
// No remote images, scripts, links, or tracking pixels are fetched.
enum MIMEParser {
    static func split(_ data: Data) -> ([String: String], Data) {
        let separator = data.range(of: Data("\r\n\r\n".utf8)) ?? data.range(of: Data("\n\n".utf8))
        guard let separator else { return ([:], data) }
        let raw = String(decoding: data[..<separator.lowerBound], as: UTF8.self)
            .replacingOccurrences(of: "\r\n", with: "\n")
            .replacingOccurrences(of: "\n[ \t]+", with: " ", options: .regularExpression)
        var headers: [String: String] = [:]
        for line in raw.components(separatedBy: "\n") {
            guard let colon = line.firstIndex(of: ":") else { continue }
            headers[String(line[..<colon]).lowercased()] = String(line[line.index(after: colon)...]).trimmingCharacters(in: .whitespaces)
        }
        return (headers, Data(data[separator.upperBound...]))
    }
    static func quotedPrintable(_ data: Data, header: Bool = false) -> Data {
        let bytes = Array(data); var out = Data(); var i = 0
        while i < bytes.count {
            if header && bytes[i] == 95 { out.append(32); i += 1; continue }
            if bytes[i] == 61 {
                if i+1 < bytes.count && bytes[i+1] == 10 { i += 2; continue }
                if i+2 < bytes.count && bytes[i+1] == 13 && bytes[i+2] == 10 { i += 3; continue }
                if i+2 < bytes.count, let byte = UInt8(String(bytes: bytes[(i+1)...(i+2)], encoding: .ascii) ?? "", radix: 16) { out.append(byte); i += 3; continue }
            }
            out.append(bytes[i]); i += 1
        }
        return out
    }
    static func decode(_ data: Data, charset: String = "utf-8") -> String {
        let charset = charset.lowercased()
        let encoding: String.Encoding = charset == "iso-8859-1" ? .isoLatin1 : charset == "windows-1252" ? .windowsCP1252 : .utf8
        return String(data: data, encoding: encoding) ?? String(data: data, encoding: .windowsCP1252) ?? String(decoding: data, as: UTF8.self)
    }
    static func header(_ value: String) -> String {
        let value = value.replacingOccurrences(of: #"(\?=)[ \t]+(?==\?)"#, with: "$1", options: .regularExpression)
        let pattern = #"=\?([^?]+)\?([bBqQ])\?([^?]*)\?="#
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return value }
        var result = value
        for match in regex.matches(in: value, range: NSRange(value.startIndex..., in: value)).reversed() {
            let ns = value as NSString
            let bytes = ns.substring(with: match.range(at: 3))
            let data = ns.substring(with: match.range(at: 2)).lowercased() == "b" ? Data(base64Encoded: bytes) ?? Data() : quotedPrintable(Data(bytes.utf8), header: true)
            let text = decode(data, charset: ns.substring(with: match.range(at: 1)))
            if let range = Range(match.range, in: result) { result.replaceSubrange(range, with: text) }
        }
        return result
    }
    static func parameter(_ name: String, _ value: String) -> String? {
        IMAPParser.capture("(?:^|;)\\s*" + name + "\\s*=\\s*\"([^\"]+)\"", value) ?? IMAPParser.capture("(?:^|;)\\s*" + name + "\\s*=\\s*([^;\\s]+)", value)
    }
    static func plainHTML(_ html: String) -> String {
        var text = html.replacingOccurrences(of: #"(?is)<(script|style|head)\b[^>]*>.*?</\1\s*>"#, with: "", options: .regularExpression)
        text = text.replacingOccurrences(of: #"(?i)<(?:br\s*/?|/p|/div|/tr|/h[1-6])\s*>"#, with: "\n", options: .regularExpression)
        text = text.replacingOccurrences(of: #"<[^>]*>"#, with: "", options: .regularExpression)
        for (entity, replacement) in [("&nbsp;", " "), ("&lt;", "<"), ("&gt;", ">"), ("&quot;", "\""), ("&#39;", "'"), ("&apos;", "'"), ("&amp;", "&")] { text = text.replacingOccurrences(of: entity, with: replacement) }
        if let regex = try? NSRegularExpression(pattern: "&#(x[0-9a-fA-F]+|[0-9]+);") {
            for match in regex.matches(in: text, range: NSRange(text.startIndex..., in: text)).reversed() {
                let token = (text as NSString).substring(with: match.range(at: 1))
                let n = token.hasPrefix("x") ? UInt32(token.dropFirst(), radix: 16) : UInt32(token)
                if let n, let scalar = UnicodeScalar(n), let range = Range(match.range, in: text) { text.replaceSubrange(range, with: String(scalar)) }
            }
        }
        return text.replacingOccurrences(of: "\n{3,}", with: "\n\n", options: .regularExpression).trimmingCharacters(in: .whitespacesAndNewlines)
    }
    struct Content { var plain: String = ""; var html: String = ""; var attachments: Int = 0 }
    static func content(_ data: Data, depth: Int = 0) -> Content {
        guard depth < 12 else { return Content() }
        let (headers, body) = split(data)
        let type = headers["content-type"] ?? "text/plain; charset=utf-8"
        if (headers["content-disposition"] ?? "").lowercased().contains("attachment") || parameter("name", type) != nil { return Content(attachments: 1) }
        if type.lowercased().hasPrefix("multipart/"), let boundary = parameter("boundary", type), boundary.count < 200 {
            // Latin-1 preserves all bytes while splitting the ASCII MIME delimiters.
            let raw = String(data: body, encoding: .isoLatin1) ?? ""
            var merged = Content()
            for part in raw.components(separatedBy: "--" + boundary).dropFirst() {
                if part.hasPrefix("--") { break }
                let trimmed = part.trimmingCharacters(in: .newlines)
                guard !trimmed.isEmpty, let bytes = trimmed.data(using: .isoLatin1) else { continue }
                let item = content(bytes, depth: depth+1)
                if !item.plain.isEmpty { merged.plain += (merged.plain.isEmpty ? "" : "\n\n") + item.plain }
                if !item.html.isEmpty { merged.html += (merged.html.isEmpty ? "" : "\n\n") + item.html }
                merged.attachments += item.attachments
            }
            return merged
        }
        guard type.lowercased().hasPrefix("text/plain") || type.lowercased().hasPrefix("text/html") else { return Content(attachments: body.isEmpty ? 0 : 1) }
        let transfer = headers["content-transfer-encoding"]?.lowercased()
        let decoded = transfer == "base64" ? Data(base64Encoded: body, options: .ignoreUnknownCharacters) ?? Data() : transfer == "quoted-printable" ? quotedPrintable(body) : body
        let text = decode(decoded, charset: parameter("charset", type) ?? "utf-8").trimmingCharacters(in: .whitespacesAndNewlines)
        return type.lowercased().hasPrefix("text/html") ? Content(html: plainHTML(text)) : Content(plain: text)
    }
    static func record(_ data: Data, id: String, flags: String, limited: Bool = false) -> MailRecord {
        let (headers, _) = split(data), content = limited ? Content() : content(data)
        let from = header(headers["from"] ?? "Unknown sender")
        let email = IMAPParser.capture("<([^>]+)>", from) ?? (MailValidation.address(from) ? from : "")
        let name = from.components(separatedBy: "<").first?.trimmingCharacters(in: CharacterSet(charactersIn: " \"")) ?? from
        let body = limited ? "This message is too large for the current sync. Open it in your provider’s mail app to read the full message and attachments." : !content.plain.isEmpty ? content.plain : !content.html.isEmpty ? content.html : "No readable text. This message may contain only attachments or encrypted content."
        return MailRecord(id: id, from: name.isEmpty ? email : name, email: email, subject: header(headers["subject"] ?? "(No subject)"), body: body, time: headers["date"] ?? "", unread: !flags.lowercased().contains("\\seen"), star: flags.lowercased().contains("\\flagged"), newsletter: headers["list-id"] != nil || headers["list-unsubscribe"] != nil, attachmentCount: content.attachments, messageID: headers["message-id"] ?? "", inReplyTo: headers["in-reply-to"] ?? "")
    }
}

final class ICloudMail {
    let transport = MailTransport()
    func sync(_ credentials: MailCredentials) throws -> [MailRecord] {
        guard credentials.kind != .outlook else {throw MailFailure(message:"Outlook requires Microsoft sign-in.")}
        guard let server = credentials.mailServer else { throw MailFailure(message: "Connect your mail account again.") }
        let connection = try IMAPConnection(channel: TLSMailChannel(host: server.imapHost, port: server.imapPort))
        defer { connection.close() }
        try connection.authenticate(credentials)
        let status = try connection.command("EXAMINE INBOX")
        guard let validity = IMAPParser.capture("UIDVALIDITY\\s+([0-9]+)", String(decoding: status, as: UTF8.self)) else { throw MailFailure(message: "The mail server did not return a valid mailbox identifier. Please reconnect.") }
        let search = try connection.command("UID SEARCH ALL")
        let ids = Array(IMAPParser.search(search).suffix(40))
        if ids.isEmpty { return [] }
        let set = ids.map(String.init).joined(separator: ",")
        let headers = try connection.command("UID FETCH \(set) (UID FLAGS RFC822.SIZE BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE MESSAGE-ID IN-REPLY-TO LIST-ID LIST-UNSUBSCRIBE)])")
        var records: [Int: MailRecord] = [:], bodyIDs: [Int] = [], total = 0
        for literal in try IMAPParser.literals(headers).reversed() {
            guard let uid = IMAPParser.capture("UID\\s+([0-9]+)", literal.prefix).flatMap(Int.init) else { continue }
            let size = IMAPParser.capture("RFC822.SIZE\\s+([0-9]+)", literal.prefix).flatMap(Int.init) ?? Int.max
            records[uid] = MIMEParser.record(literal.data, id: "\(credentials.kind.rawValue):\(validity):\(uid)", flags: literal.prefix, limited: true)
            if size <= 1024*1024 && total + size <= 10*1024*1024 { bodyIDs.append(uid); total += size }
        }
        if !bodyIDs.isEmpty {
            let full = try connection.command("UID FETCH \(bodyIDs.map(String.init).joined(separator: ",")) (UID FLAGS BODY.PEEK[])")
            for literal in try IMAPParser.literals(full) {
                guard let uid = IMAPParser.capture("UID\\s+([0-9]+)", literal.prefix).flatMap(Int.init) else { continue }
                records[uid] = MIMEParser.record(literal.data, id: "\(credentials.kind.rawValue):\(validity):\(uid)", flags: literal.prefix)
            }
        }
        guard records.count == ids.count else { throw MailFailure(message: "The inbox changed while syncing. Refresh again to get a complete snapshot.") }
        return ids.reversed().compactMap { records[$0] }
    }
    static func outgoing(from: String, to: String, subject: String, body: String, replyTo: String?) throws -> Data {
        guard MailValidation.address(from), MailValidation.address(to), !subject.unicodeScalars.contains(where: { $0.value < 32 || $0.value == 127 }), subject.utf8.count <= 900, body.utf8.count <= 1024*1024 else { throw MailFailure(message: "Use one valid recipient, a subject under 900 bytes, and a message under 1 MB.") }
        let id = "<\(UUID().uuidString.lowercased())@stupidmail.local>"
        let formatter = DateFormatter(); formatter.locale = Locale(identifier: "en_US_POSIX"); formatter.dateFormat = "EEE, dd MMM yyyy HH:mm:ss Z"
        // Fold Unicode headers at encoded-word boundaries (RFC 2047).
        var words: [String] = [], segment = ""
        for character in subject {
            if (segment + String(character)).utf8.count > 39 && !segment.isEmpty { words.append("=?UTF-8?B?\(Data(segment.utf8).base64EncodedString())?="); segment = "" }
            segment.append(character)
        }
        if !segment.isEmpty { words.append("=?UTF-8?B?\(Data(segment.utf8).base64EncodedString())?=") }
        var headers = "From: \(from)\r\nTo: \(to)\r\nSubject: \(words.joined(separator: "\r\n "))\r\nDate: \(formatter.string(from: Date()))\r\nMessage-ID: \(id)\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n"
        if let replyTo, replyTo.range(of: #"^<[^<>\s]+@[^<>\s]+>$"#, options: .regularExpression) != nil, replyTo.count < 250 { headers += "In-Reply-To: \(replyTo)\r\nReferences: \(replyTo)\r\n" }
        let normalized = body.replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\n", with: "\r\n")
        let encoded = Data(normalized.utf8).base64EncodedString(options: [.lineLength76Characters, .endLineWithCarriageReturn, .endLineWithLineFeed])
        return Data((headers + "\r\n" + encoded + "\r\n").utf8)
    }
    func send(_ credentials: MailCredentials, to: String, subject: String, body: String, replyTo: String?) throws -> String {
        let data = try Self.outgoing(from: credentials.email, to: to, subject: subject, body: body, replyTo: replyTo)
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("stupid-mail-" + UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: false, attributes: [.posixPermissions: 0o700])
        defer { try? FileManager.default.removeItem(at: dir) }
        let file = dir.appendingPathComponent("message.eml")
        guard FileManager.default.createFile(atPath: file.path, contents: data, attributes: [.posixPermissions: 0o600]) else { throw MailFailure(message: "Could not prepare this message for sending.") }
        _ = try transport.perform(credentials: credentials, sending: (to, file))
        return MIMEParser.split(data).0["message-id"] ?? ""
    }
}
