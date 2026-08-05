mod config;
mod deepseek;

use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, Monitor, WebviewWindow, WindowEvent,
};
use tauri_plugin_global_shortcut::Shortcut;

/// 问答窗的系统提示。翻译那边的 prompt 在 deepseek.rs 里就地拼。
const ASK_SYSTEM_PROMPT: &str = "你是一个严谨、简洁的助手，正在一个桌面小浮窗里回答问题。\
直接给结论，需要时再展开；不确定就明说，不要编造。默认用中文回答，除非用户用其他语言提问。";

// macOS 的 NSWindowLevel。非 macOS 上不会被用到，放在这里只是为了让调用点不必到处写 cfg。
/// NSFloatingWindowLevel
const LEVEL_FLOATING: isize = 3;
/// NSStatusWindowLevel，比菜单栏(24)还高一档
const LEVEL_STATUS: isize = 25;

// ============ macOS 平台相关 ============

#[cfg(target_os = "macos")]
mod mac {
    pub const ACCESSIBILITY_HINT: &str = "未授予「辅助功能」权限，系统会丢弃 QuickTrans 发出的复制指令。\
请到 系统设置 → 隐私与安全性 → 辅助功能 中勾选 QuickTrans，再重新按快捷键。";

    /// `c` 键的 virtual keycode（Carbon `Events.h` 里的 `kVK_ANSI_C`）。
    ///
    /// 必须硬编码：enigo 的 `Key::Unicode('c')` 会去反查当前键盘布局下哪个 keycode
    /// 打出 'c'，而那条路径最终调到 Carbon 的 `TISGetInputSourceProperty`，该 API 带
    /// `dispatch_assert_queue(main)` 断言 —— 在非主线程调用直接 SIGTRAP 打死整个进程。
    /// 取词跑在 tokio blocking 线程上，所以只能绕开反查、直接发 keycode。
    ///
    /// 硬编码 keycode 绑的是物理键位而非字符，但 ⌘C 在各主流布局上键位一致
    /// （系统菜单快捷键本身就是按键位工作的），对复制这个用途是安全的。
    pub const KEYCODE_C: u16 = 8;

    // NSWindowCollectionBehavior 的位（AppKit/NSWindow.h）
    /// 窗口出现在所有 Space —— 缺了它，在别的 App 的全屏 Space 里按快捷键，
    /// 系统会把你拽回 QuickTrans 自己的 Space 再显示窗口，看起来就是「没反应」。
    const CB_CAN_JOIN_ALL_SPACES: usize = 1 << 0;
    /// Space 切换时不跟着做位移动画
    const CB_STATIONARY: usize = 1 << 4;
    /// 允许与别的 App 的全屏窗口共存于同一 Space。和上面那一位缺一不可。
    const CB_FULLSCREEN_AUXILIARY: usize = 1 << 8;

    /// 让窗口能盖在别的 App 的全屏窗口上。
    ///
    /// tao 的 `set_visible_on_all_workspaces()` 只设了 `CanJoinAllSpaces`，
    /// 少了 `FullScreenAuxiliary`，在全屏 App 上仍然出不来，所以这里直接发消息。
    ///
    /// `level` 传 `None` 表示保持窗口原有层级（普通窗口用，比如设置窗）。
    ///
    /// **必须在主线程调用** —— AppKit 的硬性要求。
    pub fn make_overlay(win: &tauri::WebviewWindow, level: Option<isize>) {
        use objc2::msg_send;
        use objc2::runtime::AnyObject;

        let Ok(ptr) = win.ns_window() else { return };
        if ptr.is_null() {
            return;
        }
        let ns = ptr as *mut AnyObject;
        // 显式标注类型：msg_send! 不查头文件，参数得自带完整类型
        // （NSWindowCollectionBehavior 是 NSUInteger，NSWindowLevel 是 NSInteger）
        let behavior: usize = CB_CAN_JOIN_ALL_SPACES | CB_FULLSCREEN_AUXILIARY | CB_STATIONARY;

        unsafe {
            let _: () = msg_send![ns, setCollectionBehavior: behavior];
            // 托盘应用失活是常态，别因为失活就把窗口收走。msg_send! 会自动把 bool 转成 BOOL
            let _: () = msg_send![ns, setHidesOnDeactivate: false];
            if let Some(l) = level {
                let _: () = msg_send![ns, setLevel: l];
            }
        }
    }

    // 直接链系统框架，避免为权限检查引入额外 crate
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> u8;
        /// 是否已获得「发送合成事件」权限。不弹窗、不注册，纯查询。
        fn CGPreflightPostEventAccess() -> u8;
        /// 请求该权限：首次调用会弹系统授权窗，并把本 App 写进辅助功能列表。
        /// 返回值只反映"当前是否已授权"，用户当场勾选后本次进程内一般不会变 true。
        fn CGRequestPostEventAccess() -> u8;
    }

    /// 进程是否能发出合成按键。未授权时 enigo 合成的按键会被系统静默丢弃，
    /// 取词结果永远为空 —— 必须提前区分，否则只会报「没有选中文本」误导用户。
    ///
    /// 发按键要的是 PostEvent 权限，它与 Accessibility 是两个独立的 TCC 条目，
    /// 只是都显示在「隐私与安全性 → 辅助功能」下。两者任一成立即可放行：
    /// 一般认为 Accessibility 覆盖 PostEvent，但 Apple 未作保证，故都查一遍。
    pub fn is_trusted() -> bool {
        unsafe { CGPreflightPostEventAccess() != 0 || AXIsProcessTrusted() != 0 }
    }

    /// 未授权时主动请求，让系统自己弹窗并把 App 加进辅助功能列表。
    ///
    /// 不这么做的话列表里根本不会出现 QuickTrans —— 纯 preflight/AXIsProcessTrusted
    /// 只读状态、不向 TCC 注册，用户只能手动点 `+` 去翻二进制路径。
    pub fn request_post_event_access() -> bool {
        unsafe {
            if CGPreflightPostEventAccess() != 0 {
                return true;
            }
            CGRequestPostEventAccess() != 0
        }
    }

    /// 打开 系统设置 → 隐私与安全性 → 辅助功能
    pub fn open_accessibility_pane() {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}

/// 两把全局热键的注册结果。空串代表正常，非空是给用户看的原因。
#[derive(Debug, Clone, Default, Serialize)]
struct HotkeyStatus {
    translate_err: String,
    ask_err: String,
}

/// 当前生效的热键。handler 拿它来判断按下的是哪一把。
#[derive(Default)]
struct Hotkeys {
    translate: Option<Shortcut>,
    ask: Option<Shortcut>,
    status: HotkeyStatus,
}

/// 全局运行期状态
struct AppState {
    config: Mutex<config::Config>,
    hotkeys: Mutex<Hotkeys>,
}

// ============ 自定义命令（前端 invoke 调用，v2 中自定义命令无需 ACL） ============

#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    use arboard::Clipboard;
    let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(text).map_err(|e| e.to_string())?;
    Ok(())
}

/// 隐藏调用它的窗口（浮窗、问答窗、设置窗通用）
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

/// macOS：请求授权（会让系统把本 App 注册进辅助功能列表）并跳转到授权面板；其他平台空操作
#[tauri::command]
fn open_accessibility_settings() {
    #[cfg(target_os = "macos")]
    {
        // 先 request 再 open：request 负责把条目写进列表，否则用户跳过去也找不到 QuickTrans
        let _ = mac::request_post_event_access();
        mac::open_accessibility_pane();
    }
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    show_settings(&app);
}

/// 热键注册结果，供设置界面提示「该组合被别人占了」
#[tauri::command]
fn hotkey_status(state: tauri::State<AppState>) -> HotkeyStatus {
    state.hotkeys.lock().unwrap().status.clone()
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
    ask_hotkey: String,
    ask_include_selection: bool,
    ask_thinking: bool,
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
        model: non_empty(&args.model, config::DEFAULT_MODEL),
        target_lang: non_empty(&args.target_lang, config::DEFAULT_TARGET_LANG),
        base_url: non_empty(
            args.base_url.trim().trim_end_matches('/'),
            config::DEFAULT_BASE_URL,
        ),
        hotkey: non_empty(&args.hotkey, config::DEFAULT_HOTKEY),
        ask_hotkey: non_empty(&args.ask_hotkey, config::DEFAULT_ASK_HOTKEY),
        ask_include_selection: args.ask_include_selection,
        ask_thinking: args.ask_thinking,
    };

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let txt = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("config.json"), txt).map_err(|e| e.to_string())?;

    *state.config.lock().unwrap() = cfg.clone();
    register_hotkeys(&app, &cfg);
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

// ============ 问答 ============

#[derive(Deserialize)]
struct AskMsg {
    role: String,
    content: String,
}

/// 前端把整段对话历史传上来，这里直接转发 —— 多轮状态留在前端，Rust 侧无状态。
#[tauri::command]
fn ask_send(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    messages: Vec<AskMsg>,
) -> Result<(), String> {
    let cfg = state.config.lock().unwrap().clone();
    if cfg.api_key.trim().is_empty() {
        show_settings(&app);
        return Err("还没有填 DeepSeek API Key，已为你打开设置。".into());
    }
    if messages.is_empty() {
        return Err("没有内容可提问".into());
    }

    let mut msgs: Vec<Value> = vec![json!({"role": "system", "content": ASK_SYSTEM_PROMPT})];
    msgs.extend(
        messages
            .into_iter()
            .map(|m| json!({"role": m.role, "content": m.content})),
    );

    tauri::async_runtime::spawn(async move {
        deepseek::chat_stream(
            app,
            deepseek::ChatRequest {
                base_url: cfg.base_url,
                api_key: cfg.api_key,
                model: cfg.model,
                messages: msgs,
                thinking: cfg.ask_thinking,
            },
            &deepseek::ASK,
        )
        .await;
    });
    Ok(())
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
    // macOS 走 raw(keycode)：Key::Unicode 会触发键盘布局反查，而那条路径调的
    // Carbon API 只允许主线程调用，在这个 blocking 线程上会直接崩掉进程。详见 mac::KEYCODE_C。
    #[cfg(target_os = "macos")]
    enigo
        .raw(mac::KEYCODE_C, Direction::Click)
        .map_err(|e| e.to_string())?;
    #[cfg(not(target_os = "macos"))]
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

/// 显示器可用区域，换算到光标坐标系，返回 (x, y, 宽, 高)。
/// 某些平台可能给不出有效工作区，此时退回整块屏幕。
fn work_area_of(m: &Monitor) -> (f64, f64, f64, f64) {
    let k = px_per_unit(m.scale_factor());
    let area = m.work_area();
    if area.size.width > 0 && area.size.height > 0 {
        (
            area.position.x as f64 / k,
            area.position.y as f64 / k,
            area.size.width as f64 / k,
            area.size.height as f64 / k,
        )
    } else {
        let (p, s) = (m.position(), m.size());
        (
            p.x as f64 / k,
            p.y as f64 / k,
            s.width as f64 / k,
            s.height as f64 / k,
        )
    }
}

/// 窗口尺寸换算到光标坐标系，返回 (宽, 高)。
///
/// `outer_size()` 是相对「窗口当前所在屏」的物理像素，必须用窗口自己的缩放比来换算。
/// 用光标所在屏的缩放比，会在两块屏 DPI 不同时把尺寸算成一半或两倍，翻转/夹取就全错了。
/// 拿不到实际尺寸时用 `fallback`（tauri.conf.json 里配的逻辑尺寸）兜底。
fn window_size_in_cursor_units(
    win: &WebviewWindow,
    mon: Option<&Monitor>,
    fallback: (f64, f64),
) -> (f64, f64) {
    let ws = win
        .scale_factor()
        .ok()
        .or_else(|| mon.map(|m| m.scale_factor()))
        .unwrap_or(1.0);
    let wk = px_per_unit(ws);

    // ws / wk 把逻辑单位换算到光标坐标系（macOS 恒为 1，Windows 上是缩放比）
    let (mut w, mut h) = (fallback.0 * ws / wk, fallback.1 * ws / wk);
    if let Ok(size) = win.outer_size() {
        if size.width > 0 && size.height > 0 {
            w = size.width as f64 / wk;
            h = size.height as f64 / wk;
        }
    }
    (w, h)
}

#[cfg(target_os = "macos")]
fn move_window(win: &WebviewWindow, x: f64, y: f64) {
    let _ = win.set_position(tauri::LogicalPosition::new(x, y));
}
#[cfg(not(target_os = "macos"))]
fn move_window(win: &WebviewWindow, x: f64, y: f64) {
    let _ = win.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
}

/// 把浮窗摆在光标旁，并夹回所在显示器的工作区，避免在屏幕边缘被截断。
fn place_popup_near_cursor(win: &WebviewWindow, cx: f64, cy: f64) {
    const OFF_X: f64 = 12.0;
    const OFF_Y: f64 = 18.0;
    const GAP: f64 = 8.0;

    let mon = monitor_at(win, cx, cy);
    let (w, h) = window_size_in_cursor_units(win, mon.as_ref(), (420.0, 340.0));

    let mut px = cx + OFF_X;
    let mut py = cy + OFF_Y;

    if let Some(m) = mon {
        let (ax, ay, aw, ah) = work_area_of(&m);

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

    move_window(win, px, py);
}

/// 把问答窗摆在光标所在那块屏的中央偏上（Spotlight 那种观感）。
fn place_ask_on_cursor_monitor(win: &WebviewWindow, cx: f64, cy: f64) {
    let mon = monitor_at(win, cx, cy);
    let (w, h) = window_size_in_cursor_units(win, mon.as_ref(), (640.0, 460.0));

    let Some(m) = mon else {
        move_window(win, cx - w / 2.0, cy - h / 2.0);
        return;
    };
    let (ax, ay, aw, ah) = work_area_of(&m);
    let px = ax + (aw - w) / 2.0;
    // 垂直放在 1/3 处而不是正中：视线落点更自然，也给下方留出答案展开的余地
    let py = ay + ((ah - h) / 3.0).max(0.0);

    move_window(
        win,
        px.clamp(ax, (ax + aw - w).max(ax)),
        py.clamp(ay, (ay + ah - h).max(ay)),
    );
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

fn show_ask(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("ask") {
        if let Some((x, y)) = cursor_pos() {
            place_ask_on_cursor_monitor(&win, x as f64, y as f64);
        }
        let _ = win.show();
        let _ = win.unminimize();
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
            json!({
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

// ============ 触发问答：可选取词 -> 弹窗 -> 等用户提问 ============

fn trigger_ask(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let cfg = {
            let state = app.state::<AppState>();
            let guard = state.config.lock().unwrap();
            guard.clone()
        };

        if cfg.api_key.trim().is_empty() {
            show_settings(&app);
            return;
        }

        // 取词失败（没选中、没授权）都不该拦住提问，静默降级成空引用即可
        let mut quote = String::new();
        if cfg.ask_include_selection {
            if let Ok(Ok(t)) = tauri::async_runtime::spawn_blocking(grab_selection).await {
                quote = t.trim().to_string();
            }
        }

        show_ask(&app);
        let _ = app.emit("ask://open", json!({ "quote": quote, "model": cfg.model }));
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

fn parse_shortcut(s: &str) -> Option<Shortcut> {
    use tauri_plugin_global_shortcut::Modifiers;
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

/// 重新注册两把全局热键：先清空再按当前配置注册，并把失败原因留给设置界面。
///
/// 注册失败必须报出来 —— 系统级组合（比如 macOS 的 ⌘Space 被 Spotlight 占着）
/// 会直接注册失败，静默吞掉的话用户只会觉得「按了没反应」，无从排查。
fn register_hotkeys(app: &tauri::AppHandle, cfg: &config::Config) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let mut hk = Hotkeys::default();

    match parse_shortcut(&cfg.hotkey) {
        None => hk.status.translate_err = format!("「{}」不是合法的快捷键写法", cfg.hotkey),
        Some(sc) => match gs.register(sc) {
            Ok(()) => hk.translate = Some(sc),
            Err(e) => {
                hk.status.translate_err =
                    format!("「{}」注册失败，可能已被系统或其他软件占用：{e}", cfg.hotkey)
            }
        },
    }

    match parse_shortcut(&cfg.ask_hotkey) {
        None => hk.status.ask_err = format!("「{}」不是合法的快捷键写法", cfg.ask_hotkey),
        Some(sc) if hk.translate == Some(sc) => {
            hk.status.ask_err = format!("「{}」和划词翻译的快捷键重复了", cfg.ask_hotkey)
        }
        Some(sc) => match gs.register(sc) {
            Ok(()) => hk.ask = Some(sc),
            Err(e) => {
                hk.status.ask_err = format!(
                    "「{}」注册失败，可能已被系统或其他软件占用：{e}",
                    cfg.ask_hotkey
                )
            }
        },
    }

    if let Some(state) = app.try_state::<AppState>() {
        *state.hotkeys.lock().unwrap() = hk;
    }
}

fn reload_config(app: &tauri::AppHandle) {
    if let Ok(dir) = app.path().app_config_dir() {
        let cfg = config::Config::load(&dir);
        register_hotkeys(app, &cfg);
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

/// macOS：让窗口能盖在别的 App 的全屏窗口上。其他平台空操作。
///
/// 必须在 setup 里（主线程）调用。
#[allow(unused_variables)]
fn setup_overlay(app: &tauri::App, label: &str, level_macos: Option<isize>) {
    #[cfg(target_os = "macos")]
    {
        if let Some(win) = app.get_webview_window(label) {
            mac::make_overlay(&win, level_macos);
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // macOS：以「附属程序」身份运行 —— 不占 Dock、不进 Cmd+Tab，符合托盘常驻工具的惯例
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // macOS：启动时就请求发合成事件的权限。
            // 这一步的意义是让系统弹授权窗、并把 QuickTrans 写进「辅助功能」列表 ——
            // 只做 preflight 查询不会注册，列表里根本不会出现本 App，用户只能手动点 + 翻路径。
            #[cfg(target_os = "macos")]
            let _ = mac::request_post_event_access();

            // macOS：三个窗口都要能出现在别的 App 的全屏 Space 里，否则读全屏 PDF 时
            // 按快捷键什么都看不到。层级各不相同：
            //   浮窗小且瞬时，压过菜单栏无妨；问答窗常驻可读，浮动层就够；
            //   设置窗是普通窗口，只加 Space 行为、不抬层级。
            setup_overlay(app, "main", Some(LEVEL_STATUS));
            setup_overlay(app, "ask", Some(LEVEL_FLOATING));
            setup_overlay(app, "settings", None);

            // 载入配置
            let dir = app.path().app_config_dir()?;
            let cfg = config::Config::load(&dir);
            // 缺 Key，或 macOS 上还没拿到辅助功能授权，都先把设置窗顶出来
            let need_settings = cfg.api_key.trim().is_empty() || !accessibility_ok();
            let cfg_for_hotkeys = cfg.clone();
            app.manage(AppState {
                config: Mutex::new(cfg),
                hotkeys: Mutex::new(Hotkeys::default()),
            });

            // 托盘菜单
            let ask_i = MenuItem::with_id(app, "ask", "问答窗口", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let open_dir_i =
                MenuItem::with_id(app, "open_config", "打开配置目录", true, None::<&str>)?;
            let reload_i =
                MenuItem::with_id(app, "reload", "重新加载配置", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&ask_i, &settings_i, &open_dir_i, &reload_i, &sep, &quit_i],
            )?;
            TrayIconBuilder::new()
                .icon(tray_icon(app))
                .icon_as_template(cfg!(target_os = "macos"))
                .tooltip("QuickTrans 划词翻译")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "ask" => {
                        show_ask(app);
                        let _ = app.emit("ask://open", json!({ "quote": "" }));
                    }
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

            // 问答窗：**刻意不做**失焦隐藏 —— 你多半要切回 PDF 对着答案看，
            // 一失焦就消失反而添乱。只认 Esc 和关闭按钮，关闭时也只隐藏不销毁。
            if let Some(aw) = app.get_webview_window("ask") {
                let a2 = aw.clone();
                aw.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = a2.hide();
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

            // 全局快捷键：注册插件（Rust 侧，无需 ACL），按下哪把就做哪件事
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::ShortcutState;
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, shortcut, event| {
                            if event.state() != ShortcutState::Pressed {
                                return;
                            }
                            let (translate, ask) = match app.try_state::<AppState>() {
                                Some(state) => {
                                    let g = state.hotkeys.lock().unwrap();
                                    (g.translate, g.ask)
                                }
                                None => (None, None),
                            };
                            if translate == Some(*shortcut) {
                                trigger_translate(app);
                            } else if ask == Some(*shortcut) {
                                trigger_ask(app);
                            }
                        })
                        .build(),
                )?;
                register_hotkeys(app.handle(), &cfg_for_hotkeys);
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
            open_accessibility_settings,
            hotkey_status,
            ask_send
        ])
        .run(tauri::generate_context!())
        .expect("启动 QuickTrans 失败");
}
