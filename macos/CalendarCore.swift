import Foundation

enum MeetingDetector {
    static func suggestions(text:String,title:String,now:Date = Date())throws->[[String:String]] {
        let detector=try NSDataDetector(types:NSTextCheckingResult.CheckingType.date.rawValue)
        let input=String(text.prefix(6000));let range=NSRange(input.startIndex...,in:input)
        var results:[[String:String]]=[];var seen=Set<String>()
        for match in detector.matches(in:input,range:range) {
            guard let date=match.date,date>=now.addingTimeInterval(-3600),date<now.addingTimeInterval(366*86400),let swiftRange=Range(match.range,in:input) else {continue}
            let start=ISO8601DateFormatter().string(from:date)
            guard seen.insert(start).inserted else {continue}
            results.append(["title":String(title.prefix(200)),"start":start,"end":ISO8601DateFormatter().string(from:date.addingTimeInterval(match.duration>0 ? min(match.duration,86400) : 3600)),"phrase":String(input[swiftRange])])
            if results.count==8{break}
        }
        return results
    }
}
