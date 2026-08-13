use super::*;

#[cfg(target_os = "macos")]
pub(crate) fn set_dock_visible(visible: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let Some(main_thread) = MainThreadMarker::new() else {
        eprintln!("Tunnel Mate can only change Dock visibility from the macOS main thread");
        return;
    };
    let application = NSApplication::sharedApplication(main_thread);
    let policy = if visible {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    };
    if !application.setActivationPolicy(policy) {
        eprintln!("Tunnel Mate could not change its macOS activation policy");
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn set_dock_visible(_visible: bool) {}

#[cfg(target_os = "macos")]
pub(crate) fn install_native_behavior(cx: &mut App, language: Language) {
    cx.bind_keys([
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-q", QuitApplication, None),
        KeyBinding::new("cmd-h", HideApplication, None),
        KeyBinding::new("cmd-m", MinimizeWindow, None),
        KeyBinding::new("ctrl-cmd-f", ToggleFullScreen, None),
    ]);

    cx.set_menus([
        AppMenu::new("Tunnel Mate").items([
            AppMenuItem::action(
                language.pick("关于 Tunnel Mate", "About Tunnel Mate"),
                ShowAbout,
            ),
            AppMenuItem::separator(),
            AppMenuItem::action(language.pick("设置…", "Settings…"), OpenSettings),
            AppMenuItem::separator(),
            AppMenuItem::os_submenu(language.pick("服务", "Services"), SystemMenuType::Services),
            AppMenuItem::separator(),
            AppMenuItem::action(
                language.pick("隐藏 Tunnel Mate", "Hide Tunnel Mate"),
                HideApplication,
            ),
            AppMenuItem::separator(),
            AppMenuItem::action(
                language.pick("退出 Tunnel Mate", "Quit Tunnel Mate"),
                QuitApplication,
            ),
        ]),
        AppMenu::new(language.pick("文件", "File")).items([AppMenuItem::action(
            language.pick("关闭窗口", "Close Window"),
            CloseWindow,
        )]),
        AppMenu::new(language.pick("编辑", "Edit")).items([
            AppMenuItem::os_action(language.pick("剪切", "Cut"), text_input::Cut, OsAction::Cut),
            AppMenuItem::os_action(
                language.pick("复制", "Copy"),
                text_input::Copy,
                OsAction::Copy,
            ),
            AppMenuItem::os_action(
                language.pick("粘贴", "Paste"),
                text_input::Paste,
                OsAction::Paste,
            ),
            AppMenuItem::separator(),
            AppMenuItem::os_action(
                language.pick("全选", "Select All"),
                text_input::SelectAll,
                OsAction::SelectAll,
            ),
        ]),
        AppMenu::new(language.pick("窗口", "Window")).items([
            AppMenuItem::action(language.pick("最小化", "Minimize"), MinimizeWindow),
            AppMenuItem::action(language.pick("缩放", "Zoom"), ZoomWindow),
            AppMenuItem::action(
                language.pick("进入全屏幕", "Enter Full Screen"),
                ToggleFullScreen,
            ),
            AppMenuItem::separator(),
            AppMenuItem::action(
                language.pick("前置全部窗口", "Bring All to Front"),
                BringAllToFront,
            ),
        ]),
    ]);
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn install_native_behavior(cx: &mut App, _language: Language) {
    cx.bind_keys([
        KeyBinding::new("ctrl-,", OpenSettings, None),
        KeyBinding::new("ctrl-w", CloseWindow, None),
        KeyBinding::new("ctrl-q", QuitApplication, None),
        KeyBinding::new("f11", ToggleFullScreen, None),
    ]);
}

pub(crate) fn platform_window_options(bounds: Bounds<gpui::Pixels>) -> WindowOptions {
    let mut options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(800.0), px(580.0))),
        ..Default::default()
    };

    #[cfg(target_os = "macos")]
    {
        options.window_background = WindowBackgroundAppearance::Blurred;
        options.titlebar = Some(gpui::TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: Some(point(px(16.0), px(16.0))),
        });
    }

    #[cfg(target_os = "windows")]
    {
        options.window_background = WindowBackgroundAppearance::MicaBackdrop;
        options.titlebar = Some(gpui::TitlebarOptions {
            title: Some("Tunnel Mate".into()),
            appears_transparent: false,
            traffic_light_position: None,
        });
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        options.window_background = WindowBackgroundAppearance::Opaque;
        options.titlebar = Some(gpui::TitlebarOptions {
            title: Some("Tunnel Mate".into()),
            appears_transparent: false,
            traffic_light_position: None,
        });
        options.app_id = Some("com.jiayx.tunnel-mate".to_string());
        options.icon = image::load_from_memory(include_bytes!("../../../assets/icons/128x128.png"))
            .ok()
            .map(|image| Arc::new(image.into_rgba8()));
    }

    options
}

pub(crate) fn register_global_actions(
    cx: &mut App,
    window_handle: WindowHandle<TunnelMateApp>,
    app: Entity<TunnelMateApp>,
) {
    let weak = app.downgrade();
    cx.on_action(move |_: &ShowAbout, cx| {
        let _ = weak.update(cx, |app, cx| app.show_about(cx));
    });

    let weak = app.downgrade();
    cx.on_action(move |_: &OpenSettings, cx| {
        let _ = weak.update(cx, |app, cx| app.open_settings(cx));
    });

    let weak = app.downgrade();
    cx.on_action(move |_: &CloseWindow, cx| {
        let _ = weak.update(cx, |app, cx| app.request_close(cx));
    });

    let weak = app.downgrade();
    cx.on_action(move |_: &QuitApplication, cx| {
        let _ = weak.update(cx, |app, _| app.request_quit());
    });

    cx.on_action(|_: &HideApplication, cx| cx.hide());

    let handle = window_handle;
    cx.on_action(move |_: &MinimizeWindow, cx| {
        let _ = handle.update(cx, |_, window, _| window.minimize_window());
    });

    let handle = window_handle;
    cx.on_action(move |_: &ZoomWindow, cx| {
        let _ = handle.update(cx, |_, window, _| window.zoom_window());
    });

    let handle = window_handle;
    cx.on_action(move |_: &ToggleFullScreen, cx| {
        let _ = handle.update(cx, |_, window, _| window.toggle_fullscreen());
    });

    cx.on_action(move |_: &BringAllToFront, cx| {
        cx.activate(true);
        let _ = window_handle.update(cx, |_, window, _| window.activate_window());
    });
}
