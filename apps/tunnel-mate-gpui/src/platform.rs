use super::*;

#[cfg(any(target_os = "macos", target_os = "windows"))]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOW};
#[cfg(target_os = "macos")]
use {
    core::cell::Cell,
    objc2::{
        define_class, msg_send, rc::Retained, runtime::NSObject, sel, DefinedClass,
        MainThreadMarker, MainThreadOnly,
    },
    objc2_app_kit::{
        NSApplication, NSWindow, NSWindowButton, NSWindowDidUpdateNotification, NSWindowStyleMask,
    },
    objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectProtocol},
};

#[cfg(target_os = "macos")]
pub(crate) const WINDOWED_CONTENT_TOP_INSET: f32 = 38.0;

#[cfg(target_os = "windows")]
fn set_window_visible(window: &Window, visible: bool) {
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let hwnd = HWND(handle.hwnd.get() as *mut _);
    unsafe {
        let _ = ShowWindow(hwnd, if visible { SW_SHOW } else { SW_HIDE });
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn hide_window(window: &Window) {
    set_window_visible(window, false);
}

#[cfg(target_os = "macos")]
fn native_window(window: &Window) -> Option<Retained<NSWindow>> {
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return None;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    let view = (unsafe {
        handle
            .ns_view
            .as_ptr()
            .cast::<objc2_app_kit::NSView>()
            .as_ref()
    })?;
    view.window()
}

#[cfg(target_os = "macos")]
fn with_native_window(window: &Window, action: impl FnOnce(&NSWindow)) {
    if let Some(native_window) = native_window(window) {
        action(&native_window);
    }
}

#[cfg(target_os = "macos")]
struct ContentLayoutObserverIvars {
    window: Retained<NSWindow>,
    sender: async_channel::Sender<AppMessage>,
    last_top_inset: Cell<f32>,
    windowed_top_inset: Cell<f32>,
}

#[cfg(target_os = "macos")]
define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "TunnelMateContentLayoutObserver"]
    #[ivars = ContentLayoutObserverIvars]
    struct NativeContentLayoutObserver;

    impl NativeContentLayoutObserver {
        #[unsafe(method(windowDidUpdate:))]
        fn window_did_update(&self, _notification: &NSNotification) {
            self.publish_top_inset();
        }
    }

    unsafe impl NSObjectProtocol for NativeContentLayoutObserver {}
);

#[cfg(target_os = "macos")]
impl NativeContentLayoutObserver {
    fn new(
        window: Retained<NSWindow>,
        sender: async_channel::Sender<AppMessage>,
    ) -> Retained<Self> {
        let main_thread =
            MainThreadMarker::new().expect("window observer must use the main thread");
        let observer = Self::alloc(main_thread).set_ivars(ContentLayoutObserverIvars {
            window,
            sender,
            last_top_inset: Cell::new(f32::NAN),
            windowed_top_inset: Cell::new(WINDOWED_CONTENT_TOP_INSET),
        });
        let observer: Retained<Self> = unsafe { msg_send![super(observer), init] };
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                &observer,
                sel!(windowDidUpdate:),
                Some(NSWindowDidUpdateNotification),
                Some(&observer.ivars().window),
            );
        }
        observer.publish_top_inset();
        observer
    }

    fn publish_top_inset(&self) {
        let content_top_inset = self
            .ivars()
            .window
            .contentView()
            .map_or(0.0, |view| view.safeAreaInsets().top.max(0.0) as f32);
        let is_fullscreen = self
            .ivars()
            .window
            .styleMask()
            .contains(NSWindowStyleMask::FullScreen);
        let top_inset = if is_fullscreen {
            self.fullscreen_top_inset()
        } else {
            if content_top_inset > 0.0 {
                self.ivars().windowed_top_inset.set(content_top_inset);
            }
            WINDOWED_CONTENT_TOP_INSET
        };
        if (self.ivars().last_top_inset.get() - top_inset).abs() >= 0.5
            || self.ivars().last_top_inset.get().is_nan()
        {
            self.ivars().last_top_inset.set(top_inset);
            let _ = self
                .ivars()
                .sender
                .try_send(AppMessage::WindowContentTopInset(top_inset));
        }
    }

    fn fullscreen_top_inset(&self) -> f32 {
        let window = &self.ivars().window;
        let application = NSApplication::sharedApplication(self.mtm());
        let menu_bar_height = application
            .mainMenu()
            .map_or(0.0, |menu| menu.menuBarHeight().max(0.0) as f32);
        let revealed_height = menu_bar_height + self.ivars().windowed_top_inset.get();
        let reveal_progress = window
            .standardWindowButton(NSWindowButton::CloseButton)
            .and_then(|button| unsafe { button.superview() })
            .and_then(|container| unsafe { container.superview() })
            .map_or(0.0, |titlebar| {
                let frame = titlebar.frame();
                if frame.size.height > 0.0 {
                    ((frame.origin.y + frame.size.height) / frame.size.height).clamp(0.0, 1.0)
                        as f32
                } else {
                    0.0
                }
            });
        revealed_height * reveal_progress
    }
}

#[cfg(target_os = "macos")]
impl Drop for NativeContentLayoutObserver {
    fn drop(&mut self) {
        unsafe {
            NSNotificationCenter::defaultCenter().removeObserver_name_object(
                self,
                Some(NSWindowDidUpdateNotification),
                Some(&self.ivars().window),
            );
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) struct WindowLayoutObserver {
    _native: Retained<NativeContentLayoutObserver>,
}

#[cfg(target_os = "macos")]
pub(crate) fn observe_window_layout(
    window: &Window,
    sender: async_channel::Sender<AppMessage>,
) -> Option<WindowLayoutObserver> {
    Some(WindowLayoutObserver {
        _native: NativeContentLayoutObserver::new(native_window(window)?, sender),
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn hide_window(window: &Window) {
    with_native_window(window, |window| window.orderOut(None));
}

// Wayland has no portable request for fully hiding a top-level window. GPUI's
// minimize implementation uses xdg_toplevel.set_minimized on Wayland and
// WM_CHANGE_STATE on X11, so this is the reliable cross-desktop fallback.
#[cfg(target_os = "linux")]
pub(crate) fn hide_window(window: &Window) {
    window.minimize_window();
}

#[cfg(target_os = "windows")]
pub(crate) fn show_window(window: &Window) {
    set_window_visible(window, true);
}

#[cfg(target_os = "macos")]
pub(crate) fn show_window(window: &Window) {
    with_native_window(window, |window| window.makeKeyAndOrderFront(None));
}

#[cfg(target_os = "linux")]
pub(crate) fn show_window(_window: &Window) {}

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

#[cfg(target_os = "macos")]
pub(crate) fn launched_as_login_item() -> bool {
    use objc2_core_services::{kAEOpenApplication, keyAELaunchedAsLogInItem, keyAEPropData};
    use objc2_foundation::NSAppleEventManager;

    let manager = NSAppleEventManager::sharedAppleEventManager();
    let Some(event) = manager.currentAppleEvent() else {
        return false;
    };
    event.eventID() == kAEOpenApplication
        && event
            .paramDescriptorForKeyword(keyAEPropData)
            .is_some_and(|descriptor| descriptor.enumCodeValue() == keyAELaunchedAsLogInItem)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn launched_as_login_item() -> bool {
    false
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
            traffic_light_position: Some(point(px(16.0), px(12.0))),
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

#[cfg(target_os = "macos")]
fn update_window_for_global_action(
    cx: &mut App,
    window_handle: WindowHandle<TunnelMateApp>,
    update: impl FnOnce(&mut TunnelMateApp, &mut Window, &mut Context<TunnelMateApp>) + 'static,
) {
    cx.defer(move |cx| {
        let _ = window_handle.update(cx, update);
    });
}

#[cfg(not(target_os = "macos"))]
fn update_window_for_global_action(
    cx: &mut App,
    window_handle: WindowHandle<TunnelMateApp>,
    update: impl FnOnce(&mut TunnelMateApp, &mut Window, &mut Context<TunnelMateApp>) + 'static,
) {
    let _ = window_handle.update(cx, update);
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

    let handle = window_handle;
    cx.on_action(move |_: &CloseWindow, cx| {
        update_window_for_global_action(cx, handle, |app, window, cx| {
            app.request_close(window, cx)
        });
    });

    let weak = app.downgrade();
    cx.on_action(move |_: &QuitApplication, cx| {
        let _ = weak.update(cx, |app, _| app.request_quit());
    });

    cx.on_action(|_: &HideApplication, cx| cx.hide());

    let handle = window_handle;
    cx.on_action(move |_: &MinimizeWindow, cx| {
        update_window_for_global_action(cx, handle, |_, window, _| window.minimize_window());
    });

    let handle = window_handle;
    cx.on_action(move |_: &ZoomWindow, cx| {
        update_window_for_global_action(cx, handle, |_, window, _| window.zoom_window());
    });

    let handle = window_handle;
    cx.on_action(move |_: &ToggleFullScreen, cx| {
        update_window_for_global_action(cx, handle, |_, window, _| window.toggle_fullscreen());
    });

    cx.on_action(move |_: &BringAllToFront, cx| {
        cx.activate(true);
        update_window_for_global_action(cx, window_handle, |_, window, _| window.activate_window());
    });
}
