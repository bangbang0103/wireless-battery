//! Bilingual (Simplified Chinese & English) localization for the Windows native tray shell.

use chrono::{DateTime, Utc};
use windows::Win32::Globalization::GetUserDefaultUILanguage;
use wireless_battery::{BatteryLevel, ChargeState, ErrorKind, Freshness, Precision, Reachability};
use wireless_battery_tray_core::{
    format_footer_summary, format_poll_label, format_relative_time, DeviceView, LanguageMode,
    ThemeMode,
};

/// Resolved active display language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    ZhCn,
    En,
}

impl Lang {
    /// Detect Windows system UI language (`LANG_CHINESE == 0x04`).
    pub fn detect_system() -> Self {
        let lang_id = unsafe { GetUserDefaultUILanguage() };
        let primary_lang = lang_id & 0x03FF;
        if primary_lang == 0x04 {
            Self::ZhCn
        } else {
            Self::En
        }
    }

    /// Resolve the active display language from user `LanguageMode`.
    pub fn resolve(mode: LanguageMode) -> Self {
        match mode {
            LanguageMode::System => Self::detect_system(),
            LanguageMode::ZhCn => Self::ZhCn,
            LanguageMode::En => Self::En,
        }
    }

    pub const fn is_zh(self) -> bool {
        matches!(self, Self::ZhCn)
    }

    // --- Main view ---

    pub const fn settings_title(self) -> &'static str {
        match self {
            Self::ZhCn => "设置",
            Self::En => "Settings",
        }
    }

    pub const fn quit(self) -> &'static str {
        match self {
            Self::ZhCn => "退出",
            Self::En => "Quit",
        }
    }

    pub const fn reading_peripherals(self) -> &'static str {
        match self {
            Self::ZhCn => "正在读取外设电量…",
            Self::En => "Reading peripherals…",
        }
    }

    pub const fn empty_title(self) -> &'static str {
        match self {
            Self::ZhCn => "未发现可读取的无线外设",
            Self::En => "No readable wireless peripherals",
        }
    }

    pub fn empty_hint(self) -> String {
        match self {
            Self::ZhCn => {
                "被动枚举未匹配到已知接收器或外设，请确认接收器已接入并处于唤醒状态。".to_owned()
            }
            Self::En => {
                "No known receiver or peripheral matched. Make sure the receiver is plugged in and awake."
                    .to_owned()
            }
        }
    }

    pub const fn empty_action(self) -> &'static str {
        match self {
            Self::ZhCn => "重新扫描",
            Self::En => "Rescan",
        }
    }

    // --- Expanded device drawer (no "Evidence" field) ---

    pub const fn label_protocol(self) -> &'static str {
        match self {
            Self::ZhCn => "协议",
            Self::En => "Protocol",
        }
    }

    pub const fn label_bus(self) -> &'static str {
        match self {
            Self::ZhCn => "总线",
            Self::En => "Bus",
        }
    }

    pub const fn label_status(self) -> &'static str {
        match self {
            Self::ZhCn => "状态",
            Self::En => "Status",
        }
    }

    pub const fn status_live(self) -> &'static str {
        match self {
            Self::ZhCn => "实时",
            Self::En => "Live",
        }
    }

    pub const fn status_stale(self) -> &'static str {
        match self {
            Self::ZhCn => "缓存（Stale）",
            Self::En => "Stale (cached)",
        }
    }

    pub const fn status_unread(self) -> &'static str {
        match self {
            Self::ZhCn => "未读取",
            Self::En => "Not read",
        }
    }

    pub const fn btn_refresh_device(self) -> &'static str {
        match self {
            Self::ZhCn => "重新读取",
            Self::En => "Refresh",
        }
    }

    pub const fn btn_copy_error(self) -> &'static str {
        match self {
            Self::ZhCn => "复制报错",
            Self::En => "Copy error",
        }
    }

    // --- Settings cards & options ---

    pub const fn setting_theme_title(self) -> &'static str {
        match self {
            Self::ZhCn => "外观主题",
            Self::En => "Appearance",
        }
    }

    pub const fn setting_theme_desc(self, mode: ThemeMode, system_dark: bool) -> &'static str {
        match (self, mode, system_dark) {
            (Self::ZhCn, ThemeMode::System, true) => "当前跟随系统（暗黑）",
            (Self::ZhCn, ThemeMode::System, false) => "当前跟随系统（明亮）",
            (Self::ZhCn, ThemeMode::Light, _) => "始终使用明亮外观",
            (Self::ZhCn, ThemeMode::Dark, _) => "始终使用暗黑外观",
            (Self::En, ThemeMode::System, true) => "Following system (Dark)",
            (Self::En, ThemeMode::System, false) => "Following system (Light)",
            (Self::En, ThemeMode::Light, _) => "Always use light theme",
            (Self::En, ThemeMode::Dark, _) => "Always use dark theme",
        }
    }

    pub const fn theme_mode_label(self, mode: ThemeMode) -> &'static str {
        match (self, mode) {
            (Self::ZhCn, ThemeMode::System) => "跟随系统",
            (Self::ZhCn, ThemeMode::Light) => "明亮",
            (Self::ZhCn, ThemeMode::Dark) => "暗黑",
            (Self::En, ThemeMode::System) => "System",
            (Self::En, ThemeMode::Light) => "Light",
            (Self::En, ThemeMode::Dark) => "Dark",
        }
    }

    pub const fn setting_language_title(self) -> &'static str {
        match self {
            Self::ZhCn => "语言",
            Self::En => "Language",
        }
    }

    pub const fn setting_language_desc(self, mode: LanguageMode) -> &'static str {
        match (self, mode) {
            (Self::ZhCn, LanguageMode::System) => "跟随系统显示语言",
            (Self::ZhCn, LanguageMode::ZhCn) => "界面使用简体中文",
            (Self::ZhCn, LanguageMode::En) => "界面使用 English",
            (Self::En, LanguageMode::System) => "Follow system UI language",
            (Self::En, LanguageMode::ZhCn) => "Display in Simplified Chinese",
            (Self::En, LanguageMode::En) => "Display in English",
        }
    }

    pub const fn language_mode_label(self, mode: LanguageMode) -> &'static str {
        match (self, mode) {
            (Self::ZhCn, LanguageMode::System) => "跟随系统",
            (Self::En, LanguageMode::System) => "System",
            (_, LanguageMode::ZhCn) => "简体中文",
            (_, LanguageMode::En) => "English",
        }
    }

    pub const fn setting_poll_title(self) -> &'static str {
        match self {
            Self::ZhCn => "轮询频率",
            Self::En => "Poll interval",
        }
    }

    pub const fn setting_poll_desc(self) -> &'static str {
        match self {
            Self::ZhCn => "休眠设备不主动唤醒",
            Self::En => "Sleeping devices are not woken",
        }
    }

    pub fn poll_interval_label(self, secs: u64) -> String {
        format_poll_label(secs, self.is_zh())
    }

    pub const fn setting_low_battery_title(self) -> &'static str {
        match self {
            Self::ZhCn => "低电量警告",
            Self::En => "Low battery alert",
        }
    }

    pub const fn setting_low_battery_desc(self, enabled: bool, silent: bool) -> &'static str {
        match (self, enabled, silent) {
            (Self::ZhCn, false, _) => "已关闭低电量提醒与角标",
            (Self::ZhCn, true, true) => "已静默，仅显示托盘低电角标",
            (Self::ZhCn, true, false) => "15% / 10% / 5% / 1% 时发送系统通知",
            (Self::En, false, _) => "Badge and alerts disabled",
            (Self::En, true, true) => "Silent: tray badge only",
            (Self::En, true, false) => "Notify at 15%, 10%, 5%, 1%",
        }
    }

    pub const fn setting_silent_title(self) -> &'static str {
        match self {
            Self::ZhCn => "静默通知",
            Self::En => "Silent notifications",
        }
    }

    pub const fn setting_silent_desc(self) -> &'static str {
        match self {
            Self::ZhCn => "开启后不发送系统通知",
            Self::En => "Suppress OS toast notifications",
        }
    }

    pub const fn setting_autostart_title(self) -> &'static str {
        match self {
            Self::ZhCn => "开机静默自启",
            Self::En => "Launch at startup",
        }
    }

    pub const fn setting_autostart_desc(self) -> &'static str {
        match self {
            Self::ZhCn => "仅驻留系统托盘，不弹出窗口",
            Self::En => "Start minimized in system tray",
        }
    }

    pub const fn setting_version_title(self) -> &'static str {
        match self {
            Self::ZhCn => "版本与更新",
            Self::En => "Version & updates",
        }
    }

    pub fn setting_version_desc(self, version: &str) -> String {
        match self {
            Self::ZhCn => format!("当前版本 {version}"),
            Self::En => format!("Current version {version}"),
        }
    }

    pub const fn setting_version_checking(self) -> &'static str {
        match self {
            Self::ZhCn => "正在检查更新…",
            Self::En => "Checking for updates…",
        }
    }

    pub fn setting_version_up_to_date(self, version: &str) -> String {
        match self {
            Self::ZhCn => format!("{version} · 已是最新版本"),
            Self::En => format!("{version} · Up to date"),
        }
    }

    pub fn setting_version_available(self, latest: &str) -> String {
        match self {
            Self::ZhCn => format!("发现新版本 {latest}"),
            Self::En => format!("Update available: {latest}"),
        }
    }

    pub const fn setting_version_failed(self) -> &'static str {
        match self {
            Self::ZhCn => "检查更新失败，请稍后重试",
            Self::En => "Update check failed, try again",
        }
    }

    pub const fn btn_check_updates(self) -> &'static str {
        match self {
            Self::ZhCn => "检查更新",
            Self::En => "Check",
        }
    }

    pub const fn btn_checking_updates(self) -> &'static str {
        match self {
            Self::ZhCn => "检查中…",
            Self::En => "Checking…",
        }
    }

    pub const fn btn_download_update(self) -> &'static str {
        match self {
            Self::ZhCn => "前往下载",
            Self::En => "Download",
        }
    }

    // --- Feedback toasts ---

    pub const fn toast_pinned(self) -> &'static str {
        match self {
            Self::ZhCn => "已置顶（移出鼠标隐藏背景）",
            Self::En => "Pinned (transparent when unfocused)",
        }
    }

    pub const fn toast_unpinned(self) -> &'static str {
        match self {
            Self::ZhCn => "已取消置顶",
            Self::En => "Unpinned",
        }
    }

    pub const fn toast_error_copied(self) -> &'static str {
        match self {
            Self::ZhCn => "已复制报错信息",
            Self::En => "Error details copied",
        }
    }

    pub const fn toast_copy_failed(self) -> &'static str {
        match self {
            Self::ZhCn => "复制剪贴板失败",
            Self::En => "Failed to copy to clipboard",
        }
    }

    pub const fn toast_diag_failed(self) -> &'static str {
        match self {
            Self::ZhCn => "无法生成报错信息",
            Self::En => "Unable to generate error report",
        }
    }

    // --- Tray context menu ---

    pub const fn menu_open_panel(self) -> &'static str {
        match self {
            Self::ZhCn => "打开面板",
            Self::En => "Open Panel",
        }
    }

    pub const fn menu_theme(self) -> &'static str {
        match self {
            Self::ZhCn => "外观主题",
            Self::En => "Theme",
        }
    }

    pub const fn menu_theme_light(self) -> &'static str {
        match self {
            Self::ZhCn => "明亮模式",
            Self::En => "Light",
        }
    }

    pub const fn menu_theme_dark(self) -> &'static str {
        match self {
            Self::ZhCn => "暗黑模式",
            Self::En => "Dark",
        }
    }

    pub const fn menu_language(self) -> &'static str {
        match self {
            Self::ZhCn => "语言",
            Self::En => "Language",
        }
    }

    pub const fn menu_low_battery(self) -> &'static str {
        match self {
            Self::ZhCn => "低电量警告",
            Self::En => "Low battery alert",
        }
    }

    pub const fn menu_silent_notify(self) -> &'static str {
        match self {
            Self::ZhCn => "静默通知",
            Self::En => "Silent notifications",
        }
    }

    pub const fn menu_autostart(self) -> &'static str {
        match self {
            Self::ZhCn => "开机自启",
            Self::En => "Launch at startup",
        }
    }

    pub const fn menu_check_updates(self) -> &'static str {
        match self {
            Self::ZhCn => "检查更新",
            Self::En => "Check for updates",
        }
    }

    pub const fn notify_update_title(self) -> &'static str {
        match self {
            Self::ZhCn => "检查更新",
            Self::En => "Update Check",
        }
    }

    pub fn notify_update_available(self, latest: &str) -> String {
        match self {
            Self::ZhCn => format!("发现新版本 {latest}，正在打开下载页面。"),
            Self::En => format!("New version {latest} is available. Opening download page."),
        }
    }

    pub fn notify_update_up_to_date(self, version: &str) -> String {
        match self {
            Self::ZhCn => format!("当前已是最新版本（{version}）。"),
            Self::En => format!("You are up to date ({version})."),
        }
    }

    pub const fn notify_update_failed(self) -> &'static str {
        match self {
            Self::ZhCn => "检查更新失败，请检查网络后重试。",
            Self::En => "Failed to check for updates. Please try again later.",
        }
    }

    // --- OS System Notification ---

    pub const fn notify_title(self, percentage: u8) -> &'static str {
        if percentage <= 5 {
            match self {
                Self::ZhCn => "外设极低电量警报",
                Self::En => "Critical Peripheral Battery",
            }
        } else {
            match self {
                Self::ZhCn => "外设低电量提醒",
                Self::En => "Low Peripheral Battery",
            }
        }
    }

    pub fn notify_body(self, device_name: &str, percentage: u8) -> String {
        if percentage <= 1 {
            match self {
                Self::ZhCn => {
                    format!("{device_name} 电量仅剩 {percentage}%，即将耗尽，请立即充电。")
                }
                Self::En => format!(
                    "{device_name} is at {percentage}% battery and about to shut off. Charge now."
                ),
            }
        } else if percentage <= 5 {
            match self {
                Self::ZhCn => format!("{device_name} 电量仅剩 {percentage}%，请尽快连接充电。"),
                Self::En => {
                    format!("{device_name} is critically low ({percentage}%). Please charge soon.")
                }
            }
        } else {
            match self {
                Self::ZhCn => format!("{device_name} 电量仅剩 {percentage}%，建议连接充电。"),
                Self::En => {
                    format!("{device_name} battery is down to {percentage}%. Consider charging.")
                }
            }
        }
    }

    // --- Dynamic formatters ---

    pub fn footer_summary(
        self,
        app_version: &str,
        last_sync_at: Option<DateTime<Utc>>,
        poll_secs: u64,
    ) -> String {
        format_footer_summary(app_version, last_sync_at, poll_secs, self.is_zh())
    }

    pub fn format_relative_time(self, at: Option<DateTime<Utc>>) -> String {
        format_relative_time(at, self.is_zh())
    }

    pub const fn discrete_info(self, level: Option<BatteryLevel>) -> (u8, &'static str) {
        match (self, level) {
            (Self::ZhCn, Some(BatteryLevel::Critical)) => (1, "极低"),
            (Self::ZhCn, Some(BatteryLevel::Low)) => (1, "低"),
            (Self::ZhCn, Some(BatteryLevel::Medium)) => (2, "中"),
            (Self::ZhCn, Some(BatteryLevel::High)) => (3, "高"),
            (Self::ZhCn, Some(BatteryLevel::Full)) => (4, "满"),
            (Self::ZhCn, None) => (0, "未知"),
            (Self::En, Some(BatteryLevel::Critical)) => (1, "Critical"),
            (Self::En, Some(BatteryLevel::Low)) => (1, "Low"),
            (Self::En, Some(BatteryLevel::Medium)) => (2, "Medium"),
            (Self::En, Some(BatteryLevel::High)) => (3, "High"),
            (Self::En, Some(BatteryLevel::Full)) => (4, "Full"),
            (Self::En, None) => (0, "Unknown"),
        }
    }

    pub const fn error_kind_short_label(self, kind: ErrorKind) -> &'static str {
        match (self, kind) {
            (Self::ZhCn, ErrorKind::Timeout) => "读取超时",
            (Self::ZhCn, ErrorKind::DeviceBusy) => "接口正忙",
            (Self::ZhCn, ErrorKind::PermissionDenied) => "权限不足",
            (Self::ZhCn, ErrorKind::DeviceDisconnected) => "连接已断开",
            (Self::ZhCn, ErrorKind::InvalidResponse | ErrorKind::ChecksumFailed) => "响应校验失败",
            (Self::ZhCn, ErrorKind::Unsupported) => "策略未启用",
            (Self::ZhCn, _) => "读取失败",
            (Self::En, ErrorKind::Timeout) => "Timeout",
            (Self::En, ErrorKind::DeviceBusy) => "Device busy",
            (Self::En, ErrorKind::PermissionDenied) => "Permission denied",
            (Self::En, ErrorKind::DeviceDisconnected) => "Disconnected",
            (Self::En, ErrorKind::InvalidResponse | ErrorKind::ChecksumFailed) => {
                "Invalid response"
            }
            (Self::En, ErrorKind::Unsupported) => "Unsupported",
            (Self::En, _) => "Read failed",
        }
    }

    pub fn format_device_subtitle(self, dev: &DeviceView) -> String {
        let mut parts: Vec<String> = Vec::new();
        if dev.charge_state == ChargeState::Charging {
            parts.push(match self {
                Self::ZhCn => "充电中".to_owned(),
                Self::En => "Charging".to_owned(),
            });
        } else if dev.reachability == Reachability::Sleeping {
            parts.push(match self {
                Self::ZhCn => "休眠".to_owned(),
                Self::En => "Sleeping".to_owned(),
            });
        } else if dev.reachability == Reachability::Offline {
            parts.push(match self {
                Self::ZhCn => "离线".to_owned(),
                Self::En => "Offline".to_owned(),
            });
        } else if dev.precision == Precision::None {
            match (self, dev.last_error) {
                (Self::ZhCn, Some(ErrorKind::Timeout)) => {
                    parts.push("读取超时 · 请唤醒后重试".to_owned())
                }
                (Self::ZhCn, Some(ErrorKind::DeviceBusy)) => {
                    parts.push("接口正忙 · 稍后重试".to_owned())
                }
                (Self::ZhCn, Some(ErrorKind::PermissionDenied)) => {
                    parts.push("权限不足".to_owned())
                }
                (Self::ZhCn, Some(ErrorKind::DeviceDisconnected)) => {
                    parts.push("连接已断开".to_owned())
                }
                (Self::ZhCn, Some(_)) => parts.push("读取失败 · 点此重读".to_owned()),
                (Self::ZhCn, None) => parts.push(format!("{} · 读取中", dev.transport_label)),
                (Self::En, Some(ErrorKind::Timeout)) => {
                    parts.push("Timeout · Wake & retry".to_owned())
                }
                (Self::En, Some(ErrorKind::DeviceBusy)) => {
                    parts.push("Busy · Retry shortly".to_owned())
                }
                (Self::En, Some(ErrorKind::PermissionDenied)) => {
                    parts.push("Permission denied".to_owned())
                }
                (Self::En, Some(ErrorKind::DeviceDisconnected)) => {
                    parts.push("Disconnected".to_owned())
                }
                (Self::En, Some(_)) => parts.push("Read failed · Click to retry".to_owned()),
                (Self::En, None) => parts.push(format!("{} · Reading", dev.transport_label)),
            }
        } else if dev.precision == Precision::DiscreteLevel {
            parts.push(match self {
                Self::ZhCn => "离散档位".to_owned(),
                Self::En => "Discrete level".to_owned(),
            });
        } else {
            parts.push(dev.transport_label.clone());
        }

        if dev.freshness == Some(Freshness::Stale) {
            let rel = self.format_relative_time(dev.observed_at);
            parts.push(match self {
                Self::ZhCn => format!("缓存 · {rel}"),
                Self::En => format!("Cached · {rel}"),
            });
        } else if let Some(mv) = dev.voltage_mv {
            if dev.precision == Precision::Percentage {
                parts.push(format!("{:.2} V", f64::from(mv) / 1000.0));
            }
        }

        parts.join(" · ")
    }

    // --- Community & support links ---

    pub const fn setting_source_title(self) -> &'static str {
        match self {
            Self::ZhCn => "源码",
            Self::En => "Source Code",
        }
    }

    pub const fn setting_source_desc(self) -> &'static str {
        "bangbang0103/wireless-battery"
    }

    pub const fn setting_feedback_title(self) -> &'static str {
        match self {
            Self::ZhCn => "反馈",
            Self::En => "Feedback",
        }
    }

    pub const fn setting_feedback_desc(self) -> &'static str {
        "GitHub Issues"
    }

    pub const fn setting_community_title(self) -> &'static str {
        match self {
            Self::ZhCn => "社区交流",
            Self::En => "Community",
        }
    }

    pub const fn setting_community_desc(self) -> &'static str {
        "QQ · Discord"
    }

    pub const fn submit_feedback_label(self) -> &'static str {
        match self {
            Self::ZhCn => "反馈",
            Self::En => "Submit Feedback",
        }
    }

    pub const fn open_repository_label(self) -> &'static str {
        match self {
            Self::ZhCn => "查看",
            Self::En => "Open Repository",
        }
    }

    pub fn toast_qq_copied(self, group: &str) -> String {
        match self {
            Self::ZhCn => format!("已复制 QQ 群号 {group}"),
            Self::En => format!("Copied QQ group {group}"),
        }
    }

    pub const fn toast_qq_coming_soon(self) -> &'static str {
        match self {
            Self::ZhCn => "QQ 群号待补充",
            Self::En => "QQ group number coming soon",
        }
    }

    pub const fn toast_discord_copied(self) -> &'static str {
        match self {
            Self::ZhCn => "已复制 Discord 邀请链接",
            Self::En => "Discord invite link copied",
        }
    }

    pub const fn toast_discord_coming_soon(self) -> &'static str {
        match self {
            Self::ZhCn => "Discord 邀请链接待补充",
            Self::En => "Discord invite link coming soon",
        }
    }
}
