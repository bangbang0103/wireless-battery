#if DEV_TOOLS
import AppKit
import CTrayCore
import Foundation
import SwiftUI

private struct DevDevicePayload: Encodable {
    let name: String
    let kind: String
    let percentage: UInt8?
    let state: String
}

extension TrayStore {
    @discardableResult
    func devToggleLowBattery() -> Bool {
        wb_tray_dev_toggle_low_battery(handle)
    }

    func devSendLowBatteryAlert(percentage: UInt8) {
        wb_tray_dev_send_low_battery_alert(handle, percentage)
        // When launched via `swift run` (unbundled), UNUserNotificationCenter is unavailable;
        // fall back to an AppleScript notification so alerts are still testable in dev mode.
        if !isBundledApp {
            let deviceName = snapshot.devices.first?.name ?? "Dev Mock Mouse"
            let lang = AppLanguage.resolve(mode: snapshot.config.language)
            let title = lang.notifyTitle(percentage: percentage)
            let body = lang.notifyBody(deviceName: deviceName, percentage: percentage)
            Self.postUnbundledNotification(title: title, body: body)
        }
    }

    func devAddDevice(name: String, kind: String, percentage: UInt8?, state: String) {
        let payload = DevDevicePayload(
            name: name,
            kind: kind,
            percentage: percentage,
            state: state
        )
        guard let data = try? JSONEncoder().encode(payload),
              let json = String(data: data, encoding: .utf8)
        else { return }
        json.withCString { _ = wb_tray_dev_add_device_json(handle, $0) }
    }

    func devAddPresetSuite() {
        devAddDevice(name: "Dev Pro Mouse", kind: "mouse", percentage: 88, state: "live")
        devAddDevice(name: "Dev Mechanical Keyboard", kind: "keyboard", percentage: 45, state: "charging")
        devAddDevice(name: "Dev Wireless Headset", kind: "headset", percentage: 12, state: "live")
        devAddDevice(name: "Dev Gamepad", kind: "gamepad", percentage: 60, state: "stale")
    }

    func devReset() {
        wb_tray_dev_reset(handle)
    }

    private static func postUnbundledNotification(title: String, body: String) {
        let escapedTitle = title.replacingOccurrences(of: "\"", with: "\\\"")
        let escapedBody = body.replacingOccurrences(of: "\"", with: "\\\"")
        let script = "display notification \"\(escapedBody)\" with title \"\(escapedTitle)\""
        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
        proc.arguments = ["-e", script]
        try? proc.run()
    }
}

final class DevPanelController {
    static let shared = DevPanelController()

    private var panel: NSPanel?
    private var didAutoShow = false

    func showIfNeeded(store: TrayStore) {
        guard !didAutoShow else { return }
        didAutoShow = true
        show(store: store)
    }

    func show(store: TrayStore) {
        if let existing = panel {
            existing.orderFrontRegardless()
            return
        }

        let content = DevToolsPanelView(store: store)
        let hosting = NSHostingController(rootView: content)

        let panel = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: 340, height: 390),
            styleMask: [.titled, .closable, .utilityWindow],
            backing: .buffered,
            defer: false
        )
        panel.title = "Wireless Battery — Dev 调试面板"
        panel.contentViewController = hosting
        panel.isFloatingPanel = true
        panel.level = .floating
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false

        if let screen = NSScreen.main {
            let visible = screen.visibleFrame
            let origin = NSPoint(x: visible.maxX - 380, y: visible.maxY - 430)
            panel.setFrameOrigin(origin)
        }

        panel.orderFrontRegardless()
        self.panel = panel
    }
}

private struct DevToolsPanelView: View {
    @ObservedObject var store: TrayStore

    @State private var lowBatteryForced = false
    @State private var deviceName = "Dev Wireless Mouse"
    @State private var deviceKind = "mouse"
    @State private var percentageText = "85"
    @State private var deviceState = "live"
    @State private var statusMessage = "Dev 模式已就绪"

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            GroupBox("状态与通知调试") {
                VStack(spacing: 8) {
                    Button {
                        lowBatteryForced = store.devToggleLowBattery()
                        statusMessage = lowBatteryForced
                            ? "已进入低电量状态（菜单栏波纹已变红）"
                            : "已退出强制低电量状态"
                    } label: {
                        Text(lowBatteryForced ? "退出低电量状态 (当前: 低电量)" : "进入低电量状态")
                            .frame(maxWidth: .infinity)
                    }

                    HStack(spacing: 8) {
                        Button("发送低电量通知 (10%)") {
                            store.devSendLowBatteryAlert(percentage: 10)
                            statusMessage = "已发送低电量通知 (10%)"
                        }
                        .frame(maxWidth: .infinity)

                        Button("严重低电 (5%)") {
                            store.devSendLowBatteryAlert(percentage: 5)
                            statusMessage = "已发送严重低电通知 (5%)"
                        }
                        .frame(maxWidth: .infinity)
                    }
                }
                .padding(.top, 4)
            }

            GroupBox("手动添加模拟设备") {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("名称").frame(width: 36, alignment: .leading)
                        TextField("设备名称", text: $deviceName)
                            .textFieldStyle(.roundedBorder)
                    }

                    HStack {
                        Text("类型").frame(width: 36, alignment: .leading)
                        Picker("", selection: $deviceKind) {
                            Text("鼠标").tag("mouse")
                            Text("键盘").tag("keyboard")
                            Text("耳机").tag("headset")
                            Text("手柄").tag("gamepad")
                        }
                        .labelsHidden()

                        Text("电量%")
                        TextField("85", text: $percentageText)
                            .textFieldStyle(.roundedBorder)
                            .frame(width: 56)
                    }

                    HStack {
                        Text("状态").frame(width: 36, alignment: .leading)
                        Picker("", selection: $deviceState) {
                            Text("正常放电 (Live)").tag("live")
                            Text("充电中 (Charging)").tag("charging")
                            Text("休眠缓存 (Stale)").tag("stale")
                            Text("读取失败 (Error)").tag("error")
                        }
                        .labelsHidden()
                    }

                    Button {
                        let parsed = UInt8(percentageText.trimmingCharacters(in: .whitespaces))
                        let pct: UInt8? = deviceState == "error" ? nil : min(parsed ?? 50, 100)
                        store.devAddDevice(
                            name: deviceName,
                            kind: deviceKind,
                            percentage: pct,
                            state: deviceState
                        )
                        statusMessage = "已添加模拟设备：\(deviceName)"
                    } label: {
                        Text("+ 添加设备到列表")
                            .frame(maxWidth: .infinity)
                    }
                }
                .padding(.top, 4)
            }

            HStack(spacing: 8) {
                Button("填充多设备场景") {
                    store.devAddPresetSuite()
                    statusMessage = "已填充 4 台多状态测试设备"
                }
                .frame(maxWidth: .infinity)

                Button("清空 / 重置") {
                    store.devReset()
                    lowBatteryForced = false
                    statusMessage = "已清空虚拟设备并恢复真实状态"
                }
                .frame(maxWidth: .infinity)
            }

            Text(statusMessage)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(2)
        }
        .padding(14)
        .frame(width: 340)
    }
}
#endif
