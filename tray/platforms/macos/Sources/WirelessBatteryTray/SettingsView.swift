import AppKit
import SwiftUI

struct SettingsView: View {
    @ObservedObject var store: TrayStore
    let onBack: () -> Void

    private var lang: AppLanguage {
        store.language
    }

    var body: some View {
        let cfg = store.snapshot.config
        VStack(spacing: 0) {
            // Native macOS navigation header
            HStack {
                Button(action: onBack) {
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
                            if ![300, 900, 1800, 3600, 0].contains(cfg.pollIntervalSecs) {
                                Text(lang.pollLabel(cfg.pollIntervalSecs)).tag(cfg.pollIntervalSecs)
                            }
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

                // Group 3: Experimental Devices & Launch at Login
                MacSettingsGroup {
                    MacToggleRow(
                        icon: "flask",
                        title: lang.settingExperimentalTitle,
                        subtitle: lang.settingExperimentalDesc,
                        isOn: Binding(
                            get: { cfg.allowExperimental },
                            set: { v in
                                var u = cfg
                                u.allowExperimental = v
                                store.updateConfig(u)
                            }
                        )
                    )

                    Divider().padding(.leading, 38)

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

                // Group 4: Version & Check for Updates
                MacSettingsGroup {
                    MacPickerRow(
                        icon: "info.circle",
                        title: lang.settingVersionTitle,
                        subtitle: versionSubtitle
                    ) {
                        Button(versionActionLabel) {
                            if case let .available(_, url) = store.updateStatus,
                               TrayStore.isValidUpdateURL(url)
                            {
                                NSWorkspace.shared.open(url)
                            } else {
                                store.checkForUpdates()
                            }
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                        .disabled(store.updateStatus == .checking)
                    }
                }
            }
            .padding(14)
        }
    }

    private var versionSubtitle: String {
        switch store.updateStatus {
        case .idle:
            return lang.settingVersionDesc(store.snapshot.appVersion)
        case .checking:
            return lang.settingVersionChecking
        case .upToDate:
            return lang.settingVersionUpToDate(store.snapshot.appVersion)
        case let .available(latestVersion, _):
            return lang.settingVersionAvailable(latestVersion)
        case .failed:
            return lang.settingVersionFailed
        }
    }

    private var versionActionLabel: String {
        switch store.updateStatus {
        case .checking:
            return lang.btnCheckingUpdates
        case .available:
            return lang.btnDownloadUpdate
        case .idle, .upToDate, .failed:
            return lang.btnCheckUpdates
        }
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
