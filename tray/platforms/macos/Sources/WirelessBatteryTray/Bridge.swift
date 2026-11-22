import CTrayCore
import Foundation
import ServiceManagement
import UserNotifications

struct TrayConfigModel: Codable, Equatable {
    var pollIntervalSecs: UInt64
    var allowExperimental: Bool
    var allowStale: Bool
    var lowBatteryNotify: Bool
    var lowBatterySilent: Bool
    var lowBatteryThreshold: UInt8
    var autostart: Bool
    var readTimeoutMs: UInt64
    var themeMode: String
    var language: String

    enum CodingKeys: String, CodingKey {
        case pollIntervalSecs = "poll_interval_secs"
        case allowExperimental = "allow_experimental"
        case allowStale = "allow_stale"
        case lowBatteryNotify = "low_battery_notify"
        case lowBatterySilent = "low_battery_silent"
        case lowBatteryThreshold = "low_battery_threshold"
        case autostart
        case readTimeoutMs = "read_timeout_ms"
        case themeMode = "theme_mode"
        case language
    }

    init(
        pollIntervalSecs: UInt64,
        allowExperimental: Bool,
        allowStale: Bool,
        lowBatteryNotify: Bool,
        lowBatterySilent: Bool,
        lowBatteryThreshold: UInt8,
        autostart: Bool,
        readTimeoutMs: UInt64,
        themeMode: String,
        language: String
    ) {
        self.pollIntervalSecs = pollIntervalSecs
        self.allowExperimental = allowExperimental
        self.allowStale = allowStale
        self.lowBatteryNotify = lowBatteryNotify
        self.lowBatterySilent = lowBatterySilent
        self.lowBatteryThreshold = lowBatteryThreshold
        self.autostart = autostart
        self.readTimeoutMs = readTimeoutMs
        self.themeMode = themeMode
        self.language = language
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        self.pollIntervalSecs = try c.decodeIfPresent(UInt64.self, forKey: .pollIntervalSecs) ?? 300
        self.allowExperimental = try c.decodeIfPresent(Bool.self, forKey: .allowExperimental) ?? true
        self.allowStale = try c.decodeIfPresent(Bool.self, forKey: .allowStale) ?? true
        self.lowBatteryNotify = try c.decodeIfPresent(Bool.self, forKey: .lowBatteryNotify) ?? true
        self.lowBatterySilent = try c.decodeIfPresent(Bool.self, forKey: .lowBatterySilent) ?? false
        self.lowBatteryThreshold = try c.decodeIfPresent(UInt8.self, forKey: .lowBatteryThreshold) ?? 15
        self.autostart = try c.decodeIfPresent(Bool.self, forKey: .autostart) ?? false
        self.readTimeoutMs = try c.decodeIfPresent(UInt64.self, forKey: .readTimeoutMs) ?? 5000
        self.themeMode = try c.decodeIfPresent(String.self, forKey: .themeMode) ?? "system"
        self.language = try c.decodeIfPresent(String.self, forKey: .language) ?? "system"
    }

    static let `default` = TrayConfigModel(
        pollIntervalSecs: 300,
        allowExperimental: true,
        allowStale: true,
        lowBatteryNotify: true,
        lowBatterySilent: false,
        lowBatteryThreshold: 15,
        autostart: false,
        readTimeoutMs: 5000,
        themeMode: "system",
        language: "system"
    )
}

struct DeviceViewModel: Codable, Identifiable, Equatable {
    let id: String
    let name: String
    let kind: String
    let transportLabel: String
    let providerId: String?
    let supportStatus: String
    let precision: String
    let percentage: UInt8?
    let level: String?
    let voltageMv: UInt32?
    let chargeState: String
    let reachability: String
    let freshness: String?
    let observedAt: String?
    let lastError: String?
    let isLow: Bool

    enum CodingKeys: String, CodingKey {
        case id, name, kind, precision, percentage, level, reachability, freshness
        case transportLabel = "transport_label"
        case providerId = "provider_id"
        case supportStatus = "support_status"
        case voltageMv = "voltage_mv"
        case chargeState = "charge_state"
        case observedAt = "observed_at"
        case lastError = "last_error"
        case isLow = "is_low"
    }
}

struct TraySnapshotModel: Codable, Equatable {
    let appVersion: String
    let devices: [DeviceViewModel]
    let hiddenExperimentalCount: Int
    let iconState: String
    let tooltip: String
    let lastSyncAt: String?
    let refreshing: Bool
    var config: TrayConfigModel

    enum CodingKeys: String, CodingKey {
        case devices, tooltip, refreshing, config
        case appVersion = "app_version"
        case hiddenExperimentalCount = "hidden_experimental_count"
        case iconState = "icon_state"
        case lastSyncAt = "last_sync_at"
    }

    init(
        appVersion: String,
        devices: [DeviceViewModel],
        hiddenExperimentalCount: Int,
        iconState: String,
        tooltip: String,
        lastSyncAt: String?,
        refreshing: Bool,
        config: TrayConfigModel
    ) {
        self.appVersion = appVersion
        self.devices = devices
        self.hiddenExperimentalCount = hiddenExperimentalCount
        self.iconState = iconState
        self.tooltip = tooltip
        self.lastSyncAt = lastSyncAt
        self.refreshing = refreshing
        self.config = config
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        self.appVersion = try c.decodeIfPresent(String.self, forKey: .appVersion) ?? "v0.1.0"
        self.devices = try c.decode([DeviceViewModel].self, forKey: .devices)
        self.hiddenExperimentalCount = try c.decode(Int.self, forKey: .hiddenExperimentalCount)
        self.iconState = try c.decode(String.self, forKey: .iconState)
        self.tooltip = try c.decode(String.self, forKey: .tooltip)
        self.lastSyncAt = try c.decodeIfPresent(String.self, forKey: .lastSyncAt)
        self.refreshing = try c.decode(Bool.self, forKey: .refreshing)
        self.config = try c.decode(TrayConfigModel.self, forKey: .config)
    }

    static let empty = TraySnapshotModel(
        appVersion: "v0.1.0",
        devices: [],
        hiddenExperimentalCount: 0,
        iconState: "idle",
        tooltip: "Wireless Battery",
        lastSyncAt: nil,
        refreshing: false,
        config: .default
    )
}

enum UpdateCheckStatus: Equatable {
    case idle
    case checking
    case upToDate
    case available(latestVersion: String, url: URL)
    case failed
}

struct LowBatteryAlertModel: Codable {
    let deviceId: String
    let deviceName: String
    let percentage: UInt8

    enum CodingKeys: String, CodingKey {
        case deviceId = "device_id"
        case deviceName = "device_name"
        case percentage
    }
}

private struct UpdateEnvelope: Codable {
    let snapshot: TraySnapshotModel
    let alerts: [LowBatteryAlertModel]
}

final class TrayStore: ObservableObject {
    @Published var snapshot: TraySnapshotModel = .empty
    @Published var updateStatus: UpdateCheckStatus = .idle
    var handle: OpaquePointer?

    private static let isoWithFrac: ISO8601DateFormatter = {
        let fmt = ISO8601DateFormatter()
        fmt.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return fmt
    }()

    private static let isoPlain: ISO8601DateFormatter = {
        let fmt = ISO8601DateFormatter()
        fmt.formatOptions = [.withInternetDateTime]
        return fmt
    }()

    var isBundledApp: Bool {
        Bundle.main.bundleURL.pathExtension == "app"
    }

    var language: AppLanguage {
        AppLanguage.resolve(mode: snapshot.config.language)
    }

    init() {
        if isBundledApp {
            UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }
        }

        let ctx = Unmanaged.passUnretained(self).toOpaque()
        self.handle = wb_tray_create(trayUpdateTrampoline, ctx)
        if let raw = wb_tray_snapshot_json(handle) {
            defer { wb_tray_string_free(raw) }
            if let data = String(cString: raw).data(using: .utf8),
               let decoded = try? JSONDecoder().decode(TraySnapshotModel.self, from: data)
            {
                self.snapshot = decoded
                syncLoginItem(enabled: decoded.config.autostart)
            }
        }
    }

    deinit {
        wb_tray_destroy(handle)
    }

    fileprivate func handleRawUpdate(_ json: String) {
        guard let data = json.data(using: .utf8),
              let envelope = try? JSONDecoder().decode(UpdateEnvelope.self, from: data)
        else { return }
        DispatchQueue.main.async {
            self.snapshot = envelope.snapshot
            if envelope.snapshot.config.lowBatteryNotify && !envelope.snapshot.config.lowBatterySilent {
                for alert in envelope.alerts {
                    self.deliverNotification(alert)
                }
            }
        }
    }

    func refreshAll() {
        wb_tray_request_refresh(handle)
    }

    /// Opportunistically refresh when the user opens the menu bar popover if any device
    /// previously timed out / went stale or the last sync is more than 15 seconds old.
    func refreshIfNeededOnOpen() {
        guard !snapshot.refreshing else { return }
        let needsRetry = snapshot.devices.contains {
            $0.precision == "none" || $0.freshness == "stale"
        }
        let aged: Bool
        if let raw = snapshot.lastSyncAt,
           let date = Self.parseISO8601(raw)
        {
            aged = Date().timeIntervalSince(date) >= 15
        } else {
            aged = true
        }
        if needsRetry || aged {
            refreshAll()
        }
    }

    func refreshDevice(_ id: String) {
        id.withCString { wb_tray_request_device_refresh(handle, $0) }
    }

    func updateConfig(_ config: TrayConfigModel) {
        if config.autostart != snapshot.config.autostart {
            syncLoginItem(enabled: config.autostart)
        }
        // Apply optimistically on the main thread for instant UI feedback.
        self.snapshot.config = config
        guard let data = try? JSONEncoder().encode(config),
              let json = String(data: data, encoding: .utf8)
        else { return }
        json.withCString { _ = wb_tray_update_config_json(handle, $0) }
    }

    func errorReportJSON(for deviceId: String) -> String? {
        deviceId.withCString { cstr in
            guard let ptr = wb_tray_error_report_json(handle, cstr) else { return nil }
            defer { wb_tray_string_free(ptr) }
            return String(cString: ptr)
        }
    }

    func checkForUpdates() {
        guard updateStatus != .checking else { return }
        updateStatus = .checking

        let currentVersion = snapshot.appVersion
        guard let apiURL = URL(string: "https://api.github.com/repos/bangbang0103/wireless-battery/releases/latest") else {
            updateStatus = .failed
            return
        }
        var request = URLRequest(url: apiURL, timeoutInterval: 6)
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        request.setValue("wireless-battery-tray/\(currentVersion)", forHTTPHeaderField: "User-Agent")

        URLSession.shared.dataTask(with: request) { [weak self] data, response, error in
            guard let self else { return }
            let nextStatus: UpdateCheckStatus = {
                guard error == nil,
                      let http = response as? HTTPURLResponse,
                      (200..<300).contains(http.statusCode),
                      let data,
                      let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                      let rawTag = json["tag_name"] as? String
                else {
                    return .failed
                }
                let tag = rawTag.trimmingCharacters(in: .whitespacesAndNewlines)
                guard !tag.isEmpty else { return .failed }
                if Self.isNewerVersion(current: currentVersion, candidate: tag) {
                    let normalized = tag.lowercased().hasPrefix("v") ? "v\(tag.dropFirst())" : "v\(tag)"
                    let rawUrl = (json["html_url"] as? String) ?? "https://github.com/bangbang0103/wireless-battery/releases"
                    let url = URL(string: rawUrl) ?? URL(string: "https://github.com/bangbang0103/wireless-battery/releases")!
                    return .available(latestVersion: normalized, url: url)
                }
                return .upToDate
            }()

            DispatchQueue.main.async {
                self.updateStatus = nextStatus
            }
        }.resume()
    }

    private static func isNewerVersion(current: String, candidate: String) -> Bool {
        guard let cur = parseSemver(current), let cand = parseSemver(candidate) else {
            return false
        }
        return cand > cur
    }

    private static func parseSemver(_ raw: String) -> (Int, Int, Int)? {
        var s = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if s.hasPrefix("v") || s.hasPrefix("V") {
            s.removeFirst()
        }
        let core = s.split(whereSeparator: { $0 == "-" || $0 == "+" }).first.map(String.init) ?? s
        let parts = core.split(separator: ".")
        guard let major = parts.first.flatMap({ Int($0) }) else { return nil }
        let minor = parts.count > 1 ? (Int(parts[1]) ?? 0) : 0
        let patch = parts.count > 2 ? (Int(parts[2]) ?? 0) : 0
        return (major, minor, patch)
    }

    static func parseISO8601(_ raw: String) -> Date? {
        isoWithFrac.date(from: raw) ?? isoPlain.date(from: raw)
    }

    private func syncLoginItem(enabled: Bool) {
        guard isBundledApp else { return }
        do {
            if enabled {
                try SMAppService.mainApp.register()
            } else {
                try SMAppService.mainApp.unregister()
            }
        } catch {
            // Best effort: user can also manage Login Items in System Settings.
        }
    }

    func deliverNotification(_ alert: LowBatteryAlertModel) {
        guard isBundledApp else { return }
        let lang = AppLanguage.resolve(mode: snapshot.config.language)
        let tier = notificationTier(for: alert.percentage)
        let content = UNMutableNotificationContent()
        content.title = lang.notifyTitle(percentage: alert.percentage)
        content.body = lang.notifyBody(deviceName: alert.deviceName, percentage: alert.percentage)
        content.sound = .default
        let request = UNNotificationRequest(
            identifier: "wb-low-\(alert.deviceId)-\(tier)",
            content: content,
            trigger: nil
        )
        UNUserNotificationCenter.current().add(request)
    }

    private func notificationTier(for percentage: UInt8) -> UInt8 {
        if percentage <= 1 { return 1 }
        if percentage <= 5 { return 5 }
        if percentage <= 10 { return 10 }
        return 15
    }
}

private let trayUpdateTrampoline: WbTrayUpdateCallback = { cJson, userData in
    guard let cJson, let userData else { return }
    let store = Unmanaged<TrayStore>.fromOpaque(userData).takeUnretainedValue()
    store.handleRawUpdate(String(cString: cJson))
}
