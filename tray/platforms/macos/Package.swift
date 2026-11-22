// swift-tools-version: 5.9
import Foundation
import PackageDescription

let packageDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
let isDevTools = ProcessInfo.processInfo.environment["WB_DEV_TOOLS"] == "1"
let rustDebugLibDir = packageDir
    .appendingPathComponent("../../target/debug")
    .standardizedFileURL
    .path
let rustReleaseLibDir = packageDir
    .appendingPathComponent("../../target/release")
    .standardizedFileURL
    .path
let rustLibDir: String = {
    if isDevTools && FileManager.default.fileExists(atPath: rustDebugLibDir + "/libwireless_battery_tray_ffi.a") {
        return rustDebugLibDir
    }
    return rustReleaseLibDir
}()

let package = Package(
    name: "WirelessBatteryTray",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "WirelessBatteryTray", targets: ["WirelessBatteryTray"]),
    ],
    targets: [
        .systemLibrary(
            name: "CTrayCore",
            path: "Sources/CTrayCore"
        ),
        .executableTarget(
            name: "WirelessBatteryTray",
            dependencies: ["CTrayCore"],
            path: "Sources/WirelessBatteryTray",
            swiftSettings: isDevTools ? [.define("DEV_TOOLS")] : [],
            linkerSettings: [
                .unsafeFlags(["-L", rustLibDir, "-lwireless_battery_tray_ffi"]),
                .linkedFramework("CoreBluetooth"),
                .linkedFramework("IOKit"),
                .linkedFramework("CoreFoundation"),
                .linkedFramework("AppKit"),
                .linkedFramework("Security"),
                .linkedFramework("ServiceManagement"),
                .linkedFramework("UserNotifications"),
            ]
        ),
    ]
)
