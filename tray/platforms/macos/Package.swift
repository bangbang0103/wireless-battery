// swift-tools-version: 5.9
import Foundation
import PackageDescription

let packageDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
let rustReleaseLibDir = packageDir
    .appendingPathComponent("../../target/release")
    .standardizedFileURL
    .path

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
            linkerSettings: [
                .unsafeFlags(["-L", rustReleaseLibDir, "-lwireless_battery_tray_ffi"]),
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
