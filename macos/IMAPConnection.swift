import Foundation
import Network

protocol IMAPChannel: AnyObject {
    func send(_ data: Data) throws
    func line() throws -> Data
    func bytes(_ count: Int) throws -> Data
    func close()
}
final class TLSMailChannel: IMAPChannel {
    private let connection: NWConnection
    private let queue = DispatchQueue(label: "StupidMail.iCloud.TLS")
    private var buffer = Data()
    init() throws {
        // Network.framework validates Apple's server certificate and hostname.
        connection = NWConnection(host: "imap.mail.me.com", port: 993, using: .tls)
        let ready = DispatchSemaphore(value: 0), lock = NSLock()
        var complete = false, connected = false
        connection.stateUpdateHandler = { state in
            lock.lock(); defer { lock.unlock() }
            guard !complete else { return }
            switch state {
            case .ready: connected = true; complete = true; ready.signal()
            case .failed, .cancelled: complete = true; ready.signal()
            default: break
            }
        }
        connection.start(queue: queue)
        guard ready.wait(timeout: .now()+20) == .success else { connection.cancel(); throw MailFailure(message: "iCloud did not respond. Check your internet connection and try again.") }
        lock.lock(); let success = connected; lock.unlock()
        guard success else { connection.cancel(); throw MailFailure(message: "Could not establish a secure connection to iCloud.") }
    }
    func send(_ data: Data) throws {
        let done=DispatchSemaphore(value:0);var failed=false
        connection.send(content:data,completion:.contentProcessed{error in failed=error != nil;done.signal()})
        guard done.wait(timeout:.now()+20) == .success else {close();throw MailFailure(message:"The iCloud connection timed out.")}
        guard !failed else {throw MailFailure(message:"The connection to iCloud was interrupted.")}
    }
    private func receive() throws {
        let done=DispatchSemaphore(value:0);var received:Data?,ended=false
        connection.receive(minimumIncompleteLength:1,maximumLength:65536){data,_,complete,error in received=data;ended=complete||error != nil;done.signal()}
        guard done.wait(timeout:.now()+30) == .success else {close();throw MailFailure(message:"iCloud took too long to reply. Refresh to try again.")}
        guard let received,!received.isEmpty else {throw MailFailure(message:ended ? "iCloud closed the connection. Please refresh." : "iCloud returned no data.")}
        buffer.append(received)
    }
    func line() throws -> Data {
        while true {
            if let end=buffer.range(of:Data([13,10])) {let line=Data(buffer[..<end.upperBound]);buffer.removeSubrange(..<end.upperBound);return line}
            guard buffer.count<2*1024*1024 else {throw MailFailure(message:"iCloud returned an unusually long response.")}
            try receive()
        }
    }
    func bytes(_ count:Int) throws -> Data {
        guard count>=0,count<=2*1024*1024 else {throw MailFailure(message:"This message is too large to load.")}
        while buffer.count<count {try receive()}
        let data=Data(buffer.prefix(count));buffer.removeFirst(count);return data
    }
    func close(){connection.cancel()}
    deinit{close()}
}
final class IMAPConnection {
    let channel:IMAPChannel
    var sequence=0
    init(channel:IMAPChannel) throws {
        self.channel=channel
        let greeting=String(decoding:try channel.line(),as:UTF8.self)
        guard greeting.hasPrefix("* OK") else {channel.close();throw MailFailure(message:"iCloud did not accept the connection.")}
    }
    func command(_ command:String) throws -> Data {
        guard !command.unicodeScalars.contains(where:{$0.value<32 || $0.value==127}) else {throw MailFailure(message:"Invalid mail command.")}
        sequence+=1;let tag="M\(sequence)"
        try channel.send(Data("\(tag) \(command)\r\n".utf8))
        var response=Data()
        while true {
            let line=try channel.line();let text=String(decoding:line,as:UTF8.self)
            response.append(line)
            guard response.count<16*1024*1024 else {throw MailFailure(message:"The inbox response is too large.")}
            if text.hasPrefix(tag+" ") {
                guard text.hasPrefix(tag+" OK") else {throw MailFailure(message:command.hasPrefix("LOGIN") ? "iCloud rejected the sign-in. Check your primary iCloud address and app-specific password." : "iCloud could not complete this request. Your mailbox was not changed.")}
                return response
            }
            if text.hasPrefix("* BYE") {throw MailFailure(message:"iCloud ended the session. Please refresh.")}
            if let length=IMAPParser.capture(#"\{([0-9]+)\}\r\n$"#,text).flatMap(Int.init) {response.append(try channel.bytes(length))}
        }
    }
    func authenticate(_ credentials:MailCredentials) throws {
        _=try command("LOGIN \(try MailValidation.quote(credentials.email)) \(try MailValidation.quote(credentials.password))")
    }
    func close(){channel.close()}
    deinit{close()}
}
