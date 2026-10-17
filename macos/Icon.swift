import AppKit
let size = 1024
let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
let cream = NSColor(srgbRed: 244/255, green: 241/255, blue: 232/255, alpha: 1)
let terracotta = NSColor(srgbRed: 217/255, green: 119/255, blue: 87/255, alpha: 1)
let edge = NSColor(srgbRed: 207/255, green: 203/255, blue: 189/255, alpha: 1)
let charcoal = NSColor(srgbRed: 61/255, green: 57/255, blue: 41/255, alpha: 1)
edge.setFill(); NSBezierPath(roundedRect: NSRect(x: 36, y: 36, width: 952, height: 952), xRadius: 218, yRadius: 218).fill()
cream.setFill(); NSBezierPath(roundedRect: NSRect(x: 47, y: 47, width: 930, height: 930), xRadius: 207, yRadius: 207).fill()
terracotta.setFill(); NSBezierPath(ovalIn: NSRect(x: 211, y: 211, width: 602, height: 602)).fill()
cream.setFill(); NSBezierPath(ovalIn: NSRect(x: 285, y: 285, width: 454, height: 454)).fill()
terracotta.setFill(); NSBezierPath(roundedRect: NSRect(x: 360, y: 399, width: 304, height: 226), xRadius: 42, yRadius: 42).fill()
cream.setStroke()
let flap = NSBezierPath(); flap.move(to: NSPoint(x: 377, y: 602)); flap.line(to: NSPoint(x: 512, y: 501)); flap.line(to: NSPoint(x: 647, y: 602)); flap.lineWidth = 22; flap.lineCapStyle = .round; flap.lineJoinStyle = .round; flap.stroke()
charcoal.setFill(); NSBezierPath(ovalIn: NSRect(x: 606, y: 579, width: 74, height: 74)).fill()
NSGraphicsContext.restoreGraphicsState()
try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
