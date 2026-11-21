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
}

private struct MenuBarLogoIcon: View {
    let state: String
    let tooltip: String

    var body: some View {
        Canvas { context, size in
            let s = min(size.width, size.height) / 24.0
            let ox = (size.width - 24.0 * s) * 0.5
            let oy = (size.height - 24.0 * s) * 0.5
            let frameOpacity = state == "idle" ? 0.48 : 1.0
            let frameColor = Color.primary.opacity(frameOpacity)
            let signalColor: Color = {
                switch state {
                case "idle":
                    return frameColor
                case "low_battery":
                    return Color(red: 244 / 255, green: 63 / 255, blue: 94 / 255)
                default:
                    return Color(red: 16 / 255, green: 185 / 255, blue: 129 / 255)
                }
            }()

            // Minimalist horizontal battery outline (1.5, 5.5, 18 x 13, rx 2.8)
            let bodyRect = CGRect(x: ox + 1.5 * s, y: oy + 5.5 * s, width: 18.0 * s, height: 13.0 * s)
            let bodyPath = Path(roundedRect: bodyRect, cornerRadius: 2.8 * s)
            context.stroke(
                bodyPath,
                with: .color(frameColor),
                style: StrokeStyle(lineWidth: max(1.25, 1.8 * s), lineCap: .round, lineJoin: .round)
            )

            // Positive terminal pill (21.0, 9.2, 2.0 x 5.6, rx 1.0)
            let capRect = CGRect(x: ox + 21.0 * s, y: oy + 9.2 * s, width: 2.0 * s, height: 5.6 * s)
            context.fill(Path(roundedRect: capRect, cornerRadius: 1.0 * s), with: .color(frameColor))

            // Concentric wireless Wi-Fi arcs centered at (10.5, 14.6)
            let center = CGPoint(x: ox + 10.5 * s, y: oy + 14.6 * s)
            let waveStyle = StrokeStyle(lineWidth: max(1.15, 1.7 * s), lineCap: .round, lineJoin: .round)
            for radius in [5.6 * s, 3.2 * s] {
                var arc = Path()
                arc.addArc(
                    center: center,
                    radius: radius,
                    startAngle: .degrees(-132),
                    endAngle: .degrees(-48),
                    clockwise: false
                )
                context.stroke(arc, with: .color(signalColor), style: waveStyle)
            }

            // Wi-Fi base dot
            let dotR = max(0.85, 1.15 * s)
            let dotRect = CGRect(x: center.x - dotR, y: center.y - dotR, width: dotR * 2, height: dotR * 2)
            context.fill(Path(ellipseIn: dotRect), with: .color(signalColor))

            // Low-battery top-right badge dot
            if state == "low_battery" {
                let badgeR = max(1.8, 2.4 * s)
                let badgeRect = CGRect(
                    x: ox + 20.2 * s - badgeR,
                    y: oy + 4.2 * s - badgeR,
                    width: badgeR * 2,
                    height: badgeR * 2
                )
                context.fill(Path(ellipseIn: badgeRect), with: .color(signalColor))
            }
        }
        .frame(width: 20, height: 20)
        .help(tooltip)
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

    private var showChrome: Bool {
        !inFloatingPanel || isHovering || showSettings || store.snapshot.devices.isEmpty
    }

    var body: some View {
        VStack(spacing: 0) {
            if showSettings {
                settingsView
            } else {
                mainView
            }
        }
        .background(.ultraThinMaterial)
        .animation(.easeInOut(duration: 0.16), value: showChrome)
        .onHover { hovering in
            isHovering = hovering
        }
        .onAppear {
            store.refreshIfNeededOnOpen()
        }
    }

    private var mainView: some View {
        VStack(spacing: 0) {
            if showChrome {
                // Header
                HStack(spacing: 8) {
                    Circle()
                        .fill(statusColor)
                        .frame(width: 7, height: 7)
                    Text("Wireless Battery")
                        .font(.system(size: 13, weight: .semibold))
                    Text(store.snapshot.refreshing ? "· 读取中…" : "· \(TrayStore.relativeTime(from: store.snapshot.lastSyncAt))")
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                    Spacer()
                    Button {
                        if inFloatingPanel {
                            pinController.unpin()
                        } else {
                            pinController.toggle(store: store)
                        }
                    } label: {
                        Image(systemName: pinController.isPinned ? "pin.fill" : "pin")
                            .font(.system(size: 12, weight: .medium))
                            .foregroundStyle(pinController.isPinned ? Color.green : Color.secondary)
                    }
                    .buttonStyle(.borderless)
                    .help(pinController.isPinned ? "取消置顶" : "固定在屏幕最上层（可拖动）")
                    Button { store.refreshAll() } label: {
                        Image(systemName: "arrow.clockwise")
                            .font(.system(size: 12, weight: .medium))
                    }
                    .buttonStyle(.borderless)
                    .help("立即刷新全部可读设备")
                    Button { showSettings = true } label: {
                        Image(systemName: "gearshape")
                            .font(.system(size: 12, weight: .medium))
                    }
                    .buttonStyle(.borderless)
                    .help("偏好与安全策略")
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 11)

                Divider()
            }

            // Device list
            if store.snapshot.devices.isEmpty {
                emptyState
            } else {
                ScrollView {
                    LazyVStack(spacing: 6) {
                        ForEach(store.snapshot.devices) { device in
                            DeviceCardView(
                                device: device,
                                expanded: expandedId == device.id,
                                onToggle: {
                                    withAnimation(.easeInOut(duration: 0.16)) {
                                        expandedId = expandedId == device.id ? nil : device.id
                                    }
                                },
                                onRefresh: { store.refreshDevice(device.id) },
                                onCopyDiagnose: {
                                    if let json = store.diagnoseJSON(for: device.id) {
                                        NSPasteboard.general.clearContents()
                                        NSPasteboard.general.setString(json, forType: .string)
                                        copiedBanner = true
                                        DispatchQueue.main.asyncAfter(deadline: .now() + 1.8) {
                                            copiedBanner = false
                                        }
                                    }
                                }
                            )
                        }
                    }
                    .padding(10)
                }
                .frame(maxHeight: 360)
            }

            if copiedBanner {
                Text("已复制脱敏诊断 JSON")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(.white)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 4)
                    .background(Color.green.opacity(0.9), in: Capsule())
                    .padding(.bottom, 6)
            }

            if showChrome {
                Divider()

                // Footer
                HStack {
                    Text(policyLabel)
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                    Spacer()
                    Button("退出") { NSApp.terminate(nil) }
                        .buttonStyle(.borderless)
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 9)
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 8) {
            Image(systemName: "battery.75")
                .font(.system(size: 24))
                .foregroundStyle(.secondary)
            Text("未发现可读取的无线外设")
                .font(.system(size: 12, weight: .medium))
            if store.snapshot.hiddenExperimentalCount > 0 {
                Text("检测到 \(store.snapshot.hiddenExperimentalCount) 台实验性协议设备，默认未启用，可在设置中开启。")
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                Button("前往开启实验性协议") { showSettings = true }
                    .controlSize(.small)
            } else {
                Text("被动枚举未匹配到已知接收器或外设，请确认接收器已接入并处于唤醒状态。")
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                Button("重新扫描端点") { store.refreshAll() }
                    .controlSize(.small)
            }
        }
        .padding(.vertical, 28)
        .padding(.horizontal, 20)
    }

    private var settingsView: some View {
        let cfg = store.snapshot.config
        return VStack(spacing: 0) {
            HStack {
                Button { showSettings = false } label: {
                    Label("返回", systemImage: "chevron.left")
                        .font(.system(size: 12, weight: .medium))
                }
                .buttonStyle(.borderless)
                Spacer()
                Text("偏好与安全策略")
                    .font(.system(size: 13, weight: .semibold))
                Spacer()
                Color.clear.frame(width: 44, height: 1)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 11)

            Divider()

            VStack(alignment: .leading, spacing: 14) {
                VStack(alignment: .leading, spacing: 6) {
                    HStack {
                        Text("轮询频率").font(.system(size: 12, weight: .medium))
                        Spacer()
                        Text("休眠设备不主动唤醒")
                            .font(.system(size: 10.5))
                            .foregroundStyle(.secondary)
                    }
                    Picker("", selection: Binding(
                        get: { cfg.pollIntervalSecs },
                        set: { next in
                            var updated = cfg
                            updated.pollIntervalSecs = next
                            store.updateConfig(updated)
                        }
                    )) {
                        Text("1 分钟").tag(UInt64(60))
                        Text("5 分钟").tag(UInt64(300))
                        Text("15 分钟").tag(UInt64(900))
                        Text("手动").tag(UInt64(0))
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                }

                Divider()

                Toggle("允许实验性协议（Experimental）", isOn: Binding(
                    get: { cfg.allowExperimental },
                    set: { v in var u = cfg; u.allowExperimental = v; store.updateConfig(u) }
                ))
                Toggle("失败时保留上次缓存（Stale）", isOn: Binding(
                    get: { cfg.allowStale },
                    set: { v in var u = cfg; u.allowStale = v; store.updateConfig(u) }
                ))
                Toggle("低电量系统提醒（≤ 15%）", isOn: Binding(
                    get: { cfg.lowBatteryNotify },
                    set: { v in var u = cfg; u.lowBatteryNotify = v; store.updateConfig(u) }
                ))
                Toggle("登录时静默自启", isOn: Binding(
                    get: { cfg.autostart },
                    set: { v in var u = cfg; u.autostart = v; store.updateConfig(u) }
                ))
                .disabled(!store.isBundledApp)
                .help(store.isBundledApp ? "登录 macOS 时仅驻留菜单栏" : "打包为 .app 后可用")
            }
            .font(.system(size: 12))
            .toggleStyle(.switch)
            .controlSize(.small)
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

    private var policyLabel: String {
        let secs = store.snapshot.config.pollIntervalSecs
        if secs == 0 { return "仅手动刷新" }
        return secs % 60 == 0 ? "每 \(secs / 60) 分钟静默轮询" : "每 \(secs) 秒静默轮询"
    }
}

private struct DeviceCardView: View {
    let device: DeviceViewModel
    let expanded: Bool
    let onToggle: () -> Void
    let onRefresh: () -> Void
    let onCopyDiagnose: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                ZStack {
                    RoundedRectangle(cornerRadius: 8)
                        .fill(Color.primary.opacity(0.06))
                        .frame(width: 34, height: 34)
                    Image(systemName: symbolName)
                        .font(.system(size: 14))
                }

                VStack(alignment: .leading, spacing: 2) {
                    Text(device.name)
                        .font(.system(size: 13, weight: .medium))
                        .lineLimit(1)
                    Text(subtitle)
                        .font(.system(size: 11))
                        .foregroundStyle(subtitleColor)
                        .lineLimit(1)
                }

                Spacer()

                readoutView
                    .frame(width: 70, alignment: .trailing)
            }
            .contentShape(Rectangle())
            .onTapGesture(perform: onToggle)

            if expanded {
                Divider().padding(.vertical, 8)
                VStack(spacing: 8) {
                    HStack {
                        Text("协议").foregroundStyle(.secondary)
                        Spacer()
                        Text(device.providerId ?? "--").font(.system(size: 10.5, design: .monospaced))
                    }
                    HStack {
                        Text("证据").foregroundStyle(.secondary)
                        Spacer()
                        Text(evidenceLabel)
                            .foregroundStyle(device.supportStatus == "verified" ? .green : .orange)
                    }
                    HStack {
                        Text("总线").foregroundStyle(.secondary)
                        Spacer()
                        Text(device.transportLabel)
                    }
                    HStack {
                        Text("状态").foregroundStyle(.secondary)
                        Spacer()
                        Text(expandedStatusLabel)
                    }
                    HStack {
                        Spacer()
                        Button("重新读取", action: onRefresh).controlSize(.small)
                        Button("复制脱敏诊断", action: onCopyDiagnose).controlSize(.small)
                    }
                }
                .font(.system(size: 11))
            }
        }
        .padding(.horizontal, 11)
        .padding(.vertical, 9)
        .background(Color.primary.opacity(expanded ? 0.07 : 0.035), in: RoundedRectangle(cornerRadius: 10))
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
                        Capsule().fill(Color.primary.opacity(0.1))
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
                Text(discreteLabel).font(.system(size: 12, weight: .semibold))
                HStack(spacing: 3) {
                    ForEach(0..<4, id: \.self) { i in
                        Capsule()
                            .fill(i < segs ? barColor : Color.primary.opacity(0.1))
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

    private var subtitle: String {
        var parts: [String] = []
        if device.chargeState == "charging" {
            parts.append("充电中")
        } else if device.reachability == "sleeping" {
            parts.append("休眠")
        } else if device.reachability == "offline" {
            parts.append("离线")
        } else if device.precision == "none" {
            switch device.lastError {
            case "timeout": parts.append("读取超时 · 请唤醒后重试")
            case "device_busy": parts.append("接口正忙 · 稍后重试")
            case "permission_denied": parts.append("权限不足（请检查输入监控权限）")
            case "device_disconnected": parts.append("连接已断开")
            case .some: parts.append("读取失败 · 点此重读")
            case .none: parts.append("\(device.transportLabel) · 读取中")
            }
        } else if device.precision == "discrete_level" {
            parts.append("离散档位")
        } else {
            parts.append(device.transportLabel)
        }

        if device.freshness == "stale" {
            parts.append("缓存 · \(TrayStore.relativeTime(from: device.observedAt))")
        } else if device.precision == "percentage", let mv = device.voltageMv {
            parts.append(String(format: "%.2f V", Double(mv) / 1000.0))
        }
        return parts.joined(separator: " · ")
    }

    private var evidenceLabel: String {
        switch device.supportStatus {
        case "verified": return "Verified"
        case "experimental": return "Experimental"
        case "deprecated": return "Deprecated"
        default: return "Unknown"
        }
    }

    private var expandedStatusLabel: String {
        if device.freshness == "stale" { return "Stale（缓存）" }
        if device.freshness == "live" { return "Live（实时）" }
        switch device.lastError {
        case "timeout": return "读取超时"
        case "device_busy": return "接口正忙"
        case "permission_denied": return "权限不足"
        case "device_disconnected": return "连接已断开"
        case .some: return "读取失败"
        case .none: return "未读取"
        }
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

    private var discreteLabel: String {
        switch device.level {
        case "full": return "满"
        case "high": return "高"
        case "medium": return "中"
        case "low": return "低"
        case "critical": return "极低"
        default: return "未知"
        }
    }
}
