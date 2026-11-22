import AppKit
import SwiftUI

@main
struct WirelessBatteryTrayApp: App {
    @StateObject private var store = TrayStore()
    @StateObject private var pinController = FloatingPinController()

    init() {
        // Keep the process out of the Dock even when launched directly without a `.app` bundle.
        NSApplication.shared.setActivationPolicy(.accessory)
    }

    var body: some Scene {
        MenuBarExtra {
            PopoverRootView(store: store, pinController: pinController, inFloatingPanel: false)
                .frame(width: 344)
        } label: {
            // Pure minimalist Logo in macOS menu bar (zero numbers; subtle low-battery dot).
            MenuBarLogoIcon(state: store.snapshot.iconState, tooltip: store.snapshot.tooltip)
        }
        .menuBarExtraStyle(.window)
    }
}

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

private struct MenuBarLogoIcon: View {
    let state: String
    let tooltip: String
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        Image(nsImage: Self.renderNSImage(state: state, isDark: colorScheme == .dark))
            .help(tooltip)
    }

    private static func renderNSImage(state: String, isDark: Bool) -> NSImage {
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

private struct PopoverRootView: View {
    @ObservedObject var store: TrayStore
    @ObservedObject var pinController: FloatingPinController
    let inFloatingPanel: Bool
    @State private var showSettings = false
    @State private var expandedId: String?
    @State private var copiedBanner = false
    @State private var isHovering = false

    private var lang: AppLanguage {
        store.language
    }

    private var showChrome: Bool {
        !inFloatingPanel || isHovering || showSettings || store.snapshot.devices.isEmpty
    }

    private var compactTransparent: Bool {
        inFloatingPanel && !showChrome
    }

    private var preferredScheme: ColorScheme? {
        switch store.snapshot.config.themeMode {
        case "light": return .light
        case "dark": return .dark
        default: return nil
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            if showSettings {
                settingsView
            } else {
                mainView
            }
        }
        .background(showChrome ? AnyShapeStyle(.ultraThinMaterial) : AnyShapeStyle(Color.clear))
        .preferredColorScheme(preferredScheme)
        .animation(.easeInOut(duration: 0.18), value: showChrome)
        .onHover { hovering in
            isHovering = hovering
        }
        .onChange(of: showChrome) { visible in
            if inFloatingPanel {
                pinController.updateChromeVisibility(visible)
            }
        }
        .onChange(of: store.snapshot.config.themeMode) { mode in
            pinController.updateAppearance(themeMode: mode)
        }
        .onAppear {
            if inFloatingPanel {
                pinController.updateChromeVisibility(showChrome)
                pinController.updateAppearance(themeMode: store.snapshot.config.themeMode)
            }
            store.refreshIfNeededOnOpen()
        }
    }

    // MARK: - Main View

    private var mainView: some View {
        VStack(spacing: 0) {
            if showChrome {
                // Native macOS Menu Bar Popover Header
                HStack(spacing: 8) {
                    Text("Wireless Battery")
                        .font(.system(size: 13, weight: .semibold))
                    if store.snapshot.refreshing {
                        ProgressView()
                            .controlSize(.mini)
                    }
                    Spacer()
                    HeaderIconButton(
                        systemName: pinController.isPinned ? "pin.fill" : "pin",
                        active: pinController.isPinned,
                        help: pinController.isPinned ? lang.pinHelpPinned : lang.pinHelpUnpinned
                    ) {
                        if inFloatingPanel {
                            pinController.unpin()
                        } else {
                            pinController.toggle(store: store)
                        }
                    }
                    HeaderIconButton(
                        systemName: "arrow.clockwise",
                        active: false,
                        help: lang.refreshAllHelp
                    ) {
                        store.refreshAll()
                    }
                    HeaderIconButton(
                        systemName: "gearshape",
                        active: false,
                        help: lang.settingsHelp
                    ) {
                        withAnimation(.easeInOut(duration: 0.16)) {
                            showSettings = true
                        }
                    }
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 10)

                Divider()
            }

            // Divider-separated device list (native macOS Control Center / Bluetooth list style)
            if store.snapshot.devices.isEmpty {
                emptyState
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(Array(store.snapshot.devices.enumerated()), id: \.element.id) { index, device in
                            if index > 0 {
                                Divider()
                                    .padding(.horizontal, 14)
                            }
                            DeviceRowView(
                                device: device,
                                lang: lang,
                                expanded: !compactTransparent && expandedId == device.id,
                                compact: compactTransparent,
                                onToggle: {
                                    guard !compactTransparent else { return }
                                    withAnimation(.easeInOut(duration: 0.16)) {
                                        expandedId = expandedId == device.id ? nil : device.id
                                    }
                                },
                                onRefresh: { store.refreshDevice(device.id) },
                                onCopyError: {
                                    if let json = store.errorReportJSON(for: device.id) {
                                        NSPasteboard.general.clearContents()
                                        NSPasteboard.general.setString(json, forType: .string)
                                        withAnimation(.easeInOut(duration: 0.15)) {
                                            copiedBanner = true
                                        }
                                        DispatchQueue.main.asyncAfter(deadline: .now() + 1.8) {
                                            withAnimation(.easeInOut(duration: 0.15)) {
                                                copiedBanner = false
                                            }
                                        }
                                    }
                                }
                            )
                        }
                    }
                    .padding(.vertical, 6)
                }
                .frame(maxHeight: 360)
            }

            if copiedBanner {
                HStack(spacing: 5) {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 11))
                        .foregroundStyle(.green)
                    Text(lang.toastErrorCopied)
                        .font(.system(size: 11, weight: .medium))
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 5)
                .background(.regularMaterial, in: Capsule())
                .overlay(Capsule().strokeBorder(Color.primary.opacity(0.1), lineWidth: 0.5))
                .padding(.bottom, 8)
                .transition(.opacity.combined(with: .scale(scale: 0.96)))
            }

            if showChrome {
                Divider()

                // Minimalist macOS Footer
                HStack(spacing: 6) {
                    Circle()
                        .fill(statusColor)
                        .frame(width: 6, height: 6)
                    Text(
                        store.snapshot.refreshing
                            ? lang.readingPeripherals
                            : lang.footerSummary(
                                lastSyncAt: store.snapshot.lastSyncAt,
                                pollSecs: store.snapshot.config.pollIntervalSecs
                            )
                    )
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    Spacer()
                    Button(lang.quitLabel) { NSApp.terminate(nil) }
                        .buttonStyle(.borderless)
                        .font(.system(size: 11.5))
                        .foregroundStyle(.secondary)
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 9)
            }
        }
    }

    // MARK: - Empty State

    private var emptyState: some View {
        let hasHidden = store.snapshot.hiddenExperimentalCount > 0
        return VStack(spacing: 8) {
            Image(systemName: "battery.75")
                .font(.system(size: 24))
                .foregroundStyle(.secondary)
            Text(lang.emptyTitle)
                .font(.system(size: 12.5, weight: .medium))
            Text(lang.emptyHint(hiddenExperimentalCount: store.snapshot.hiddenExperimentalCount))
                .font(.system(size: 11))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            Button(lang.emptyAction(hasHiddenExperimental: hasHidden)) {
                if hasHidden {
                    showSettings = true
                } else {
                    store.refreshAll()
                }
            }
            .buttonStyle(.bordered)
            .controlSize(.small)
            .padding(.top, 2)
        }
        .padding(.vertical, 28)
        .padding(.horizontal, 20)
    }

    // MARK: - Native macOS Settings View

    private var settingsView: some View {
        let cfg = store.snapshot.config
        return VStack(spacing: 0) {
            // Native macOS navigation header
            HStack {
                Button {
                    withAnimation(.easeInOut(duration: 0.16)) {
                        showSettings = false
                    }
                } label: {
                    HStack(spacing: 3) {
                        Image(systemName: "chevron.left")
                            .font(.system(size: 11, weight: .semibold))
                        Text(lang.backLabel)
                            .font(.system(size: 12, weight: .medium))
                    }
                }
                .buttonStyle(.borderless)
                Spacer()
                Text(lang.settingsTitle)
                    .font(.system(size: 13, weight: .semibold))
                Spacer()
                Color.clear.frame(width: 44, height: 1)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)

            Divider()

            // Native macOS Ventura/Sonoma/Sequoia grouped inset Settings containers
            VStack(spacing: 10) {
                // Group 1: Appearance, Language, Poll Interval (native macOS pop-up menu Pickers)
                MacSettingsGroup {
                    MacPickerRow(
                        icon: "circle.lefthalf.filled",
                        title: lang.settingThemeTitle
                    ) {
                        Picker("", selection: Binding(
                            get: { cfg.themeMode },
                            set: { next in
                                var u = cfg
                                u.themeMode = next
                                store.updateConfig(u)
                            }
                        )) {
                            Text(lang.themeLabel("system")).tag("system")
                            Text(lang.themeLabel("light")).tag("light")
                            Text(lang.themeLabel("dark")).tag("dark")
                        }
                        .pickerStyle(.menu)
                        .labelsHidden()
                        .controlSize(.small)
                        .fixedSize()
                    }

                    Divider().padding(.leading, 38)

                    MacPickerRow(
                        icon: "globe",
                        title: lang.settingLanguageTitle
                    ) {
                        Picker("", selection: Binding(
                            get: { cfg.language },
                            set: { next in
                                var u = cfg
                                u.language = next
                                store.updateConfig(u)
                            }
                        )) {
                            Text(lang.languageLabel("system")).tag("system")
                            Text(lang.languageLabel("zh_cn")).tag("zh_cn")
                            Text(lang.languageLabel("en")).tag("en")
                        }
                        .pickerStyle(.menu)
                        .labelsHidden()
                        .controlSize(.small)
                        .fixedSize()
                    }

                    Divider().padding(.leading, 38)

                    MacPickerRow(
                        icon: "clock",
                        title: lang.settingPollTitle,
                        subtitle: lang.settingPollDesc
                    ) {
                        Picker("", selection: Binding(
                            get: { cfg.pollIntervalSecs },
                            set: { next in
                                var u = cfg
                                u.pollIntervalSecs = next
                                store.updateConfig(u)
                            }
                        )) {
                            Text(lang.pollLabel(300)).tag(UInt64(300))
                            Text(lang.pollLabel(900)).tag(UInt64(900))
                            Text(lang.pollLabel(1800)).tag(UInt64(1800))
                            Text(lang.pollLabel(3600)).tag(UInt64(3600))
                            Text(lang.pollLabel(0)).tag(UInt64(0))
                        }
                        .pickerStyle(.menu)
                        .labelsHidden()
                        .controlSize(.small)
                        .fixedSize()
                    }
                }

                // Group 2: Low Battery Alert + nested Silent Notifications sub-option
                MacSettingsGroup {
                    MacToggleRow(
                        icon: "bell.badge",
                        title: lang.settingLowBatteryTitle,
                        subtitle: lang.lowBatteryDesc(enabled: cfg.lowBatteryNotify, silent: cfg.lowBatterySilent),
                        isOn: Binding(
                            get: { cfg.lowBatteryNotify },
                            set: { v in
                                var u = cfg
                                u.lowBatteryNotify = v
                                withAnimation(.easeInOut(duration: 0.16)) {
                                    store.updateConfig(u)
                                }
                            }
                        )
                    )

                    if cfg.lowBatteryNotify {
                        Divider().padding(.leading, 38)

                        MacToggleRow(
                            icon: "bell.slash",
                            title: lang.settingSilentTitle,
                            subtitle: lang.settingSilentDesc,
                            indented: true,
                            isOn: Binding(
                                get: { cfg.lowBatterySilent },
                                set: { v in
                                    var u = cfg
                                    u.lowBatterySilent = v
                                    store.updateConfig(u)
                                }
                            )
                        )
                    }
                }

                // Group 3: Launch at Login
                MacSettingsGroup {
                    MacToggleRow(
                        icon: "power",
                        title: lang.settingAutostartTitle,
                        subtitle: lang.settingAutostartDesc,
                        isOn: Binding(
                            get: { cfg.autostart },
                            set: { v in
                                var u = cfg
                                u.autostart = v
                                store.updateConfig(u)
                            }
                        )
                    )
                    .disabled(!store.isBundledApp)
                    .help(store.isBundledApp ? lang.settingAutostartHelpBundled : lang.settingAutostartHelpUnbundled)
                }
            }
            .padding(14)
        }
    }

    private var statusColor: Color {
        switch store.snapshot.iconState {
        case "low_battery": return .red
        case "normal": return .green
        default: return .secondary
        }
    }
}

// MARK: - Native macOS Helper Components

private struct HeaderIconButton: View {
    let systemName: String
    let active: Bool
    let help: String
    let action: () -> Void
    @State private var hovered = false

    var body: some View {
        Button(action: action) {
            Image(systemName: systemName)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(active ? Color.accentColor : (hovered ? Color.primary : Color.secondary))
                .frame(width: 24, height: 24)
                .background(
                    RoundedRectangle(cornerRadius: 6)
                        .fill(
                            active
                                ? Color.accentColor.opacity(0.16)
                                : (hovered ? Color.primary.opacity(0.08) : Color.clear)
                        )
                )
        }
        .buttonStyle(.plain)
        .onHover { hovered = $0 }
        .help(help)
    }
}

private struct MacSettingsGroup<Content: View>: View {
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(spacing: 0) {
            content()
        }
        .background(Color.primary.opacity(0.045), in: RoundedRectangle(cornerRadius: 10))
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .strokeBorder(Color.primary.opacity(0.08), lineWidth: 0.5)
        )
    }
}

private struct MacPickerRow<Control: View>: View {
    let icon: String
    let title: String
    var subtitle: String? = nil
    @ViewBuilder let control: () -> Control

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: icon)
                .font(.system(size: 13))
                .foregroundStyle(.secondary)
                .frame(width: 18, alignment: .center)

            VStack(alignment: .leading, spacing: 1) {
                Text(title)
                    .font(.system(size: 12, weight: .medium))
                if let subtitle {
                    Text(subtitle)
                        .font(.system(size: 10.5))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }

            Spacer(minLength: 8)
            control()
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 9)
    }
}

private struct MacToggleRow: View {
    let icon: String
    let title: String
    let subtitle: String
    var indented: Bool = false
    @Binding var isOn: Bool

    var body: some View {
        HStack(spacing: 10) {
            if indented {
                Spacer().frame(width: 12)
            }
            Image(systemName: icon)
                .font(.system(size: indented ? 12 : 13))
                .foregroundStyle(.secondary)
                .frame(width: 18, alignment: .center)

            VStack(alignment: .leading, spacing: 1) {
                Text(title)
                    .font(.system(size: 12, weight: indented ? .regular : .medium))
                Text(subtitle)
                    .font(.system(size: 10.5))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer(minLength: 8)

            Toggle("", isOn: $isOn)
                .toggleStyle(.switch)
                .controlSize(.mini)
                .labelsHidden()
        }
        .padding(.horizontal, 12)
        .padding(.vertical, indented ? 8 : 9)
    }
}

// MARK: - Divider-Separated Device Row

private struct DeviceRowView: View {
    let device: DeviceViewModel
    let lang: AppLanguage
    let expanded: Bool
    let compact: Bool
    let onToggle: () -> Void
    let onRefresh: () -> Void
    let onCopyError: () -> Void
    @State private var hovered = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                if compact {
                    Image(systemName: symbolName)
                        .font(.system(size: 15))
                        .frame(width: 30, height: 30)
                        .shadow(color: Color.black.opacity(0.35), radius: 1.5, x: 0, y: 1)
                } else {
                    ZStack {
                        RoundedRectangle(cornerRadius: 8)
                            .fill(Color.primary.opacity(0.06))
                            .frame(width: 34, height: 34)
                        Image(systemName: symbolName)
                            .font(.system(size: 14))
                    }
                }

                VStack(alignment: .leading, spacing: 2) {
                    Text(device.name)
                        .font(.system(size: 13, weight: .medium))
                        .lineLimit(1)
                    Text(lang.deviceSubtitle(for: device))
                        .font(.system(size: 11))
                        .foregroundStyle(subtitleColor)
                        .lineLimit(1)
                }
                .shadow(color: compact ? Color.black.opacity(0.35) : .clear, radius: 1.5, x: 0, y: 1)

                Spacer()

                readoutView
                    .frame(width: 68, alignment: .trailing)
                    .shadow(color: compact ? Color.black.opacity(0.35) : .clear, radius: 1.5, x: 0, y: 1)

                if !compact {
                    Image(systemName: expanded ? "chevron.up" : "chevron.down")
                        .font(.system(size: 9.5, weight: .semibold))
                        .foregroundStyle(
                            hovered || expanded
                                ? HierarchicalShapeStyle.secondary
                                : HierarchicalShapeStyle.tertiary
                        )
                        .frame(width: 14)
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 9)
            .background(
                RoundedRectangle(cornerRadius: 8)
                    .fill(!compact && hovered ? Color.primary.opacity(0.055) : Color.clear)
            )
            .padding(.horizontal, 6)
            .contentShape(Rectangle())
            .onHover { hovered = $0 }
            .onTapGesture(perform: onToggle)

            if expanded {
                VStack(spacing: 7) {
                    HStack {
                        Text(lang.labelProtocol).foregroundStyle(.secondary)
                        Spacer()
                        Text(device.providerId ?? "--")
                            .font(.system(size: 10.5, design: .monospaced))
                    }
                    HStack {
                        Text(lang.labelBus).foregroundStyle(.secondary)
                        Spacer()
                        Text(device.transportLabel)
                    }
                    HStack {
                        Text(lang.labelStatus).foregroundStyle(.secondary)
                        Spacer()
                        Text(expandedStatusLabel)
                            .foregroundStyle(expandedStatusColor)
                    }
                    HStack(spacing: 8) {
                        Spacer()
                        Button(lang.btnRefreshDevice, action: onRefresh)
                            .buttonStyle(.bordered)
                            .controlSize(.small)
                        if device.lastError != nil {
                            Button(lang.btnCopyError, action: onCopyError)
                                .buttonStyle(.bordered)
                                .controlSize(.small)
                        }
                    }
                    .padding(.top, 2)
                }
                .font(.system(size: 11))
                .padding(.horizontal, 12)
                .padding(.vertical, 10)
                .background(Color.primary.opacity(0.045), in: RoundedRectangle(cornerRadius: 8))
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .strokeBorder(Color.primary.opacity(0.07), lineWidth: 0.5)
                )
                .padding(.horizontal, 14)
                .padding(.bottom, 8)
                .transition(.opacity)
            }
        }
    }

    @ViewBuilder
    private var readoutView: some View {
        if device.precision == "percentage", let pct = device.percentage {
            VStack(alignment: .trailing, spacing: 5) {
                HStack(spacing: 2) {
                    if device.chargeState == "charging" {
                        Image(systemName: "bolt.fill")
                            .font(.system(size: 10))
                            .foregroundStyle(.green)
                    }
                    Text("\(pct)%")
                        .font(.system(size: 14, weight: .semibold).monospacedDigit())
                        .foregroundStyle(readoutTextColor)
                }
                GeometryReader { geo in
                    ZStack(alignment: .leading) {
                        Capsule().fill(Color.primary.opacity(0.12))
                        Capsule()
                            .fill(barColor)
                            .frame(width: max(3, geo.size.width * CGFloat(pct) / 100.0))
                    }
                }
                .frame(height: 3.5)
            }
        } else if device.precision == "discrete_level" {
            let segs = discreteSegments
            VStack(alignment: .trailing, spacing: 5) {
                Text(lang.discreteLabel(device.level))
                    .font(.system(size: 12, weight: .semibold))
                HStack(spacing: 3) {
                    ForEach(0..<4, id: \.self) { i in
                        Capsule()
                            .fill(i < segs ? barColor : Color.primary.opacity(0.12))
                            .frame(height: 3.5)
                    }
                }
            }
        } else if device.precision == "voltage_only", let mv = device.voltageMv {
            Text(String(format: "%.2f V", Double(mv) / 1000.0))
                .font(.system(size: 12, weight: .semibold).monospacedDigit())
        } else {
            Text("--").font(.system(size: 14, weight: .semibold)).foregroundStyle(.secondary)
        }
    }

    private var symbolName: String {
        switch device.kind {
        case "mouse": return "computermouse"
        case "keyboard": return "keyboard"
        case "gamepad": return "gamecontroller"
        case "headset": return "headphones"
        default: return "battery.75"
        }
    }

    private var subtitleColor: Color {
        if device.chargeState == "charging" { return .green }
        if device.precision == "none" && device.lastError != nil { return .orange }
        return .secondary
    }

    private var expandedStatusLabel: String {
        if let err = device.lastError {
            return lang.errorShortLabel(err)
        }
        if device.freshness == "stale" { return lang.statusStale }
        if device.freshness == "live" { return lang.statusLive }
        return lang.statusUnread
    }

    private var expandedStatusColor: Color {
        if device.lastError != nil { return .orange }
        if device.freshness == "live" { return .green }
        return .secondary
    }

    private var readoutTextColor: Color {
        if device.chargeState == "charging" { return .green }
        if let pct = device.percentage {
            if pct <= 15 { return .red }
            if pct <= 30 { return .orange }
        }
        return device.freshness == "stale" ? .secondary : .primary
    }

    private var barColor: Color {
        if device.chargeState == "charging" { return .green }
        if let pct = device.percentage {
            if pct <= 15 { return .red }
            if pct <= 30 { return .orange }
        }
        return device.freshness == "stale" ? .green.opacity(0.5) : .green
    }

    private var discreteSegments: Int {
        switch device.level {
        case "full": return 4
        case "high": return 3
        case "medium": return 2
        case "low", "critical": return 1
        default: return 0
        }
    }
}
