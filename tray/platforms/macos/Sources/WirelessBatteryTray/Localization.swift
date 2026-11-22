import CTrayCore
import Foundation

/// Resolved active display language for the macOS menu bar app.
enum AppLanguage: Equatable {
    case zhCn
    case en

    static func resolve(mode: String) -> AppLanguage {
        switch mode {
        case "zh_cn":
            return .zhCn
        case "en":
            return .en
        default:
            let preferred = Locale.preferredLanguages.first?.lowercased() ?? ""
            return preferred.hasPrefix("zh") ? .zhCn : .en
        }
    }

    var isZh: Bool { self == .zhCn }

    // MARK: - Main View

    var settingsTitle: String { isZh ? "设置" : "Settings" }
    var backLabel: String { isZh ? "返回" : "Back" }
    var quitLabel: String { isZh ? "退出" : "Quit" }
    var openPanelLabel: String { isZh ? "打开面板" : "Open Panel" }
    var menuThemeLightLabel: String { isZh ? "明亮模式" : "Light" }
    var menuThemeDarkLabel: String { isZh ? "暗黑模式" : "Dark" }
    var menuLanguageLabel: String { isZh ? "语言" : "Language" }
    var menuAutostartLabel: String { isZh ? "开机自启" : "Launch at startup" }
    var menuCheckUpdatesLabel: String { isZh ? "检查更新" : "Check for updates" }
    var readingPeripherals: String { isZh ? "正在读取外设电量…" : "Reading peripherals…" }

    var pinHelpPinned: String { isZh ? "取消置顶" : "Unpin" }
    var pinHelpUnpinned: String { isZh ? "固定在屏幕最上层（移出鼠标隐藏背景）" : "Pin floating panel (transparent when unfocused)" }
    var refreshAllHelp: String { isZh ? "立即刷新全部可读设备" : "Refresh all readable devices" }
    var settingsHelp: String { isZh ? "设置" : "Settings" }

    var emptyTitle: String { isZh ? "未发现可读取的无线外设" : "No readable wireless peripherals" }

    var emptyHint: String {
        isZh
            ? "被动枚举未匹配到已知接收器或外设，请确认接收器已接入并处于唤醒状态。"
            : "No known receiver or peripheral matched. Make sure the receiver is connected and awake."
    }

    var emptyAction: String { isZh ? "重新扫描" : "Rescan" }

    // MARK: - Expanded Device Drawer (no "Evidence" field)

    var labelProtocol: String { isZh ? "协议" : "Protocol" }
    var labelBus: String { isZh ? "总线" : "Bus" }
    var labelStatus: String { isZh ? "状态" : "Status" }

    var statusLive: String { isZh ? "实时" : "Live" }
    var statusStale: String { isZh ? "缓存（Stale）" : "Stale (cached)" }
    var statusUnread: String { isZh ? "未读取" : "Not read" }

    var btnRefreshDevice: String { isZh ? "重新读取" : "Refresh" }
    var btnCopyError: String { isZh ? "复制报错" : "Copy Error" }
    var toastErrorCopied: String { isZh ? "已复制报错信息" : "Error details copied" }

    // MARK: - Settings

    var settingThemeTitle: String { isZh ? "外观主题" : "Appearance" }
    var settingLanguageTitle: String { isZh ? "语言" : "Language" }
    var settingPollTitle: String { isZh ? "轮询频率" : "Poll Interval" }
    var settingPollDesc: String { isZh ? "休眠设备不主动唤醒" : "Sleeping devices are not woken" }

    var settingLowBatteryTitle: String { isZh ? "低电量警告" : "Low Battery Alert" }
    var settingSilentTitle: String { isZh ? "静默通知" : "Silent Notifications" }
    var settingSilentDesc: String { isZh ? "开启后不发送系统通知" : "Suppress macOS notifications" }

    var settingAutostartTitle: String { isZh ? "登录时静默自启" : "Launch at Login" }
    var settingAutostartDesc: String { isZh ? "仅驻留菜单栏，不弹出窗口" : "Start minimized in menu bar" }
    var settingAutostartHelpBundled: String { isZh ? "登录 macOS 时仅驻留菜单栏" : "Stay in the menu bar when logging in" }
    var settingAutostartHelpUnbundled: String { isZh ? "打包为 .app 后可用" : "Available when packaged as a .app bundle" }

    var settingVersionTitle: String { isZh ? "版本与更新" : "Version & Updates" }
    var settingVersionChecking: String { isZh ? "正在检查更新…" : "Checking for updates…" }
    var settingVersionFailed: String { isZh ? "检查更新失败，请重试" : "Update check failed" }
    var btnCheckUpdates: String { isZh ? "检查更新" : "Check" }
    var btnCheckingUpdates: String { isZh ? "检查中…" : "Checking…" }
    var btnDownloadUpdate: String { isZh ? "前往下载" : "Download" }
    var notifyUpdateTitle: String { isZh ? "检查更新" : "Update Check" }
    var notifyUpdateFailed: String {
        isZh ? "检查更新失败，请检查网络后重试。" : "Failed to check for updates. Please try again later."
    }

    func notifyUpdateAvailable(latestVersion: String) -> String {
        isZh ? "发现新版本 \(latestVersion)，正在打开下载页面。" : "New version \(latestVersion) is available. Opening download page."
    }

    func notifyUpdateUpToDate(version: String) -> String {
        isZh ? "当前已是最新版本（\(version)）。" : "You are up to date (\(version))."
    }

    func settingVersionDesc(_ version: String) -> String {
        isZh ? "当前版本 \(version)" : "Current version \(version)"
    }

    func settingVersionUpToDate(_ version: String) -> String {
        isZh ? "\(version) · 已是最新版本" : "\(version) · Up to date"
    }

    func settingVersionAvailable(_ latest: String) -> String {
        isZh ? "发现新版本 \(latest)" : "Update available: \(latest)"
    }

    func themeLabel(_ mode: String) -> String {
        switch mode {
        case "light": return isZh ? "明亮" : "Light"
        case "dark": return isZh ? "暗黑" : "Dark"
        default: return isZh ? "跟随系统" : "System"
        }
    }

    func languageLabel(_ mode: String) -> String {
        switch mode {
        case "zh_cn": return "简体中文"
        case "en": return "English"
        default: return isZh ? "跟随系统" : "System"
        }
    }

    func pollLabel(_ secs: UInt64) -> String {
        TrayStore.takeCString(wb_tray_format_poll_label(secs, isZh)) ?? "\(secs)s"
    }

    func lowBatteryDesc(enabled: Bool, silent: Bool) -> String {
        if !enabled {
            return isZh ? "已关闭低电量提醒与角标" : "Badge and alerts disabled"
        }
        if silent {
            return isZh ? "已静默，仅显示菜单栏低电角标" : "Silent: menu bar badge only"
        }
        return isZh ? "15% / 10% / 5% / 1% 时发送系统通知" : "Notify at 15%, 10%, 5%, 1%"
    }

    // MARK: - Notifications

    func notifyTitle(percentage: UInt8) -> String {
        if percentage <= 5 {
            return isZh ? "外设极低电量警报" : "Critical Peripheral Battery"
        }
        return isZh ? "外设低电量提醒" : "Low Peripheral Battery"
    }

    func notifyBody(deviceName: String, percentage: UInt8) -> String {
        if percentage <= 1 {
            return isZh
                ? "\(deviceName) 电量仅剩 \(percentage)%，即将耗尽，请立即充电。"
                : "\(deviceName) is at \(percentage)% battery and about to shut off. Charge now."
        }
        if percentage <= 5 {
            return isZh
                ? "\(deviceName) 电量仅剩 \(percentage)%，请尽快连接充电。"
                : "\(deviceName) is critically low (\(percentage)%). Please charge soon."
        }
        return isZh
            ? "\(deviceName) 电量仅剩 \(percentage)%，建议连接充电。"
            : "\(deviceName) battery is down to \(percentage)%. Consider charging."
    }

    // MARK: - Dynamic Formatters

    func footerSummary(appVersion: String, lastSyncAt: String?, pollSecs: UInt64) -> String {
        appVersion.withCString { verPtr in
            if let lastSyncAt {
                return lastSyncAt.withCString { syncPtr in
                    TrayStore.takeCString(wb_tray_format_footer_summary(verPtr, syncPtr, pollSecs, isZh))
                        ?? appVersion
                }
            } else {
                return TrayStore.takeCString(wb_tray_format_footer_summary(verPtr, nil, pollSecs, isZh))
                    ?? appVersion
            }
        }
    }

    func relativeTime(from raw: String?) -> String {
        if let raw {
            return raw.withCString { ptr in
                TrayStore.takeCString(wb_tray_format_relative_time(ptr, isZh))
                    ?? (isZh ? "未同步" : "Not synced")
            }
        }
        return TrayStore.takeCString(wb_tray_format_relative_time(nil, isZh))
            ?? (isZh ? "未同步" : "Not synced")
    }

    func discreteLabel(_ level: String?) -> String {
        switch level {
        case "full": return isZh ? "满" : "Full"
        case "high": return isZh ? "高" : "High"
        case "medium": return isZh ? "中" : "Medium"
        case "low": return isZh ? "低" : "Low"
        case "critical": return isZh ? "极低" : "Critical"
        default: return isZh ? "未知" : "Unknown"
        }
    }

    func errorShortLabel(_ error: String?) -> String {
        switch error {
        case "timeout": return isZh ? "读取超时" : "Timeout"
        case "device_busy": return isZh ? "接口正忙" : "Device busy"
        case "permission_denied": return isZh ? "权限不足" : "Permission denied"
        case "device_disconnected": return isZh ? "连接已断开" : "Disconnected"
        case "invalid_response", "checksum_failed": return isZh ? "响应校验失败" : "Invalid response"
        case "unsupported": return isZh ? "策略未启用" : "Unsupported"
        case .some: return isZh ? "读取失败" : "Read failed"
        case .none: return statusUnread
        }
    }

    func deviceSubtitle(for device: DeviceViewModel) -> String {
        var parts: [String] = []
        if device.chargeState == "charging" {
            parts.append(isZh ? "充电中" : "Charging")
        } else if device.reachability == "sleeping" {
            parts.append(isZh ? "休眠" : "Sleeping")
        } else if device.reachability == "offline" {
            parts.append(isZh ? "离线" : "Offline")
        } else if device.precision == "none" {
            switch device.lastError {
            case "timeout":
                parts.append(isZh ? "读取超时 · 请唤醒后重试" : "Timeout · Wake & retry")
            case "device_busy":
                parts.append(isZh ? "接口正忙 · 稍后重试" : "Busy · Retry shortly")
            case "permission_denied":
                parts.append(isZh ? "权限不足（请检查输入监控权限）" : "Permission denied (Input Monitoring)")
            case "device_disconnected":
                parts.append(isZh ? "连接已断开" : "Disconnected")
            case .some:
                parts.append(isZh ? "读取失败 · 点此重读" : "Read failed · Click to retry")
            case .none:
                parts.append(isZh ? "\(device.transportLabel) · 读取中" : "\(device.transportLabel) · Reading")
            }
        } else if device.precision == "discrete_level" {
            parts.append(isZh ? "离散档位" : "Discrete level")
        } else {
            parts.append(device.transportLabel)
        }

        if device.freshness == "stale" {
            let rel = relativeTime(from: device.observedAt)
            parts.append(isZh ? "缓存 · \(rel)" : "Cached · \(rel)")
        } else if device.precision == "percentage", let mv = device.voltageMv {
            parts.append(String(format: "%.2f V", Double(mv) / 1000.0))
        }
        return parts.joined(separator: " · ")
    }
}
