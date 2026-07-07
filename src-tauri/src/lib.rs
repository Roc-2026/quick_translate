mod config;
mod deepseek;

use std::sync::Mutex;
use std::time::Duration;

use serde::Deserialize;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, PhysicalPosition, WebviewWindow, WindowEvent,
};

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
    Ok(())
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

// ============ 取词：模拟 Ctrl+C 并读取剪贴板（阻塞，放到 blocking 线程） ============

fn grab_selection() -> Result<String, String> {
    use arboard::Clipboard;
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
    let backup = cb.get_text().ok();

    // 等用户松开触发组合键，避免修饰键干扰
    std::thread::sleep(Duration::from_millis(120));

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    // 先释放可能仍被按住的修饰键
    let _ = enigo.key(Key::Alt, Direction::Release);
    let _ = enigo.key(Key::Shift, Direction::Release);
    let _ = enigo.key(Key::Control, Direction::Release);
    // 发送 Ctrl+C
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('c'), Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| e.to_string())?;

    // 等系统把选区写入剪贴板
    std::thread::sleep(Duration::from_millis(140));
    let text = cb.get_text().unwrap_or_default();

    // 还原用户原有剪贴板内容
    if let Some(b) = backup {
        let _ = cb.set_text(b);
    }

    Ok(text)
}

/// 读取鼠标当前位置（物理像素）
fn cursor_pos() -> Option<(i32, i32)> {
    use enigo::{Enigo, Mouse, Settings};
    let enigo = Enigo::new(&Settings::default()).ok()?;
    enigo.location().ok()
}

/// 在鼠标附近显示浮窗
fn show_popup(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if let Some((x, y)) = cursor_pos() {
            let _ = win.set_position(PhysicalPosition::new(x + 12, y + 18));
        }
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn show_settings(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
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

        show_popup(&app);

        if cfg.api_key.is_empty() {
            let _ = app.emit(
                "tr://error",
                "未配置 DeepSeek API Key。请点击托盘图标 →「设置」填入后再试。".to_string(),
            );
            show_settings(&app);
            return;
        }

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

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 载入配置
            let dir = app.path().app_config_dir()?;
            let cfg = config::Config::load(&dir);
            let hotkey = cfg.hotkey.clone();
            let need_settings = cfg.api_key.trim().is_empty();
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
                .icon(app.default_window_icon().unwrap().clone())
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
            save_config
        ])
        .run(tauri::generate_context!())
        .expect("启动 QuickTrans 失败");
}
