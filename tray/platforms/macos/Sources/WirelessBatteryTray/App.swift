import AppKit
import SwiftUI

@main
struct WirelessBatteryTrayApp: App {
    @NSApplicationDelegateAdaptor(TrayApplicationDelegate.self) private var delegate

    var body: some Scene {
        Settings { EmptyView() }
    }
}

struct PopoverRootView: View {
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
                SettingsView(store: store) {
                    withAnimation(.easeInOut(duration: 0.16)) {
                        showSettings = false
                    }
                }
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
                .frame(maxHeight: 540)
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
                            ? "\(store.snapshot.appVersion) · \(lang.readingPeripherals)"
                            : lang.footerSummary(
                                appVersion: store.snapshot.appVersion,
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
        VStack(spacing: 8) {
            Image(systemName: "battery.75")
                .font(.system(size: 24))
                .foregroundStyle(.secondary)
            Text(lang.emptyTitle)
                .font(.system(size: 12.5, weight: .medium))
            Text(lang.emptyHint)
                .font(.system(size: 11))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            Button(lang.emptyAction) {
                store.refreshAll()
            }
            .buttonStyle(.bordered)
            .controlSize(.small)
            .padding(.top, 2)
        }
        .padding(.vertical, 28)
        .padding(.horizontal, 20)
    }

    private var statusColor: Color {
        switch store.snapshot.iconState {
        case "low_battery": return .red
        case "normal": return .green
        default: return .secondary
        }
    }
}

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
