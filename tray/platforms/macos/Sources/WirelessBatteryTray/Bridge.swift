import AppKit
import CTrayCore
import Foundation
import ServiceManagement
import UserNotifications

struct TrayConfigModel: Codable, Equatable {
    var pollIntervalSecs: UInt64
    var allowStale: Bool
    var lowBatteryNotify: Bool
    var lowBatterySilent: Bool
    var lowBatteryThreshold: UInt8
    var autostart: Bool
    var showPinButton: Bool
    var readTimeoutMs: UInt64
    var themeMode: String
    var language: String

    enum CodingKeys: String, CodingKey {
        case pollIntervalSecs = "poll_interval_secs"
        case allowStale = "allow_stale"
        case lowBatteryNotify = "low_battery_notify"
        case lowBatterySilent = "low_battery_silent"
        case lowBatteryThreshold = "low_battery_threshold"
        case autostart
        case showPinButton = "show_pin_button"
        case readTimeoutMs = "read_timeout_ms"
        case themeMode = "theme_mode"
        case language
    }

    init(
        pollIntervalSecs: UInt64,
        allowStale: Bool,
        lowBatteryNotify: Bool,
        lowBatterySilent: Bool,
        lowBatteryThreshold: UInt8,
        autostart: Bool,
        showPinButton: Bool,
        readTimeoutMs: UInt64,
        themeMode: String,
        language: String
    ) {
        self.pollIntervalSecs = pollIntervalSecs
        self.allowStale = allowStale
        self.lowBatteryNotify = lowBatteryNotify
        self.lowBatterySilent = lowBatterySilent
        self.lowBatteryThreshold = lowBatteryThreshold
        self.autostart = autostart
        self.showPinButton = showPinButton
        self.readTimeoutMs = readTimeoutMs
        self.themeMode = themeMode
        self.language = language
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        self.pollIntervalSecs = try c.decodeIfPresent(UInt64.self, forKey: .pollIntervalSecs) ?? 300
        self.allowStale = try c.decodeIfPresent(Bool.self, forKey: .allowStale) ?? true
        self.lowBatteryNotify = try c.decodeIfPresent(Bool.self, forKey: .lowBatteryNotify) ?? true
        self.lowBatterySilent = try c.decodeIfPresent(Bool.self, forKey: .lowBatterySilent) ?? false
        self.lowBatteryThreshold = try c.decodeIfPresent(UInt8.self, forKey: .lowBatteryThreshold) ?? 15
        self.autostart = try c.decodeIfPresent(Bool.self, forKey: .autostart) ?? false
        self.showPinButton = try c.decodeIfPresent(Bool.self, forKey: .showPinButton) ?? false
        self.readTimeoutMs = try c.decodeIfPresent(UInt64.self, forKey: .readTimeoutMs) ?? 5000
        self.themeMode = try c.decodeIfPresent(String.self, forKey: .themeMode) ?? "system"
        self.language = try c.decodeIfPresent(String.self, forKey: .language) ?? "system"
    }

    static let `default` = TrayConfigModel(
        pollIntervalSecs: 300,
        allowStale: true,
        lowBatteryNotify: true,
        lowBatterySilent: false,
        lowBatteryThreshold: 15,
        autostart: false,
        showPinButton: false,
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
    let iconState: String
    let tooltip: String
    let lastSyncAt: String?
    let refreshing: Bool
    var config: TrayConfigModel

    enum CodingKeys: String, CodingKey {
        case devices, tooltip, refreshing, config
        case appVersion = "app_version"
        case iconState = "icon_state"
        case lastSyncAt = "last_sync_at"
    }

    init(
        appVersion: String,
        devices: [DeviceViewModel],
        iconState: String,
        tooltip: String,
        lastSyncAt: String?,
        refreshing: Bool,
        config: TrayConfigModel
    ) {
        self.appVersion = appVersion
        self.devices = devices
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
        self.iconState = try c.decode(String.self, forKey: .iconState)
        self.tooltip = try c.decode(String.self, forKey: .tooltip)
        self.lastSyncAt = try c.decodeIfPresent(String.self, forKey: .lastSyncAt)
        self.refreshing = try c.decode(Bool.self, forKey: .refreshing)
        self.config = try c.decode(TrayConfigModel.self, forKey: .config)
    }

    static let empty = TraySnapshotModel(
        appVersion: "v0.1.0",
        devices: [],
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

struct CommunityLinksModel: Codable, Equatable {
    let repoUrl: String
    let issuesUrl: String
    let qqGroup: String
    let qqUrl: String
    let discordUrl: String

    static let fallback = CommunityLinksModel(
        repoUrl: "https://github.com/bangbang0103/wireless-battery",
        issuesUrl: "https://github.com/bangbang0103/wireless-battery/issues",
        qqGroup: "",
        qqUrl: "",
        discordUrl: ""
    )

    static func loadFromCore() -> CommunityLinksModel {
        guard let ptr = wb_tray_community_links_json() else {
            return .fallback
        }
        defer { wb_tray_string_free(ptr) }
        let json = String(cString: ptr)
        guard let data = json.data(using: .utf8),
              let parsed = try? JSONDecoder().decode(CommunityLinksModel.self, from: data)
        else {
            return .fallback
        }
        return parsed
    }
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

private struct ReleaseEvaluationResponse: Decodable {
    let status: String
    let latestVersion: String?
    let url: String?

    enum CodingKeys: String, CodingKey {
        case status
        case latestVersion = "latest_version"
        case url
    }
}

private final class CallbackBox {
    private let lock = NSLock()
    private weak var store: TrayStore?

    init(store: TrayStore) {
        self.store = store
    }

    func deliver(_ json: String) {
        lock.lock()
        let target = store
        lock.unlock()
        target?.handleRawUpdate(json)
    }

    func invalidate() {
        lock.lock()
        store = nil
        lock.unlock()
    }
}

private final class NotificationDelegate: NSObject, UNUserNotificationCenterDelegate {
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .sound])
    }
}

final class TrayStore: ObservableObject {
    @Published var snapshot: TraySnapshotModel = .empty
    @Published var updateStatus: UpdateCheckStatus = .idle
    let communityLinks: CommunityLinksModel = CommunityLinksModel.loadFromCore()
    var handle: OpaquePointer?
    private let notificationDelegate = NotificationDelegate()
    private var callbackBox: CallbackBox?
    private var terminationObserver: NSObjectProtocol?

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

    static func takeCString(_ ptr: UnsafeMutablePointer<CChar>?) -> String? {
        guard let ptr else { return nil }
        defer { wb_tray_string_free(ptr) }
        return String(cString: ptr)
    }

    static func isValidUpdateURL(_ url: URL) -> Bool {
        url.absoluteString.withCString { wb_tray_is_valid_update_url($0) }
    }

    init() {
        if isBundledApp {
            let center = UNUserNotificationCenter.current()
            center.delegate = notificationDelegate
            center.requestAuthorization(options: [.alert, .sound]) { _, _ in }
        }

        let box = CallbackBox(store: self)
        self.callbackBox = box
        let ctx = Unmanaged.passRetained(box).toOpaque()
        let created = wb_tray_create(trayUpdateTrampoline, ctx)
        self.handle = created
        if let created {
            wb_tray_set_system_language_zh(created, AppLanguage.resolve(mode: "system").isZh)
            if let snapshotJson = Self.takeCString(wb_tray_snapshot_json(created)),
               let data = snapshotJson.data(using: .utf8),
               let decoded = try? JSONDecoder().decode(TraySnapshotModel.self, from: data)
            {
                self.snapshot = decoded
                syncLoginItem(enabled: decoded.config.autostart)
            }
        } else {
            Unmanaged<CallbackBox>.fromOpaque(ctx).release()
            self.callbackBox = nil
        }

        self.terminationObserver = NotificationCenter.default.addObserver(
            forName: NSApplication.willTerminateNotification,
            object: nil,
            queue: nil
        ) { [weak self] _ in
            self?.shutdown()
        }
    }

    deinit {
        if let observer = terminationObserver {
            NotificationCenter.default.removeObserver(observer)
            terminationObserver = nil
        }
        shutdown()
    }

    func shutdown() {
        callbackBox?.invalidate()
        if let h = handle {
            handle = nil
            wb_tray_destroy(h)
        }
        if let box = callbackBox {
            callbackBox = nil
            Unmanaged.passUnretained(box).release()
        }
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
        guard let apiUrlString = Self.takeCString(wb_tray_latest_release_api_url()),
              let apiURL = URL(string: apiUrlString)
        else {
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
                      let body = String(data: data, encoding: .utf8)
                else {
                    return .failed
                }
                let evalJson: String? = body.withCString { bodyPtr in
                    currentVersion.withCString { verPtr in
                        Self.takeCString(wb_tray_evaluate_release_json(bodyPtr, verPtr))
                    }
                }
                guard let evalData = evalJson?.data(using: .utf8),
                      let eval = try? JSONDecoder().decode(ReleaseEvaluationResponse.self, from: evalData)
                else {
                    return .failed
                }
                switch eval.status {
                case "available":
                    let fallback = Self.takeCString(wb_tray_releases_url()).flatMap(URL.init(string:))
                    let candidate = eval.url
                        .flatMap(URL.init(string:))
                        .flatMap { Self.isValidUpdateURL($0) ? $0 : nil }
                    guard let url = candidate ?? fallback else {
                        return .failed
                    }
                    return .available(latestVersion: eval.latestVersion ?? "", url: url)
                case "up_to_date":
                    return .upToDate
                default:
                    return .failed
                }
            }()

            DispatchQueue.main.async {
                self.updateStatus = nextStatus
            }
        }.resume()
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
    let box = Unmanaged<CallbackBox>.fromOpaque(userData).takeUnretainedValue()
    box.deliver(String(cString: cJson))
}
