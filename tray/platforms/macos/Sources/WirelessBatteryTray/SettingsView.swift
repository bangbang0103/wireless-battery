import AppKit
import SwiftUI

enum CommunityLinkKind: Equatable {
    case bugReport
    case sourceRepo
    case qq
    case discord
}

struct SettingsView: View {
    @ObservedObject var store: TrayStore
    let onBack: () -> Void
    @State private var toastMessage: String?
    @State private var toastIsSuccess = true

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
            ScrollView {
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

                // Group 5: Source Repository
                MacSettingsGroup {
                    MacCommunityRow(
                        kind: .sourceRepo,
                        title: lang.settingSourceTitle,
                        subtitle: lang.settingSourceDesc
                    ) {
                        Button(lang.openRepositoryLabel) {
                            handleCommunityLinkTap(.sourceRepo)
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                    }
                }

                // Group 6: Feedback
                MacSettingsGroup {
                    MacCommunityRow(
                        kind: .bugReport,
                        title: lang.settingFeedbackTitle,
                        subtitle: lang.settingFeedbackDesc
                    ) {
                        Button(lang.submitFeedbackLabel) {
                            handleCommunityLinkTap(.bugReport)
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                    }
                }

                // Group 7: Community (QQ & Discord)
                MacSettingsGroup {
                    MacPickerRow(
                        icon: "person.2",
                        title: lang.settingCommunityTitle,
                        subtitle: lang.settingCommunityDesc
                    ) {
                        HStack(spacing: 3) {
                            CommunityIconButton(kind: .qq, accessibilityLabel: "QQ") {
                                handleCommunityLinkTap(.qq)
                            }
                            CommunityIconButton(kind: .discord, accessibilityLabel: "Discord") {
                                handleCommunityLinkTap(.discord)
                            }
                        }
                    }
                }
            }
            .padding(14)
            }
            .frame(maxHeight: 620)
        }
        .overlay(alignment: .bottom) {
            if let message = toastMessage {
                HStack(spacing: 5) {
                    Image(systemName: toastIsSuccess ? "checkmark.circle.fill" : "info.circle.fill")
                        .font(.system(size: 11))
                        .foregroundStyle(toastIsSuccess ? Color.green : Color.accentColor)
                    Text(message)
                        .font(.system(size: 11, weight: .medium))
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 5)
                .background(.regularMaterial, in: Capsule())
                .overlay(Capsule().strokeBorder(Color.primary.opacity(0.1), lineWidth: 0.5))
                .shadow(color: .black.opacity(0.12), radius: 6, y: 2)
                .padding(.bottom, 64)
                .allowsHitTesting(false)
                .transition(.opacity.combined(with: .scale(scale: 0.96)))
            }
        }
    }

    private func showToast(_ message: String, success: Bool) {
        toastIsSuccess = success
        withAnimation(.easeInOut(duration: 0.15)) {
            toastMessage = message
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.8) {
            if toastMessage == message {
                withAnimation(.easeInOut(duration: 0.15)) {
                    toastMessage = nil
                }
            }
        }
    }

    private func handleCommunityLinkTap(_ kind: CommunityLinkKind) {
        let links = store.communityLinks
        switch kind {
        case .bugReport:
            if let url = URL(string: links.issuesUrl), url.scheme == "https" || url.scheme == "http" {
                NSWorkspace.shared.open(url)
            }
        case .sourceRepo:
            if let url = URL(string: links.repoUrl), url.scheme == "https" || url.scheme == "http" {
                NSWorkspace.shared.open(url)
            }
        case .qq:
            if !links.qqGroup.isEmpty {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(links.qqGroup, forType: .string)
                showToast(lang.toastQqCopied(links.qqGroup), success: true)
            } else {
                showToast(lang.toastQqComingSoon, success: false)
            }
        case .discord:
            if !links.discordUrl.isEmpty {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(links.discordUrl, forType: .string)
                showToast(lang.toastDiscordCopied, success: true)
            } else {
                showToast(lang.toastDiscordComingSoon, success: false)
            }
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

private struct MacCommunityRow<Control: View>: View {
    let kind: CommunityLinkKind
    let title: String
    var subtitle: String? = nil
    @ViewBuilder let control: () -> Control

    var body: some View {
        HStack(spacing: 10) {
            CommunityLinkIcon(kind: kind, size: 13)
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

private struct CommunityIconButton: View {
    let kind: CommunityLinkKind
    let accessibilityLabel: String
    let action: () -> Void
    @State private var isHovered = false

    var body: some View {
        Button(action: action) {
            CommunityLinkIcon(kind: kind, size: 13)
                .foregroundStyle(isHovered ? Color.primary : Color.secondary)
                .frame(width: 26, height: 26)
                .background(
                    RoundedRectangle(cornerRadius: 6)
                        .fill(isHovered ? Color.primary.opacity(0.08) : Color.clear)
                )
        }
        .buttonStyle(.plain)
        .onHover { hovering in
            withAnimation(.easeInOut(duration: 0.12)) {
                isHovered = hovering
            }
        }
        .accessibilityLabel(Text(accessibilityLabel))
    }
}

private struct CommunityLinkIcon: View {
    let kind: CommunityLinkKind
    let size: CGFloat

    var body: some View {
        switch kind {
        case .bugReport:
            ZStack {
                Circle()
                    .stroke(lineWidth: 1.1)
                    .padding(0.5)
                Circle()
                    .fill()
                    .frame(width: size * 0.22, height: size * 0.22)
            }
            .frame(width: size, height: size)
        case .sourceRepo:
            GitHubIconShape()
                .fill(style: FillStyle(eoFill: true))
                .frame(width: size, height: size)
        case .qq:
            QQIconShape()
                .fill(style: FillStyle(eoFill: true))
                .frame(width: size, height: size)
        case .discord:
            DiscordIconShape()
                .fill(style: FillStyle(eoFill: true))
                .frame(width: size + 1, height: size * 0.8)
        }
    }
}

private struct GitHubIconShape: Shape {
    func path(in rect: CGRect) -> Path {
        let sx = rect.width / 24.0
        let sy = rect.height / 24.0
        var p = Path()
        p.move(to: CGPoint(x: 12 * sx, y: 1.5 * sy))
        p.addCurve(
            to: CGPoint(x: 1.5 * sx, y: 12 * sy),
            control1: CGPoint(x: 6.2 * sx, y: 1.5 * sy),
            control2: CGPoint(x: 1.5 * sx, y: 6.2 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 8.68 * sx, y: 21.96 * sy),
            control1: CGPoint(x: 1.5 * sx, y: 16.64 * sy),
            control2: CGPoint(x: 4.51 * sx, y: 20.57 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 9.38 * sx, y: 21.31 * sy),
            control1: CGPoint(x: 9.21 * sx, y: 22.06 * sy),
            control2: CGPoint(x: 9.38 * sx, y: 21.74 * sy)
        )
        p.addLine(to: CGPoint(x: 9.38 * sx, y: 19.01 * sy))
        p.addCurve(
            to: CGPoint(x: 6.13 * sx, y: 17.79 * sy),
            control1: CGPoint(x: 6.46 * sx, y: 19.64 * sy),
            control2: CGPoint(x: 5.84 * sx, y: 17.6 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 4.97 * sx, y: 16.32 * sy),
            control1: CGPoint(x: 5.65 * sx, y: 16.58 * sy),
            control2: CGPoint(x: 4.96 * sx, y: 16.26 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 5.05 * sx, y: 16.1 * sy),
            control1: CGPoint(x: 4.02 * sx, y: 15.67 * sy),
            control2: CGPoint(x: 5.04 * sx, y: 15.68 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 6.66 * sx, y: 17.18 * sy),
            control1: CGPoint(x: 6.1 * sx, y: 16.17 * sy),
            control2: CGPoint(x: 6.66 * sx, y: 17.18 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 9.43 * sx, y: 18.08 * sy),
            control1: CGPoint(x: 7.6 * sx, y: 18.78 * sy),
            control2: CGPoint(x: 9.12 * sx, y: 18.32 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 10.17 * sx, y: 16.65 * sy),
            control1: CGPoint(x: 9.52 * sx, y: 17.4 * sy),
            control2: CGPoint(x: 9.8 * sx, y: 16.94 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 5.39 * sx, y: 11.46 * sy),
            control1: CGPoint(x: 7.84 * sx, y: 16.39 * sy),
            control2: CGPoint(x: 5.39 * sx, y: 15.49 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 6.46 * sx, y: 8.63 * sy),
            control1: CGPoint(x: 5.39 * sx, y: 10.31 * sy),
            control2: CGPoint(x: 5.8 * sx, y: 9.37 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 6.56 * sx, y: 5.84 * sy),
            control1: CGPoint(x: 6.35 * sx, y: 8.37 * sy),
            control2: CGPoint(x: 6.0 * sx, y: 7.31 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 9.45 * sx, y: 6.94 * sy),
            control1: CGPoint(x: 6.56 * sx, y: 5.84 * sy),
            control2: CGPoint(x: 7.43 * sx, y: 5.56 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 14.55 * sx, y: 6.94 * sy),
            control1: CGPoint(x: 11.12 * sx, y: 6.48 * sy),
            control2: CGPoint(x: 12.88 * sx, y: 6.48 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 17.44 * sx, y: 5.84 * sy),
            control1: CGPoint(x: 16.57 * sx, y: 5.56 * sy),
            control2: CGPoint(x: 17.44 * sx, y: 5.84 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 17.54 * sx, y: 8.63 * sy),
            control1: CGPoint(x: 18.0 * sx, y: 7.31 * sy),
            control2: CGPoint(x: 17.65 * sx, y: 8.37 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 18.61 * sx, y: 11.46 * sy),
            control1: CGPoint(x: 18.2 * sx, y: 9.37 * sy),
            control2: CGPoint(x: 18.61 * sx, y: 10.31 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 13.82 * sx, y: 16.64 * sy),
            control1: CGPoint(x: 18.61 * sx, y: 15.5 * sy),
            control2: CGPoint(x: 16.15 * sx, y: 16.38 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 14.62 * sx, y: 18.61 * sy),
            control1: CGPoint(x: 14.29 * sx, y: 17.04 * sy),
            control2: CGPoint(x: 14.62 * sx, y: 17.78 * sy)
        )
        p.addLine(to: CGPoint(x: 14.62 * sx, y: 21.31 * sy))
        p.addCurve(
            to: CGPoint(x: 15.32 * sx, y: 21.96 * sy),
            control1: CGPoint(x: 14.62 * sx, y: 21.75 * sy),
            control2: CGPoint(x: 14.79 * sx, y: 22.06 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 22.5 * sx, y: 12 * sy),
            control1: CGPoint(x: 19.49 * sx, y: 20.57 * sy),
            control2: CGPoint(x: 22.5 * sx, y: 16.64 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 12 * sx, y: 1.5 * sy),
            control1: CGPoint(x: 22.5 * sx, y: 6.2 * sy),
            control2: CGPoint(x: 17.8 * sx, y: 1.5 * sy)
        )
        p.closeSubpath()
        return p
    }
}

private struct QQIconShape: Shape {
    func path(in rect: CGRect) -> Path {
        let sx = rect.width / 24.0
        let sy = rect.height / 24.0
        var p = Path()
        p.move(to: CGPoint(x: 12.0 * sx, y: 2.0 * sy))
        p.addCurve(
            to: CGPoint(x: 6.4 * sx, y: 8.9 * sy),
            control1: CGPoint(x: 8.4 * sx, y: 2.0 * sy),
            control2: CGPoint(x: 6.4 * sx, y: 5.0 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 4.1 * sx, y: 14.6 * sy),
            control1: CGPoint(x: 5.1 * sx, y: 10.6 * sy),
            control2: CGPoint(x: 4.0 * sx, y: 12.9 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 5.9 * sx, y: 16.0 * sy),
            control1: CGPoint(x: 4.2 * sx, y: 16.0 * sy),
            control2: CGPoint(x: 5.2 * sx, y: 16.5 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 7.4 * sx, y: 18.8 * sy),
            control1: CGPoint(x: 6.3 * sx, y: 17.2 * sy),
            control2: CGPoint(x: 6.8 * sx, y: 18.1 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 5.8 * sx, y: 20.6 * sy),
            control1: CGPoint(x: 6.3 * sx, y: 19.3 * sy),
            control2: CGPoint(x: 5.6 * sx, y: 20.0 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 10.8 * sx, y: 21.0 * sy),
            control1: CGPoint(x: 6.1 * sx, y: 21.5 * sy),
            control2: CGPoint(x: 8.9 * sx, y: 21.6 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 13.2 * sx, y: 21.0 * sy),
            control1: CGPoint(x: 11.6 * sx, y: 21.1 * sy),
            control2: CGPoint(x: 12.4 * sx, y: 21.1 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 18.2 * sx, y: 20.6 * sy),
            control1: CGPoint(x: 15.1 * sx, y: 21.6 * sy),
            control2: CGPoint(x: 17.9 * sx, y: 21.5 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 16.6 * sx, y: 18.8 * sy),
            control1: CGPoint(x: 18.4 * sx, y: 20.0 * sy),
            control2: CGPoint(x: 17.7 * sx, y: 19.3 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 18.1 * sx, y: 16.0 * sy),
            control1: CGPoint(x: 17.2 * sx, y: 18.1 * sy),
            control2: CGPoint(x: 17.7 * sx, y: 17.2 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 19.9 * sx, y: 14.6 * sy),
            control1: CGPoint(x: 18.8 * sx, y: 16.5 * sy),
            control2: CGPoint(x: 19.8 * sx, y: 16.0 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 17.6 * sx, y: 8.9 * sy),
            control1: CGPoint(x: 20.0 * sx, y: 12.9 * sy),
            control2: CGPoint(x: 18.9 * sx, y: 10.6 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 12.0 * sx, y: 2.0 * sy),
            control1: CGPoint(x: 17.6 * sx, y: 5.0 * sy),
            control2: CGPoint(x: 15.6 * sx, y: 2.0 * sy)
        )
        p.closeSubpath()
        return p
    }
}

private struct DiscordIconShape: Shape {
    func path(in rect: CGRect) -> Path {
        let sx = rect.width / 24.0
        let sy = rect.height / 18.0
        var p = Path()
        p.move(to: CGPoint(x: 20.32 * sx, y: 1.5 * sy))
        p.addCurve(
            to: CGPoint(x: 15.63 * sx, y: 0.05 * sy),
            control1: CGPoint(x: 18.82 * sx, y: 0.81 * sy),
            control2: CGPoint(x: 17.24 * sx, y: 0.32 * sy)
        )
        p.addLine(to: CGPoint(x: 15.0 * sx, y: 1.32 * sy))
        p.addCurve(
            to: CGPoint(x: 9.0 * sx, y: 1.32 * sy),
            control1: CGPoint(x: 13.02 * sx, y: 1.02 * sy),
            control2: CGPoint(x: 10.98 * sx, y: 1.02 * sy)
        )
        p.addLine(to: CGPoint(x: 8.37 * sx, y: 0.05 * sy))
        p.addCurve(
            to: CGPoint(x: 3.68 * sx, y: 1.5 * sy),
            control1: CGPoint(x: 6.76 * sx, y: 0.32 * sy),
            control2: CGPoint(x: 5.18 * sx, y: 0.81 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 0.56 * sx, y: 14.37 * sy),
            control1: CGPoint(x: 0.72 * sx, y: 5.91 * sy),
            control2: CGPoint(x: -0.09 * sx, y: 10.22 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 6.32 * sx, y: 17.28 * sy),
            control1: CGPoint(x: 2.26 * sx, y: 15.63 * sy),
            control2: CGPoint(x: 4.18 * sx, y: 16.61 * sy)
        )
        p.addLine(to: CGPoint(x: 7.55 * sx, y: 15.27 * sy))
        p.addCurve(
            to: CGPoint(x: 5.61 * sx, y: 14.33 * sy),
            control1: CGPoint(x: 6.88 * sx, y: 15.02 * sy),
            control2: CGPoint(x: 6.23 * sx, y: 14.71 * sy)
        )
        p.addLine(to: CGPoint(x: 6.09 * sx, y: 13.96 * sy))
        p.addCurve(
            to: CGPoint(x: 17.91 * sx, y: 13.96 * sy),
            control1: CGPoint(x: 9.85 * sx, y: 15.69 * sy),
            control2: CGPoint(x: 14.15 * sx, y: 15.69 * sy)
        )
        p.addLine(to: CGPoint(x: 18.39 * sx, y: 14.33 * sy))
        p.addCurve(
            to: CGPoint(x: 16.45 * sx, y: 15.27 * sy),
            control1: CGPoint(x: 17.77 * sx, y: 14.71 * sy),
            control2: CGPoint(x: 17.12 * sx, y: 15.02 * sy)
        )
        p.addLine(to: CGPoint(x: 17.68 * sx, y: 17.28 * sy))
        p.addCurve(
            to: CGPoint(x: 23.44 * sx, y: 14.37 * sy),
            control1: CGPoint(x: 19.82 * sx, y: 16.61 * sy),
            control2: CGPoint(x: 21.74 * sx, y: 15.63 * sy)
        )
        p.addCurve(
            to: CGPoint(x: 20.32 * sx, y: 1.5 * sy),
            control1: CGPoint(x: 24.2 * sx, y: 9.57 * sy),
            control2: CGPoint(x: 22.74 * sx, y: 5.3 * sy)
        )
        p.closeSubpath()

        p.addEllipse(in: CGRect(x: 6.4 * sx, y: 7.4 * sy, width: 3.4 * sx, height: 3.8 * sy))
        p.addEllipse(in: CGRect(x: 14.2 * sx, y: 7.4 * sy, width: 3.4 * sx, height: 3.8 * sy))
        return p
    }
}
