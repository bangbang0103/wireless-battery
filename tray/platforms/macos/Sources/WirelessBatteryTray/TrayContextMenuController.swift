import AppKit
import Combine
import UserNotifications

final class TrayContextMenuController: NSObject {
    private let store: TrayStore
    private let openPanel: () -> Void
    private var updateStatusObserver: AnyCancellable?
    private var shouldReportUpdateCheck = false

    init(store: TrayStore, openPanel: @escaping () -> Void) {
        self.store = store
        self.openPanel = openPanel
        super.init()
        updateStatusObserver = store.$updateStatus
            .receive(on: RunLoop.main)
            .sink { [weak self] status in
                self?.handleUpdateStatus(status)
            }
    }

    func show(for button: NSStatusBarButton, event: NSEvent) {
        let lang = store.language
        let config = store.snapshot.config
        let menu = NSMenu()

        addItem(to: menu, title: lang.openPanelLabel, action: #selector(openPanelFromMenu))
        addSubmenu(to: menu, title: lang.settingThemeTitle, items: [
            (lang.themeLabel("system"), #selector(setTheme(_:)), "system", config.themeMode == "system"),
            (lang.menuThemeLightLabel, #selector(setTheme(_:)), "light", config.themeMode == "light"),
            (lang.menuThemeDarkLabel, #selector(setTheme(_:)), "dark", config.themeMode == "dark"),
        ])
        addSubmenu(to: menu, title: lang.menuLanguageLabel, items: [
            (lang.languageLabel("system"), #selector(setLanguage(_:)), "system", config.language == "system"),
            (lang.languageLabel("zh_cn"), #selector(setLanguage(_:)), "zh_cn", config.language == "zh_cn"),
            (lang.languageLabel("en"), #selector(setLanguage(_:)), "en", config.language == "en"),
        ])

        menu.addItem(.separator())
        addItem(
            to: menu,
            title: lang.settingLowBatteryTitle,
            action: #selector(toggleLowBatteryNotify),
            checked: config.lowBatteryNotify
        )
        addItem(
            to: menu,
            title: lang.settingSilentTitle,
            action: #selector(toggleLowBatterySilent),
            checked: config.lowBatterySilent,
            enabled: config.lowBatteryNotify
        )
        addItem(
            to: menu,
            title: lang.menuAutostartLabel,
            action: #selector(toggleAutostart),
            checked: config.autostart,
            enabled: store.isBundledApp
        )
        addItem(
            to: menu,
            title: lang.menuCheckUpdatesLabel,
            action: #selector(checkForUpdates),
            enabled: store.updateStatus != .checking
        )
        menu.addItem(.separator())

        #if DEV_TOOLS
            let devItem = NSMenuItem(title: "Dev 调试", action: nil, keyEquivalent: "")
            let devMenu = NSMenu()
            addItem(to: devMenu, title: "打开 Dev 调试面板", action: #selector(openDevPanel))
            devMenu.addItem(.separator())
            addItem(to: devMenu, title: "切换低电量状态", action: #selector(toggleDevLowBattery))
            addItem(to: devMenu, title: "发送低电量通知 (10%)", action: #selector(sendDevLowBattery10))
            addItem(to: devMenu, title: "发送严重低电通知 (5%)", action: #selector(sendDevLowBattery5))
            addItem(to: devMenu, title: "填充多设备测试场景", action: #selector(addDevPresetSuite))
            devMenu.addItem(.separator())
            addItem(to: devMenu, title: "清空虚拟设备 / 重置", action: #selector(resetDevDevices))
            devItem.submenu = devMenu
            menu.addItem(devItem)
            menu.addItem(.separator())
        #endif

        addItem(to: menu, title: lang.quitLabel, action: #selector(quit))
        NSMenu.popUpContextMenu(menu, with: event, for: button)
    }

    private func addItem(
        to menu: NSMenu,
        title: String,
        action: Selector,
        checked: Bool = false,
        enabled: Bool = true,
        value: String? = nil
    ) {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
        item.target = self
        item.state = checked ? .on : .off
        item.isEnabled = enabled
        item.representedObject = value
        menu.addItem(item)
    }

    private func addSubmenu(
        to menu: NSMenu,
        title: String,
        items: [(title: String, action: Selector, value: String, checked: Bool)]
    ) {
        let submenuItem = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        let submenu = NSMenu()
        for item in items {
            addItem(to: submenu, title: item.title, action: item.action, checked: item.checked, value: item.value)
        }
        submenuItem.submenu = submenu
        menu.addItem(submenuItem)
    }

    private func updateConfig(_ change: (inout TrayConfigModel) -> Void) {
        var config = store.snapshot.config
        change(&config)
        store.updateConfig(config)
    }

    @objc private func openPanelFromMenu() {
        openPanel()
    }

    @objc private func setTheme(_ item: NSMenuItem) {
        guard let mode = item.representedObject as? String else { return }
        updateConfig { $0.themeMode = mode }
    }

    @objc private func setLanguage(_ item: NSMenuItem) {
        guard let mode = item.representedObject as? String else { return }
        updateConfig { $0.language = mode }
    }

    @objc private func toggleLowBatteryNotify() {
        updateConfig { $0.lowBatteryNotify.toggle() }
    }

    @objc private func toggleLowBatterySilent() {
        guard store.snapshot.config.lowBatteryNotify else { return }
        updateConfig { $0.lowBatterySilent.toggle() }
    }

    @objc private func toggleAutostart() {
        updateConfig { $0.autostart.toggle() }
    }

    @objc private func checkForUpdates() {
        shouldReportUpdateCheck = true
        store.checkForUpdates()
    }

    @objc private func quit() {
        NSApplication.shared.terminate(nil)
    }

    private func handleUpdateStatus(_ status: UpdateCheckStatus) {
        guard shouldReportUpdateCheck else { return }
        let lang = store.language
        switch status {
        case let .available(latestVersion, url):
            shouldReportUpdateCheck = false
            notifyUpdate(lang.notifyUpdateAvailable(latestVersion: latestVersion))
            NSWorkspace.shared.open(url)
        case .upToDate:
            shouldReportUpdateCheck = false
            notifyUpdate(lang.notifyUpdateUpToDate(version: store.snapshot.appVersion))
        case .failed:
            shouldReportUpdateCheck = false
            notifyUpdate(lang.notifyUpdateFailed)
        case .idle, .checking:
            break
        }
    }

    private func notifyUpdate(_ body: String) {
        guard store.isBundledApp else { return }
        let content = UNMutableNotificationContent()
        content.title = store.language.notifyUpdateTitle
        content.body = body
        let request = UNNotificationRequest(identifier: "wb-update-\(UUID().uuidString)", content: content, trigger: nil)
        UNUserNotificationCenter.current().add(request)
    }

    #if DEV_TOOLS
        @objc private func openDevPanel() {
            DevPanelController.shared.show(store: store)
        }

        @objc private func toggleDevLowBattery() {
            store.devToggleLowBattery()
        }

        @objc private func sendDevLowBattery10() {
            store.devSendLowBatteryAlert(percentage: 10)
        }

        @objc private func sendDevLowBattery5() {
            store.devSendLowBatteryAlert(percentage: 5)
        }

        @objc private func addDevPresetSuite() {
            store.devAddPresetSuite()
        }

        @objc private func resetDevDevices() {
            store.devReset()
        }
    #endif
}
