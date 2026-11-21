//! Native Windows system tray entry point for Wireless Battery.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod autostart;
#[cfg(windows)]
mod icon;
#[cfg(windows)]
mod popover;
#[cfg(windows)]
mod renderer;
#[cfg(windows)]
mod theme;
#[cfg(windows)]
mod tray;

#[cfg(not(windows))]
fn main() {
    eprintln!("wireless-battery-tray Windows shell is only supported on Windows.");
}

#[cfg(windows)]
fn main() {
    if let Err(error) = win_app::run() {
        eprintln!("wireless-battery-tray error: {error}");
    }
}

#[cfg(windows)]
mod win_app {
    use crate::autostart::{copy_text_to_clipboard, set_autostart};
    use crate::icon::load_app_icon;
    use crate::popover::{PopoverAction, PopoverWindow};
    use crate::theme::SystemTheme;
    use crate::tray::{
        TrayIconHost, MENU_CMD_OPEN_PANEL, MENU_CMD_QUIT, MENU_CMD_TOGGLE_AUTOSTART,
        MENU_CMD_TOGGLE_LOW_BATTERY_NOTIFY,
        WM_TRAY_CALLBACK,
    };
    use std::sync::{mpsc, Arc, Mutex};
    use windows::core::w;
    use windows::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
    };
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::Shell::NIN_SELECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
        PostMessageW, PostQuitMessage, RegisterClassExW, RegisterWindowMessageW, SetWindowLongPtrW,
        TranslateMessage, GWLP_USERDATA, MSG, WINDOW_EX_STYLE, WM_APP, WM_CONTEXTMENU,
        WM_SETTINGCHANGE, WNDCLASSEXW, WS_OVERLAPPED,
    };

    const NIN_KEYSELECT: u32 = NIN_SELECT + 1;
    use wireless_battery_tray_core::{PollerHandle, TrayService, TrayUpdate};

    const WM_CORE_UPDATE: u32 = WM_APP + 20;
    const WM_POPOVER_ACTION: u32 = WM_APP + 21;

    struct SingleInstanceGuard(HANDLE);

    impl SingleInstanceGuard {
        fn acquire() -> Option<Self> {
            unsafe {
                let handle =
                    CreateMutexW(None, false, w!("Local\\WirelessBatteryTraySingleInstance"))
                        .ok()?;
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    let _ = CloseHandle(handle);
                    return None;
                }
                Some(Self(handle))
            }
        }
    }

    impl Drop for SingleInstanceGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    struct HostState {
        taskbar_created_msg: u32,
        theme: SystemTheme,
        tray: TrayIconHost,
        popover: PopoverWindow,
        poller: PollerHandle,
        updates_rx: mpsc::Receiver<TrayUpdate>,
        actions_rx: mpsc::Receiver<PopoverAction>,
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let Some(_single_instance) = SingleInstanceGuard::acquire() else {
            return Ok(());
        };

        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }

        let result = run_loop();

        unsafe {
            CoUninitialize();
        }
        result
    }

    fn run_loop() -> Result<(), Box<dyn std::error::Error>> {
        let service = Arc::new(TrayService::new()?);
        let initial_snapshot = service.snapshot();
        let theme = SystemTheme::detect();

        // Sync autostart registry entry with persisted config on startup.
        let _ = set_autostart(initial_snapshot.config.autostart);

        let class_name = w!("WirelessBatteryTrayMessageHost");
        let hinstance = unsafe { GetModuleHandleW(None)? };
        let app_icon = load_app_icon(hinstance);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(host_wnd_proc),
            hInstance: hinstance.into(),
            hIcon: app_icon,
            hIconSm: app_icon,
            lpszClassName: class_name,
            ..Default::default()
        };
        unsafe {
            let _ = RegisterClassExW(&wc);
        }

        let host_hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("WirelessBatteryTrayHost"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                hinstance,
                None,
            )
        };
        if host_hwnd.0 == 0 {
            return Err(windows::core::Error::from_win32().into());
        }

        let taskbar_created_msg = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };

        let (updates_tx, updates_rx) = mpsc::channel::<TrayUpdate>();
        let (actions_tx, actions_rx) = mpsc::channel::<PopoverAction>();

        let host_hwnd_raw = host_hwnd.0;
        let action_tx_shared = Arc::new(Mutex::new(actions_tx));
        let popover = PopoverWindow::create(initial_snapshot, theme, {
            let tx = Arc::clone(&action_tx_shared);
            move |action| {
                if let Ok(guard) = tx.lock() {
                    let _ = guard.send(action);
                }
                unsafe {
                    let _ =
                        PostMessageW(HWND(host_hwnd_raw), WM_POPOVER_ACTION, WPARAM(0), LPARAM(0));
                }
            }
        })?;

        let mut tray = TrayIconHost::new(host_hwnd, theme.taskbar_dark);
        tray.install();

        let poller = PollerHandle::start(service, move |update| {
            let _ = updates_tx.send(update);
            unsafe {
                let _ = PostMessageW(HWND(host_hwnd_raw), WM_CORE_UPDATE, WPARAM(0), LPARAM(0));
            }
        });

        let mut host = Box::new(HostState {
            taskbar_created_msg,
            theme,
            tray,
            popover,
            poller,
            updates_rx,
            actions_rx,
        });

        unsafe {
            SetWindowLongPtrW(
                host_hwnd,
                GWLP_USERDATA,
                (&mut *host as *mut HostState) as isize,
            );
        }

        let mut msg = MSG::default();
        unsafe {
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            SetWindowLongPtrW(host_hwnd, GWLP_USERDATA, 0);
        }

        host.tray.remove();
        host.poller.shutdown();
        Ok(())
    }

    unsafe extern "system" fn host_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut HostState;
        if ptr.is_null() {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let host = &mut *ptr;

        if msg == host.taskbar_created_msg && host.taskbar_created_msg != 0 {
            let _ = host.tray.install();
            return LRESULT(0);
        }

        match msg {
            WM_CORE_UPDATE => {
                while let Ok(update) = host.updates_rx.try_recv() {
                    host.tray.update(
                        update.snapshot.icon_state,
                        host.theme.taskbar_dark,
                        &update.snapshot.tooltip,
                    );
                    for alert in &update.alerts {
                        host.tray.notify_low_battery(alert);
                    }
                    host.popover.set_snapshot(update.snapshot);
                }
                LRESULT(0)
            }
            WM_POPOVER_ACTION => {
                while let Ok(action) = host.actions_rx.try_recv() {
                    dispatch_popover_action(host, action);
                }
                LRESULT(0)
            }
            WM_SETTINGCHANGE => {
                let next_theme = SystemTheme::detect();
                if next_theme != host.theme {
                    host.theme = next_theme;
                    let snap = host.poller.service().snapshot();
                    host.tray
                        .update(snap.icon_state, host.theme.taskbar_dark, &snap.tooltip);
                    host.popover.set_theme(host.theme);
                }
                LRESULT(0)
            }
            WM_TRAY_CALLBACK => {
                // Under NOTIFYICON_VERSION_4, a single click fires both WM_LBUTTONUP and
                // NIN_SELECT. Only handle the canonical v4 notifications to avoid toggling twice.
                let event = (lparam.0 & 0xFFFF) as u32;
                match event {
                    NIN_SELECT | NIN_KEYSELECT => {
                        let anchor = host.tray.anchor_rect();
                        let was_visible = host.popover.is_visible();
                        host.popover.toggle(anchor);
                        if !was_visible && host.popover.is_visible() {
                            maybe_refresh_on_open(host);
                        }
                    }
                    WM_CONTEXTMENU => {
                        host.popover.hide();
                        let cfg = host.poller.service().config();
                        match host
                            .tray
                            .show_context_menu(cfg.autostart, cfg.low_battery_notify)
                        {
                            Some(MENU_CMD_OPEN_PANEL) => {
                                let anchor = host.tray.anchor_rect();
                                host.popover.show(anchor);
                                maybe_refresh_on_open(host);
                            }
                            Some(MENU_CMD_TOGGLE_LOW_BATTERY_NOTIFY) => {
                                dispatch_popover_action(host, PopoverAction::ToggleLowBatteryNotify);
                            }
                            Some(MENU_CMD_TOGGLE_AUTOSTART) => {
                                dispatch_popover_action(host, PopoverAction::ToggleAutostart);
                            }
                            Some(MENU_CMD_QUIT) => {
                                PostQuitMessage(0);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    /// Opportunistically refresh on open when any device previously timed out
    /// (e.g., wireless keyboard woke up after startup) or data is > 15s old.
    fn maybe_refresh_on_open(host: &mut HostState) {
        let snap = host.poller.service().snapshot();
        let needs_retry = snap.devices.iter().any(|d| {
            d.precision == wireless_battery::Precision::None
                || d.freshness == Some(wireless_battery::Freshness::Stale)
        });
        let aged = snap.last_sync_at.map_or(true, |ts| {
            chrono::Utc::now().signed_duration_since(ts).num_seconds() >= 15
        });
        if !snap.refreshing && (needs_retry || aged) {
            host.poller.request_refresh();
        }
    }

    fn dispatch_popover_action(host: &mut HostState, action: PopoverAction) {
        match action {
            PopoverAction::RefreshAll => {
                host.poller.request_refresh();
            }
            PopoverAction::RefreshDevice(id) => {
                host.poller.request_device_refresh(id);
            }
            PopoverAction::CopyDiagnostic(id) => match host.poller.service().diagnose_json(&id) {
                Ok(json) => {
                    if copy_text_to_clipboard(host.popover.hwnd(), &json) {
                        host.popover.show_feedback("已复制脱敏诊断 JSON");
                    } else {
                        host.popover.show_feedback("复制剪贴板失败");
                    }
                }
                Err(_) => {
                    host.popover.show_feedback("无法生成设备诊断");
                }
            },
            PopoverAction::SetPollInterval(secs) => {
                let mut cfg = host.poller.service().config();
                cfg.poll_interval_secs = secs;
                let snap = host.poller.update_config(cfg);
                host.popover.set_snapshot(snap);
            }
            PopoverAction::ToggleLowBatteryNotify => {
                let mut cfg = host.poller.service().config();
                cfg.low_battery_notify = !cfg.low_battery_notify;
                let snap = host.poller.update_config(cfg);
                host.tray
                    .update(snap.icon_state, host.theme.taskbar_dark, &snap.tooltip);
                host.popover.set_snapshot(snap);
            }
            PopoverAction::ToggleAutostart => {
                let mut cfg = host.poller.service().config();
                cfg.autostart = !cfg.autostart;
                let _ = set_autostart(cfg.autostart);
                let snap = host.poller.update_config(cfg);
                host.popover.set_snapshot(snap);
            }
            PopoverAction::Quit => unsafe {
                PostQuitMessage(0);
            },
        }
    }
}
