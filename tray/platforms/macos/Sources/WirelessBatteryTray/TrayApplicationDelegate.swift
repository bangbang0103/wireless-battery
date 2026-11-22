import AppKit
import Combine
import SwiftUI

final class TrayApplicationDelegate: NSObject, NSApplicationDelegate {
    private let store = TrayStore()
    private let pinController = FloatingPinController()
    private let popover = NSPopover()
    private var statusItem: NSStatusItem?
    private var contextMenuController: TrayContextMenuController?
    private var snapshotObserver: AnyCancellable?
    private var appearanceObserver: NSKeyValueObservation?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApplication.shared.setActivationPolicy(.accessory)

        let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        self.statusItem = statusItem

        let content = PopoverRootView(store: store, pinController: pinController, inFloatingPanel: false)
            .frame(width: 344)
        popover.contentViewController = NSHostingController(rootView: content)
        popover.behavior = .transient
        contextMenuController = TrayContextMenuController(store: store) { [weak self] in
            self?.openPanel()
        }

        guard let button = statusItem.button else { return }
        button.target = self
        button.action = #selector(handleStatusItemClick(_:))
        button.sendAction(on: [.leftMouseUp, .rightMouseUp])

        snapshotObserver = store.$snapshot
            .receive(on: RunLoop.main)
            .sink { [weak self] snapshot in
                self?.updateStatusItem(snapshot)
            }
        appearanceObserver = button.observe(\.effectiveAppearance, options: [.new]) { [weak self] _, _ in
            guard let self else { return }
            self.updateStatusItem(self.store.snapshot)
        }
        updateStatusItem(store.snapshot)

        #if DEV_TOOLS
            DevPanelController.shared.showIfNeeded(store: store)
        #endif
    }

    @objc private func handleStatusItemClick(_ sender: NSStatusBarButton) {
        switch NSApplication.shared.currentEvent?.type {
        case .rightMouseUp, .rightMouseDown:
            showContextMenu(for: sender)
        default:
            togglePopover(for: sender)
        }
    }

    private func togglePopover(for button: NSStatusBarButton) {
        if popover.isShown {
            popover.performClose(nil)
        } else {
            showPopover(for: button)
        }
    }

    private func showPopover(for button: NSStatusBarButton) {
        store.refreshIfNeededOnOpen()
        popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
    }

    private func showContextMenu(for button: NSStatusBarButton) {
        popover.performClose(nil)
        guard let event = NSApplication.shared.currentEvent else { return }
        contextMenuController?.show(for: button, event: event)
    }

    @objc private func openPanel() {
        DispatchQueue.main.async { [weak self] in
            guard let self, let button = self.statusItem?.button else { return }
            self.showPopover(for: button)
        }
    }

    private func updateStatusItem(_ snapshot: TraySnapshotModel) {
        guard let button = statusItem?.button else { return }
        let isDark = button.effectiveAppearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
        button.image = MenuBarIconRenderer.renderNSImage(state: snapshot.iconState, isDark: isDark)
        button.toolTip = snapshot.tooltip
    }
}
