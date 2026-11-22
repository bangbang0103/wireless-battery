import AppKit

enum MenuBarIconRenderer {
    static func renderNSImage(state: String, isDark: Bool) -> NSImage {
        let size = NSSize(width: 20, height: 20)
        let isIdle = state == "idle"
        let image = NSImage(size: size, flipped: true) { rect in
            let s = min(rect.width, rect.height) / 24.0
            let ox = (rect.width - 24.0 * s) * 0.5
            let oy = (rect.height - 24.0 * s) * 0.5
            let boltOpacity: CGFloat = isIdle ? 0.48 : 1.0
            let baseColor: NSColor = isDark ? .white : .black
            let boltColor = baseColor.withAlphaComponent(boltOpacity)
            let waveColor: NSColor = state == "low_battery"
                ? NSColor(srgbRed: 244.0 / 255.0, green: 63.0 / 255.0, blue: 94.0 / 255.0, alpha: 1.0)
                : boltColor

            func pt(_ x: CGFloat, _ y: CGFloat) -> NSPoint {
                NSPoint(x: ox + x * s, y: oy + y * s)
            }

            func addQuad(_ path: NSBezierPath, from p0: NSPoint, control c: NSPoint, to p2: NSPoint) {
                let cp1 = NSPoint(x: p0.x + (2.0 / 3.0) * (c.x - p0.x), y: p0.y + (2.0 / 3.0) * (c.y - p0.y))
                let cp2 = NSPoint(x: p2.x + (2.0 / 3.0) * (c.x - p2.x), y: p2.y + (2.0 / 3.0) * (c.y - p2.y))
                path.curve(to: p2, controlPoint1: cp1, controlPoint2: cp2)
            }

            // Centered 70%-rounded diagonal lightning bolt (24x24 viewBox)
            let corners: [(NSPoint, NSPoint, NSPoint)] = [
                (pt(11.27, 3.89), pt(11.60, 2.40), pt(10.71, 3.64)),
                (pt(3.37, 13.92), pt(2.60, 15.00), pt(3.93, 15.00)),
                (pt(6.77, 15.00), pt(8.10, 15.00), pt(7.73, 16.28)),
                (pt(6.52, 20.53), pt(6.10, 22.00), pt(7.01, 20.77)),
                (pt(14.81, 10.17), pt(15.60, 9.10), pt(14.27, 9.10)),
                (pt(11.43, 9.10), pt(10.10, 9.10), pt(10.39, 7.80)),
            ]
            let boltPath = NSBezierPath()
            boltPath.move(to: corners[0].0)
            for (idx, corner) in corners.enumerated() {
                if idx > 0 {
                    boltPath.line(to: corner.0)
                }
                addQuad(boltPath, from: corner.0, control: corner.1, to: corner.2)
            }
            boltPath.close()
            boltColor.setFill()
            boltPath.fill()

            // Wireless broadcast waves (red on low_battery, monochrome otherwise)
            waveColor.setStroke()
            let innerWave = NSBezierPath()
            innerWave.move(to: pt(15.9, 5.2))
            addQuad(innerWave, from: pt(15.9, 5.2), control: pt(19.6, 8.0), to: pt(19.1, 12.0))
            innerWave.lineWidth = max(1.25, 2.1 * s)
            innerWave.lineCapStyle = .round
            innerWave.lineJoinStyle = .round
            innerWave.stroke()

            let outerWave = NSBezierPath()
            outerWave.move(to: pt(16.8, 2.0))
            addQuad(outerWave, from: pt(16.8, 2.0), control: pt(22.6, 6.0), to: pt(21.6, 13.0))
            outerWave.lineWidth = max(1.1, 1.8 * s)
            outerWave.lineCapStyle = .round
            outerWave.lineJoinStyle = .round
            outerWave.stroke()

            return true
        }
        image.isTemplate = false
        return image
    }
}
