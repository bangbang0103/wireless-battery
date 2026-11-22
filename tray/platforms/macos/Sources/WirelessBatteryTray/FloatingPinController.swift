import AppKit
import SwiftUI

/// Manages a lightweight always-on-top draggable NSPanel when the user pins the popover.
final class FloatingPinController: ObservableObject {
    @Published var isPinned = false
    private var panel: NSPanel?

    func toggle(store: TrayStore) {
        if isPinned {
            unpin()
        } else {
            pin(store: store)
        }
    }

    func pin(store: TrayStore) {
        if let existing = panel {
            applyAppearance(themeMode: store.snapshot.config.themeMode, to: existing)
            existing.orderFrontRegardless()
            isPinned = true
            return
        }

        let content = PopoverRootView(store: store, pinController: self, inFloatingPanel: true)
            .frame(width: 344)
        let hosting = NSHostingController(rootView: content)

        let panel = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: 344, height: 300),
            styleMask: [.borderless, .nonactivatingPanel, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        panel.contentViewController = hosting
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.isMovableByWindowBackground = true
        panel.hidesOnDeactivate = false
        panel.contentView?.wantsLayer = true
        panel.contentView?.layer?.cornerRadius = 14
        panel.contentView?.layer?.masksToBounds = true
        applyAppearance(themeMode: store.snapshot.config.themeMode, to: panel)

        if let screen = NSScreen.main {
            let visible = screen.visibleFrame
            let origin = NSPoint(x: visible.maxX - 360, y: visible.maxY - 320)
            panel.setFrameOrigin(origin)
        }

        panel.orderFrontRegardless()
        self.panel = panel
        self.isPinned = true
    }

    func unpin() {
        panel?.close()
        panel = nil
        isPinned = false
    }

    func updateChromeVisibility(_ visible: Bool) {
        guard let panel else { return }
        panel.hasShadow = visible
        panel.contentView?.layer?.cornerRadius = visible ? 14 : 0
    }

    func updateAppearance(themeMode: String) {
        guard let panel else { return }
        applyAppearance(themeMode: themeMode, to: panel)
    }

    private func applyAppearance(themeMode: String, to panel: NSPanel) {
        switch themeMode {
        case "light":
            panel.appearance = NSAppearance(named: .aqua)
        case "dark":
            panel.appearance = NSAppearance(named: .darkAqua)
        default:
            panel.appearance = nil
        }
    }
}
