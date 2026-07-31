mod config;
mod deepseek;

use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, Monitor, WebviewWindow, WindowEvent,
};

// ============ macOS 平台相关 ============

#[cfg(target_os = "macos")]
mod mac {
    pub const ACCESSIBILITY_HINT: &str = "未授予「辅助功能」权限，系统会丢弃 QuickTrans 发出的复制指令。\
请到 系统设置 → 隐私与安全性 → 辅助功能 中勾选 QuickTrans，再重新按快捷键。";

    // 直接链系统框架，避免为一次权限检查引入额外 crate
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> u8;
    }

    /// 进程是否已获得「辅助功能」授权。未授权时 enigo 合成的按键会被系统静默丢弃，
    /// 取词结果永远为空 —— 必须提前区分，否则只会报「没有选中文本」误导用户。
    pub fn is_trusted() -> bool {
        unsafe { AXIsProcessTrusted() != 0 }
    }

    /// 打开 系统设置 → 隐私与安全性 → 辅助功能
    pub fn open_accessibility_pane() {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}

/// 全局运行期状态
struct AppState {
    config: Mutex<config::Config>,
}

// ============ 自定义命令（前端 invoke 调用，v2 中自定义命令无需 ACL） ============

#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    use arboard::Clipboard;
    let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(text).map_err(|e| e.to_string())?;
    Ok(())
}

/// 隐藏调用它的窗口（浮窗、设置窗通用）
#[tauri::command]
fn hide_window(window: WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
fn open_config_dir(app: tauri::AppHandle) -> Result<(), String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 平台信息 + macOS 辅助功能授权状态，供设置界面按平台渲染
#[derive(Serialize)]
struct PlatformInfo {
    os: &'static str,
    /// 非 macOS 恒为 true（无需该权限）
    accessibility_ok: bool,
}

/// macOS 上未授权辅助功能时取词必然失败，需要引导用户先去授权
#[cfg(target_os = "macos")]
fn accessibility_ok() -> bool {
    mac::is_trusted()
}
#[cfg(not(target_os = "macos"))]
fn accessibility_ok() -> bool {
    true
}

#[tauri::command]
fn platform_info() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS,
        accessibility_ok: accessibility_ok(),
    }
}

/// macOS：跳转到辅助功能授权面板；其他平台空操作
#[tauri::command]
fn open_accessibility_settings() {
    #[cfg(target_os = "macos")]
    {
        mac::open_accessibility_pane();
    }
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    show_settings(&app);
}

/// 返回当前配置，供设置界面预填
#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> config::Config {
    state.config.lock().unwrap().clone()
}

#[derive(Deserialize)]
struct SaveArgs {
    api_key: String,
    model: String,
    target_lang: String,
    base_url: String,
    hotkey: String,
}

/// 保存配置：写入 config.json、更新内存、动态重注册热键（无需重启）
#[tauri::command]
fn save_config(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    args: SaveArgs,
) -> Result<(), String> {
    let cfg = config::Config {
        api_key: args.api_key.trim().to_string(),
        model: non_empty(&args.model, "deepseek-chat"),
        target_lang: non_empty(&args.target_lang, "中文"),
        base_url: non_empty(args.base_url.trim().trim_end_matches('/'), "https://api.deepseek.com"),
        hotkey: non_empty(&args.hotkey, "Ctrl+Alt+T"),
    };

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let txt = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("config.json"), txt).map_err(|e| e.to_string())?;

    *state.config.lock().unwrap() = cfg.clone();
    register_hotkey(&app, &cfg.hotkey);
    Ok(())
}

fn non_empty(v: &str, default: &str) -> String {
    let t = v.trim();
    if t.is_empty() {
        default.to_string()
    } else {
        t.to_string()
    }
}

// ============ 取词：模拟复制快捷键并读取剪贴板（阻塞，放到 blocking 线程） ============

fn grab_selection() -> Result<String, String> {
    use arboard::Clipboard;
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    // 未授权时按键会被系统吞掉，先拦下来给出可操作的提示
    #[cfg(target_os = "macos")]
    {
        if !mac::is_trusted() {
            return Err(mac::ACCESSIBILITY_HINT.to_string());
        }
    }

    // macOS 的复制键是 Cmd（enigo 的 Key::Meta 在 macOS 映射到 Command），其余平台是 Ctrl。
    // 释放修饰键时 Windows 侧刻意不碰 Meta —— 单独发一个 Win 键抬起会招来开始菜单。
    #[cfg(target_os = "macos")]
    let copy_mod = Key::Meta;
    #[cfg(target_os = "macos")]
    let release_mods: &[Key] = &[Key::Meta, Key::Alt, Key::Shift, Key::Control];

    #[cfg(not(target_os = "macos"))]
    let copy_mod = Key::Control;
    #[cfg(not(target_os = "macos"))]
    let release_mods: &[Key] = &[Key::Alt, Key::Shift, Key::Control];

    // macOS 合成事件的投递与剪贴板回写都比 Windows 慢，等待时间相应放宽
    #[cfg(target_os = "macos")]
    let (settle, wait) = (Duration::from_millis(180), Duration::from_millis(220));
    #[cfg(not(target_os = "macos"))]
    let (settle, wait) = (Duration::from_millis(120), Duration::from_millis(140));

    let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
    let backup = cb.get_text().ok();

    // 等用户松开触发组合键，避免修饰键干扰
    std::thread::sleep(settle);

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    // 先释放可能仍被按住的修饰键
    for k in release_mods {
        let _ = enigo.key(*k, Direction::Release);
    }
    // 发送 Cmd+C / Ctrl+C
    enigo
        .key(copy_mod, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('c'), Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(copy_mod, Direction::Release)
        .map_err(|e| e.to_string())?;

    // 等系统把选区写入剪贴板
    std::thread::sleep(wait);
    let text = cb.get_text().unwrap_or_default();

    // 还原用户原有剪贴板内容
    if let Some(b) = backup {
        let _ = cb.set_text(b);
    }

    Ok(text)
}

/// 读取鼠标当前位置。
///
/// 单位跨平台不一致：Windows 上 enigo 返回物理像素，macOS 上返回逻辑点。
/// 下面所有摆放计算都统一在「光标所在坐标系」里做，最后再按平台选窗口坐标类型。
fn cursor_pos() -> Option<(i32, i32)> {
    use enigo::{Enigo, Mouse, Settings};
    let enigo = Enigo::new(&Settings::default()).ok()?;
    enigo.location().ok()
}

/// 物理像素 → 光标坐标系的换算系数。macOS 用逻辑点需要除以缩放比，Windows 本来就是物理像素。
#[cfg(target_os = "macos")]
#[inline]
fn px_per_unit(scale_factor: f64) -> f64 {
    scale_factor
}
#[cfg(not(target_os = "macos"))]
#[inline]
fn px_per_unit(_scale_factor: f64) -> f64 {
    1.0
}

/// 找到包含该光标位置的显示器；找不到就退回窗口当前所在的显示器。
fn monitor_at(win: &WebviewWindow, x: f64, y: f64) -> Option<Monitor> {
    win.available_monitors()
        .ok()
        .and_then(|mons| {
            mons.into_iter().find(|m| {
                let k = px_per_unit(m.scale_factor());
                let (p, s) = (m.position(), m.size());
                let (ox, oy) = (p.x as f64 / k, p.y as f64 / k);
                let (w, h) = (s.width as f64 / k, s.height as f64 / k);
                x >= ox && y >= oy && x < ox + w && y < oy + h
            })
        })
        .or_else(|| win.current_monitor().ok().flatten())
}

/// 把浮窗摆在光标旁，并夹回所在显示器的工作区，避免在屏幕边缘被截断。
fn place_popup_near_cursor(win: &WebviewWindow, cx: f64, cy: f64) {
    const OFF_X: f64 = 12.0;
    const OFF_Y: f64 = 18.0;
    const GAP: f64 = 8.0;

    let mon = monitor_at(win, cx, cy);

    // 窗口尺寸：outer_size() 是相对「窗口当前所在屏」的物理像素，必须用窗口自己的缩放比来换算。
    // 用光标所在屏的缩放比，会在两块屏 DPI 不同时把尺寸算成一半或两倍，翻转/夹取就全错了。
    let ws = win
        .scale_factor()
        .ok()
        .or_else(|| mon.as_ref().map(|m| m.scale_factor()))
        .unwrap_or(1.0);
    let wk = px_per_unit(ws);

    // 兜底值取自 tauri.conf.json 里 main 窗口的「逻辑」尺寸；
    // ws / wk 把逻辑单位换算到光标坐标系（macOS 恒为 1，Windows 上是缩放比）。
    let (mut w, mut h) = (420.0 * ws / wk, 340.0 * ws / wk);
    if let Ok(size) = win.outer_size() {
        if size.width > 0 && size.height > 0 {
            w = size.width as f64 / wk;
            h = size.height as f64 / wk;
        }
    }

    let mut px = cx + OFF_X;
    let mut py = cy + OFF_Y;

    if let Some(m) = mon {
        let mk = px_per_unit(m.scale_factor());
        let area = m.work_area();
        // 某些平台可能给不出有效工作区，此时退回整块屏幕
        let (ax, ay, aw, ah) = if area.size.width > 0 && area.size.height > 0 {
            (
                area.position.x as f64 / mk,
                area.position.y as f64 / mk,
                area.size.width as f64 / mk,
                area.size.height as f64 / mk,
            )
        } else {
            let (p, s) = (m.position(), m.size());
            (
                p.x as f64 / mk,
                p.y as f64 / mk,
                s.width as f64 / mk,
                s.height as f64 / mk,
            )
        };

        // 右/下方放不下就翻到光标另一侧，再整体夹进工作区
        if px + w > ax + aw - GAP {
            px = cx - OFF_X - w;
        }
        if py + h > ay + ah - GAP {
            py = cy - OFF_Y - h;
        }
        px = px.clamp(ax + GAP, (ax + aw - w - GAP).max(ax + GAP));
        py = py.clamp(ay + GAP, (ay + ah - h - GAP).max(ay + GAP));
    }

    #[cfg(target_os = "macos")]
    let _ = win.set_position(tauri::LogicalPosition::new(px, py));
    #[cfg(not(target_os = "macos"))]
    let _ = win.set_position(tauri::PhysicalPosition::new(px as i32, py as i32));
}

/// 在鼠标附近显示浮窗
fn show_popup(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if let Some((x, y)) = cursor_pos() {
            place_popup_near_cursor(&win, x as f64, y as f64);
        }
        // show 必须在 set_focus 之前：窗口不可见时 set_focus 是空操作，别调换顺序
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn show_settings(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        // 顺序有讲究：窗口不可见时 set_focus 直接空操作，所以必须 show → unminimize → set_focus。
        // macOS 上把 Accessory 应用拉到前台靠的也是 set_focus（内部走 activateIgnoringOtherApps），
        // AppHandle::show() 只是 ⌘H 的逆操作、对没隐藏过的应用没用，所以这里不需要它。
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

// ============ 触发翻译：取词 -> 显示浮窗 -> 流式请求 ============

fn trigger_translate(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let grabbed = tauri::async_runtime::spawn_blocking(grab_selection).await;
        let text = match grabbed {
            Ok(Ok(t)) => t.trim().to_string(),
            Ok(Err(e)) => {
                show_popup(&app);
                let _ = app.emit("tr://error", format!("取词失败：{e}"));
                return;
            }
            Err(e) => {
                show_popup(&app);
                let _ = app.emit("tr://error", format!("内部线程错误：{e}"));
                return;
            }
        };

        if text.is_empty() {
            show_popup(&app);
            let _ = app.emit(
                "tr://error",
                "没有选中文本（请先用鼠标选中，再按快捷键）".to_string(),
            );
            return;
        }

        let cfg = {
            let state = app.state::<AppState>();
            let guard = state.config.lock().unwrap();
            guard.clone()
        };

        // Key 没配就别弹浮窗了：紧接着打开设置窗会让浮窗失焦、被自动隐藏，
        // 那条错误用户根本来不及看。直接把设置窗顶出来更有用。
        if cfg.api_key.is_empty() {
            show_settings(&app);
            return;
        }

        show_popup(&app);

        let _ = app.emit(
            "tr://start",
            serde_json::json!({
                "source": text,
                "target": cfg.target_lang,
                "model": cfg.model,
            }),
        );

        deepseek::translate_stream(
            app.clone(),
            cfg.base_url,
            cfg.api_key,
            cfg.model,
            cfg.target_lang,
            text,
        )
        .await;
    });
}

// ============ 快捷键解析与注册 ============

fn key_to_code(k: &str) -> Option<tauri_plugin_global_shortcut::Code> {
    use tauri_plugin_global_shortcut::Code;
    let c = match k {
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "space" => Code::Space,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        _ => return None,
    };
    Some(c)
}

fn parse_shortcut(s: &str) -> Option<tauri_plugin_global_shortcut::Shortcut> {
    use tauri_plugin_global_shortcut::{Modifiers, Shortcut};
    let mut mods = Modifiers::empty();
    let mut code = None;
    for part in s.split('+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" | "option" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "super" | "cmd" | "win" | "meta" => mods |= Modifiers::SUPER,
            other => code = key_to_code(other),
        }
    }
    code.map(|c| Shortcut::new(if mods.is_empty() { None } else { Some(mods) }, c))
}

/// 重新注册全局热键：先清空再注册当前配置的快捷键
fn register_hotkey(app: &tauri::AppHandle, hotkey: &str) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if let Some(sc) = parse_shortcut(hotkey) {
        let _ = gs.register(sc);
    }
}

fn reload_config(app: &tauri::AppHandle) {
    if let Ok(dir) = app.path().app_config_dir() {
        let cfg = config::Config::load(&dir);
        register_hotkey(app, &cfg.hotkey);
        if let Some(state) = app.try_state::<AppState>() {
            *state.config.lock().unwrap() = cfg;
        }
    }
}

// ============ 入口 ============

/// 托盘图标：macOS 菜单栏要 template 图（只保留 alpha，由系统按明暗自动反色），
/// 直接放彩色图在深色菜单栏里会很脏；其余平台沿用应用图标。
fn tray_icon(app: &tauri::App) -> tauri::image::Image<'_> {
    #[cfg(target_os = "macos")]
    {
        const TRAY_PNG: &[u8] = include_bytes!("../icons/tray-mac.png");
        if let Ok(img) = tauri::image::Image::from_bytes(TRAY_PNG) {
            return img;
        }
    }
    app.default_window_icon().unwrap().clone()
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // macOS：以「附属程序」身份运行 —— 不占 Dock、不进 Cmd+Tab，符合托盘常驻工具的惯例
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // 载入配置
            let dir = app.path().app_config_dir()?;
            let cfg = config::Config::load(&dir);
            let hotkey = cfg.hotkey.clone();
            // 缺 Key，或 macOS 上还没拿到辅助功能授权，都先把设置窗顶出来
            let need_settings = cfg.api_key.trim().is_empty() || !accessibility_ok();
            app.manage(AppState {
                config: Mutex::new(cfg),
            });

            // 托盘菜单
            let settings_i =
                MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let open_dir_i =
                MenuItem::with_id(app, "open_config", "打开配置目录", true, None::<&str>)?;
            let reload_i =
                MenuItem::with_id(app, "reload", "重新加载配置", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&settings_i, &open_dir_i, &reload_i, &sep, &quit_i],
            )?;
            TrayIconBuilder::new()
                .icon(tray_icon(app))
                .icon_as_template(cfg!(target_os = "macos"))
                .tooltip("QuickTrans 划词翻译")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "settings" => show_settings(app),
                    "open_config" => {
                        let _ = open_config_dir(app.clone());
                    }
                    "reload" => reload_config(app),
                    _ => {}
                })
                .build(app)?;

            // 浮窗：失焦自动隐藏
            if let Some(win) = app.get_webview_window("main") {
                let w = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let _ = w.hide();
                    }
                });
            }

            // 设置窗：点关闭时只隐藏，不销毁（方便再次从托盘打开）
            if let Some(sw) = app.get_webview_window("settings") {
                let s2 = sw.clone();
                sw.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = s2.hide();
                    }
                });
            }

            // 全局快捷键：注册插件（Rust 侧，无需 ACL），触发即翻译
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::ShortcutState;
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, _shortcut, event| {
                            // 同一时刻只注册一个热键，触发即翻译
                            if event.state() == ShortcutState::Pressed {
                                trigger_translate(app);
                            }
                        })
                        .build(),
                )?;
                register_hotkey(app.handle(), &hotkey);
            }

            // 首次启动无 Key：自动弹出设置窗
            if need_settings {
                show_settings(app.handle());
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            copy_text,
            hide_window,
            open_config_dir,
            open_settings,
            get_config,
            save_config,
            platform_info,
            open_accessibility_settings
        ])
        .run(tauri::generate_context!())
        .expect("启动 QuickTrans 失败");
}
