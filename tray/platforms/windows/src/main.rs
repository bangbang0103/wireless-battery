//! Native Windows system tray entry point for Wireless Battery.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod autostart;
#[cfg(all(windows, feature = "dev-tools"))]
mod dev_tools;
#[cfg(windows)]
mod i18n;
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
#[cfg(windows)]
mod updater;

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
    use crate::i18n::Lang;
    use crate::icon::load_app_icon;
    use crate::popover::{PopoverAction, PopoverWindow};
    use crate::renderer::UpdateUiStatus;
    use crate::theme::SystemTheme;
    use crate::tray::{
        TrayIconHost, MENU_CMD_CHECK_UPDATES, MENU_CMD_LANG_EN, MENU_CMD_LANG_SYSTEM,
        MENU_CMD_LANG_ZH_CN, MENU_CMD_OPEN_PANEL, MENU_CMD_QUIT, MENU_CMD_THEME_DARK,
        MENU_CMD_THEME_LIGHT, MENU_CMD_THEME_SYSTEM, MENU_CMD_TOGGLE_AUTOSTART,
        MENU_CMD_TOGGLE_LOW_BATTERY_NOTIFY, MENU_CMD_TOGGLE_LOW_BATTERY_SILENT, WM_TRAY_CALLBACK,
    };
    use crate::updater;
    use std::cell::{Cell, RefCell};
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
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        GetWindowLongPtrW, KillTimer, PostMessageW, PostQuitMessage, RegisterClassExW,
        RegisterWindowMessageW, SetTimer, SetWindowLongPtrW, TranslateMessage, GWLP_USERDATA, MSG,
        WINDOW_EX_STYLE, WM_APP, WM_CONTEXTMENU, WM_DEVICECHANGE, WM_SETTINGCHANGE, WM_TIMER,
        WNDCLASSEXW, WS_OVERLAPPED,
    };

    const NIN_KEYSELECT: u32 = NIN_SELECT + 1;
    const WM_DWMCOLORIZATIONCOLORCHANGED: u32 = 0x0320;
    use wireless_battery_tray_core::{
        LanguageMode, PollerHandle, ThemeMode, TrayService, TrayUpdate, UpdateCheckResult,
    };

    const WM_CORE_UPDATE: u32 = WM_APP + 20;
    const WM_POPOVER_ACTION: u32 = WM_APP + 21;
    const WM_UPDATE_CHECK_RESULT: u32 = WM_APP + 22;
    const DBT_DEVNODES_CHANGED: usize = 0x0007;
    const TIMER_DEVICE_CHANGE: usize = 1;

    struct SingleInstanceGuard(HANDLE);

    impl SingleInstanceGuard {
        fn acquire() -> Option<Self> {
            #[cfg(feature = "dev-tools")]
            let mutex_name = w!("Local\\WirelessBatteryTrayDevSingleInstance");
            #[cfg(not(feature = "dev-tools"))]
            let mutex_name = w!("Local\\WirelessBatteryTraySingleInstance");
            unsafe {
                let handle = CreateMutexW(None, false, mutex_name).ok()?;
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
        host_hwnd: HWND,
        taskbar_created_msg: u32,
        theme: Cell<SystemTheme>,
        tray: RefCell<TrayIconHost>,
        popover: PopoverWindow,
        poller: PollerHandle,
        updates_rx: mpsc::Receiver<TrayUpdate>,
        actions_rx: mpsc::Receiver<PopoverAction>,
        update_check_tx: mpsc::Sender<(UpdateCheckResult, bool)>,
        update_check_rx: mpsc::Receiver<(UpdateCheckResult, bool)>,
        #[cfg(feature = "dev-tools")]
        dev_panel: crate::dev_tools::DevPanelWindow,
        #[cfg(feature = "dev-tools")]
        dev_actions_rx: mpsc::Receiver<crate::dev_tools::DevAction>,
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
        service.set_system_is_zh(Lang::detect_system() == Lang::ZhCn);
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
        let (update_check_tx, update_check_rx) = mpsc::channel::<(UpdateCheckResult, bool)>();

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
        })?;

        #[cfg(feature = "dev-tools")]
        let (dev_panel, dev_actions_rx) = {
            let (dev_tx, dev_rx) = mpsc::channel::<crate::dev_tools::DevAction>();
            let panel =
                crate::dev_tools::DevPanelWindow::create(host_hwnd, Arc::new(Mutex::new(dev_tx)))?;
            (panel, dev_rx)
        };

        let host = Box::new(HostState {
            host_hwnd,
            taskbar_created_msg,
            theme: Cell::new(theme),
            tray: RefCell::new(tray),
            popover,
            poller,
            updates_rx,
            actions_rx,
            update_check_tx,
            update_check_rx,
            #[cfg(feature = "dev-tools")]
            dev_panel,
            #[cfg(feature = "dev-tools")]
            dev_actions_rx,
        });

        unsafe {
            SetWindowLongPtrW(
                host_hwnd,
                GWLP_USERDATA,
                (&*host as *const HostState) as isize,
            );
        }

        let mut msg = MSG::default();
        unsafe {
            while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            SetWindowLongPtrW(host_hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(host_hwnd);
        }

        if let Ok(mut tray) = host.tray.try_borrow_mut() {
            tray.remove();
        }
        drop(host);
        Ok(())
    }

    unsafe extern "system" fn host_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const HostState;
        if ptr.is_null() {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let host = &*ptr;

        if msg == host.taskbar_created_msg && host.taskbar_created_msg != 0 {
            if let Ok(mut tray) = host.tray.try_borrow_mut() {
                let _ = tray.install();
            }
            return LRESULT(0);
        }

        match msg {
            WM_CORE_UPDATE => {
                while let Ok(update) = host.updates_rx.try_recv() {
                    let lang = Lang::resolve(update.snapshot.config.language);
                    let dark = host.theme.get().taskbar_dark;
                    if let Ok(mut tray) = host.tray.try_borrow_mut() {
                        tray.update(update.snapshot.icon_state, dark, &update.snapshot.tooltip);
                        if update.snapshot.config.low_battery_notify
                            && !update.snapshot.config.low_battery_silent
                        {
                            for alert in &update.alerts {
                                tray.notify_low_battery(alert, lang);
                            }
                        }
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
            WM_UPDATE_CHECK_RESULT => {
                while let Ok((result, from_menu)) = host.update_check_rx.try_recv() {
                    let lang = Lang::resolve(host.poller.service().config().language);
                    match result {
                        UpdateCheckResult::Available {
                            latest_version,
                            url,
                        } => {
                            let body = lang.notify_update_available(&latest_version);
                            host.popover.set_update_status(UpdateUiStatus::Available {
                                latest_version,
                                url: url.clone(),
                            });
                            if from_menu {
                                if let Ok(tray) = host.tray.try_borrow() {
                                    tray.notify_info(lang.notify_update_title(), &body);
                                }
                                updater::open_url(&url);
                            } else {
                                host.popover.show_feedback(body);
                            }
                        }
                        UpdateCheckResult::UpToDate { current_version } => {
                            let body = lang.notify_update_up_to_date(&current_version);
                            host.popover.set_update_status(UpdateUiStatus::UpToDate);
                            if from_menu {
                                if let Ok(tray) = host.tray.try_borrow() {
                                    tray.notify_info(lang.notify_update_title(), &body);
                                }
                            } else {
                                host.popover.show_feedback(body);
                            }
                        }
                        UpdateCheckResult::Failed(_) => {
                            host.popover.set_update_status(UpdateUiStatus::Failed);
                            if from_menu {
                                if let Ok(tray) = host.tray.try_borrow() {
                                    tray.notify_info(
                                        lang.notify_update_title(),
                                        lang.notify_update_failed(),
                                    );
                                }
                            } else {
                                host.popover.show_feedback(lang.notify_update_failed());
                            }
                        }
                    }
                }
                LRESULT(0)
            }
            #[cfg(feature = "dev-tools")]
            crate::dev_tools::WM_DEV_ACTION => {
                while let Ok(action) = host.dev_actions_rx.try_recv() {
                    if let Ok(mut tray) = host.tray.try_borrow_mut() {
                        crate::dev_tools::dispatch_dev_action(
                            action,
                            &host.dev_panel,
                            &host.poller,
                            &mut tray,
                            &host.popover,
                            host.theme.get(),
                        );
                    }
                }
                LRESULT(0)
            }
            WM_SETTINGCHANGE | WM_DWMCOLORIZATIONCOLORCHANGED => {
                host.poller
                    .service()
                    .set_system_is_zh(Lang::detect_system() == Lang::ZhCn);
                let next_theme = SystemTheme::detect();
                if next_theme != host.theme.get() {
                    host.theme.set(next_theme);
                    let snap = host.poller.service().snapshot();
                    if let Ok(mut tray) = host.tray.try_borrow_mut() {
                        tray.update(snap.icon_state, next_theme.taskbar_dark, &snap.tooltip);
                    }
                    host.popover.set_theme(next_theme);
                }
                LRESULT(0)
            }
            WM_DEVICECHANGE if wparam.0 == DBT_DEVNODES_CHANGED => {
                // Debounce bursts of PnP notifications during USB/HID enumeration.
                let _ = SetTimer(hwnd, TIMER_DEVICE_CHANGE, 1_000, None);
                LRESULT(0)
            }
            WM_TIMER if wparam.0 == TIMER_DEVICE_CHANGE => {
                let _ = KillTimer(hwnd, TIMER_DEVICE_CHANGE);
                host.poller.request_refresh();
                LRESULT(0)
            }
            WM_TRAY_CALLBACK => {
                // Under NOTIFYICON_VERSION_4, a single click fires both WM_LBUTTONUP and
                // NIN_SELECT. Only handle the canonical v4 notifications to avoid toggling twice.
                let event = (lparam.0 & 0xFFFF) as u32;
                match event {
                    NIN_SELECT | NIN_KEYSELECT => {
                        let anchor = host.tray.try_borrow().ok().and_then(|t| t.anchor_rect());
                        let was_visible = host.popover.is_visible();
                        host.popover.toggle(anchor);
                        if !was_visible && host.popover.is_visible() {
                            maybe_refresh_on_open(host);
                        }
                    }
                    WM_CONTEXTMENU => {
                        host.popover.hide();
                        let cfg = host.poller.service().config();
                        let lang = Lang::resolve(cfg.language);
                        let cmd = TrayIconHost::show_context_menu_for(
                            host.host_hwnd,
                            cfg.autostart,
                            cfg.low_battery_notify,
                            cfg.low_battery_silent,
                            cfg.theme_mode,
                            cfg.language,
                            lang,
                        );
                        match cmd {
                            Some(MENU_CMD_OPEN_PANEL) => {
                                let anchor =
                                    host.tray.try_borrow().ok().and_then(|t| t.anchor_rect());
                                host.popover.show(anchor);
                                maybe_refresh_on_open(host);
                            }
                            Some(MENU_CMD_THEME_SYSTEM) => {
                                dispatch_popover_action(
                                    host,
                                    PopoverAction::SetThemeMode(ThemeMode::System),
                                );
                            }
                            Some(MENU_CMD_THEME_LIGHT) => {
                                dispatch_popover_action(
                                    host,
                                    PopoverAction::SetThemeMode(ThemeMode::Light),
                                );
                            }
                            Some(MENU_CMD_THEME_DARK) => {
                                dispatch_popover_action(
                                    host,
                                    PopoverAction::SetThemeMode(ThemeMode::Dark),
                                );
                            }
                            Some(MENU_CMD_LANG_SYSTEM) => {
                                dispatch_popover_action(
                                    host,
                                    PopoverAction::SetLanguageMode(LanguageMode::System),
                                );
                            }
                            Some(MENU_CMD_LANG_ZH_CN) => {
                                dispatch_popover_action(
                                    host,
                                    PopoverAction::SetLanguageMode(LanguageMode::ZhCn),
                                );
                            }
                            Some(MENU_CMD_LANG_EN) => {
                                dispatch_popover_action(
                                    host,
                                    PopoverAction::SetLanguageMode(LanguageMode::En),
                                );
                            }
                            Some(MENU_CMD_TOGGLE_LOW_BATTERY_NOTIFY) => {
                                dispatch_popover_action(
                                    host,
                                    PopoverAction::ToggleLowBatteryNotify,
                                );
                            }
                            Some(MENU_CMD_TOGGLE_LOW_BATTERY_SILENT) => {
                                dispatch_popover_action(
                                    host,
                                    PopoverAction::ToggleLowBatterySilent,
                                );
                            }
                            Some(MENU_CMD_TOGGLE_AUTOSTART) => {
                                dispatch_popover_action(host, PopoverAction::ToggleAutostart);
                            }
                            Some(MENU_CMD_CHECK_UPDATES) => {
                                host.popover.set_update_status(UpdateUiStatus::Checking);
                                spawn_update_check(host, true);
                            }
                            Some(MENU_CMD_QUIT) => {
                                PostQuitMessage(0);
                            }
                            #[cfg(feature = "dev-tools")]
                            Some(cmd) => {
                                if let Ok(mut tray) = host.tray.try_borrow_mut() {
                                    crate::dev_tools::handle_context_menu_cmd(
                                        cmd,
                                        &host.dev_panel,
                                        &host.poller,
                                        &mut tray,
                                        &host.popover,
                                        host.theme.get(),
                                    );
                                }
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
    fn maybe_refresh_on_open(host: &HostState) {
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

    fn dispatch_popover_action(host: &HostState, action: PopoverAction) {
        match action {
            PopoverAction::RefreshAll => {
                host.poller.request_refresh();
            }
            PopoverAction::RefreshDevice(id) => {
                host.poller.request_device_refresh(id);
            }
            PopoverAction::CopyError(id) => {
                let lang = Lang::resolve(host.poller.service().config().language);
                match host.poller.service().error_report_json(&id) {
                    Ok(json) => {
                        if copy_text_to_clipboard(host.popover.hwnd(), &json) {
                            host.popover.show_feedback(lang.toast_error_copied());
                        } else {
                            host.popover.show_feedback(lang.toast_copy_failed());
                        }
                    }
                    Err(_) => {
                        host.popover.show_feedback(lang.toast_diag_failed());
                    }
                }
            }
            PopoverAction::SetThemeMode(mode) => {
                let mut cfg = host.poller.service().config();
                if cfg.theme_mode != mode {
                    cfg.theme_mode = mode;
                    let snap = host.poller.service().update_config_async(cfg);
                    host.popover.set_snapshot(snap);
                }
            }
            PopoverAction::SetLanguageMode(mode) => {
                let mut cfg = host.poller.service().config();
                if cfg.language != mode {
                    cfg.language = mode;
                    let snap = host.poller.service().update_config_async(cfg);
                    if let Ok(mut tray) = host.tray.try_borrow_mut() {
                        tray.update(
                            snap.icon_state,
                            host.theme.get().taskbar_dark,
                            &snap.tooltip,
                        );
                    }
                    host.popover.set_snapshot(snap);
                }
            }
            PopoverAction::SetPollInterval(secs) => {
                let mut cfg = host.poller.service().config();
                if cfg.poll_interval_secs != secs {
                    cfg.poll_interval_secs = secs;
                    let snap = host.poller.update_config_async(cfg);
                    host.popover.set_snapshot(snap);
                }
            }
            PopoverAction::ToggleLowBatteryNotify => {
                let mut cfg = host.poller.service().config();
                cfg.low_battery_notify = !cfg.low_battery_notify;
                let snap = host.poller.service().update_config_async(cfg);
                if let Ok(mut tray) = host.tray.try_borrow_mut() {
                    tray.update(
                        snap.icon_state,
                        host.theme.get().taskbar_dark,
                        &snap.tooltip,
                    );
                }
                host.popover.set_snapshot(snap);
            }
            PopoverAction::ToggleLowBatterySilent => {
                let mut cfg = host.poller.service().config();
                cfg.low_battery_silent = !cfg.low_battery_silent;
                let snap = host.poller.service().update_config_async(cfg);
                host.popover.set_snapshot(snap);
            }
            PopoverAction::ToggleAllowExperimental => {
                let mut cfg = host.poller.service().config();
                cfg.allow_experimental = !cfg.allow_experimental;
                let snap = host.poller.update_config_async(cfg);
                if let Ok(mut tray) = host.tray.try_borrow_mut() {
                    tray.update(
                        snap.icon_state,
                        host.theme.get().taskbar_dark,
                        &snap.tooltip,
                    );
                }
                host.popover.set_snapshot(snap);
            }
            PopoverAction::ToggleAutostart => {
                let mut cfg = host.poller.service().config();
                cfg.autostart = !cfg.autostart;
                let enabled = cfg.autostart;
                let snap = host.poller.service().update_config_async(cfg);
                host.popover.set_snapshot(snap);
                let _ = std::thread::Builder::new()
                    .name("wb-tray-autostart".to_owned())
                    .spawn(move || {
                        let _ = set_autostart(enabled);
                    });
            }
            PopoverAction::CheckUpdates => {
                spawn_update_check(host, false);
            }
            PopoverAction::OpenUpdateUrl(url) => {
                updater::open_url(&url);
            }
            PopoverAction::Quit => unsafe {
                PostQuitMessage(0);
            },
        }
    }

    fn spawn_update_check(host: &HostState, from_menu: bool) {
        let tx = host.update_check_tx.clone();
        let hwnd_raw = host.host_hwnd.0;
        let _ = std::thread::Builder::new()
            .name("wb-tray-update-check".to_owned())
            .spawn(move || {
                let result = updater::check_latest_release();
                let _ = tx.send((result, from_menu));
                unsafe {
                    let _ =
                        PostMessageW(HWND(hwnd_raw), WM_UPDATE_CHECK_RESULT, WPARAM(0), LPARAM(0));
                }
            });
    }
}
