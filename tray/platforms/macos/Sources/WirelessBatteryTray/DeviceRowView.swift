import SwiftUI

struct DeviceRowView: View {
    let device: DeviceViewModel
    let lang: AppLanguage
    let expanded: Bool
    let compact: Bool
    let onToggle: () -> Void
    let onRefresh: () -> Void
    let onCopyError: () -> Void
    @State private var hovered = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                if compact {
                    Image(systemName: symbolName)
                        .font(.system(size: 15))
                        .frame(width: 30, height: 30)
                        .shadow(color: Color.black.opacity(0.35), radius: 1.5, x: 0, y: 1)
                } else {
                    ZStack {
                        RoundedRectangle(cornerRadius: 8)
                            .fill(Color.primary.opacity(0.06))
                            .frame(width: 34, height: 34)
                        Image(systemName: symbolName)
                            .font(.system(size: 14))
                    }
                }

                VStack(alignment: .leading, spacing: 2) {
                    Text(device.name)
                        .font(.system(size: 13, weight: .medium))
                        .lineLimit(1)
                    Text(lang.deviceSubtitle(for: device))
                        .font(.system(size: 11))
                        .foregroundStyle(subtitleColor)
                        .lineLimit(1)
                }
                .shadow(color: compact ? Color.black.opacity(0.35) : .clear, radius: 1.5, x: 0, y: 1)

                Spacer()

                readoutView
                    .frame(width: 68, alignment: .trailing)
                    .shadow(color: compact ? Color.black.opacity(0.35) : .clear, radius: 1.5, x: 0, y: 1)

                if !compact {
                    Image(systemName: expanded ? "chevron.up" : "chevron.down")
                        .font(.system(size: 9.5, weight: .semibold))
                        .foregroundStyle(
                            hovered || expanded
                                ? HierarchicalShapeStyle.secondary
                                : HierarchicalShapeStyle.tertiary
                        )
                        .frame(width: 14)
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 9)
            .background(
                RoundedRectangle(cornerRadius: 8)
                    .fill(!compact && hovered ? Color.primary.opacity(0.055) : Color.clear)
            )
            .padding(.horizontal, 6)
            .contentShape(Rectangle())
            .onHover { hovered = $0 }
            .onTapGesture(perform: onToggle)

            if expanded {
                VStack(spacing: 7) {
                    HStack {
                        Text(lang.labelProtocol).foregroundStyle(.secondary)
                        Spacer()
                        Text(device.providerId ?? "--")
                            .font(.system(size: 10.5, design: .monospaced))
                    }
                    HStack {
                        Text(lang.labelBus).foregroundStyle(.secondary)
                        Spacer()
                        Text(device.transportLabel)
                    }
                    HStack {
                        Text(lang.labelStatus).foregroundStyle(.secondary)
                        Spacer()
                        Text(expandedStatusLabel)
                            .foregroundStyle(expandedStatusColor)
                    }
                    HStack(spacing: 8) {
                        Spacer()
                        Button(lang.btnRefreshDevice, action: onRefresh)
                            .buttonStyle(.bordered)
                            .controlSize(.small)
                        if device.lastError != nil {
                            Button(lang.btnCopyError, action: onCopyError)
                                .buttonStyle(.bordered)
                                .controlSize(.small)
                        }
                    }
                    .padding(.top, 2)
                }
                .font(.system(size: 11))
                .padding(.horizontal, 12)
                .padding(.vertical, 10)
                .background(Color.primary.opacity(0.045), in: RoundedRectangle(cornerRadius: 8))
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .strokeBorder(Color.primary.opacity(0.07), lineWidth: 0.5)
                )
                .padding(.horizontal, 14)
                .padding(.bottom, 8)
                .transition(.opacity)
            }
        }
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
                        Capsule().fill(Color.primary.opacity(0.12))
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
                Text(lang.discreteLabel(device.level))
                    .font(.system(size: 12, weight: .semibold))
                HStack(spacing: 3) {
                    ForEach(0..<4, id: \.self) { i in
                        Capsule()
                            .fill(i < segs ? barColor : Color.primary.opacity(0.12))
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

    private var expandedStatusLabel: String {
        if let err = device.lastError {
            return lang.errorShortLabel(err)
        }
        if device.freshness == "stale" { return lang.statusStale }
        if device.freshness == "live" { return lang.statusLive }
        return lang.statusUnread
    }

    private var expandedStatusColor: Color {
        if device.lastError != nil { return .orange }
        if device.freshness == "live" { return .green }
        return .secondary
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
}
