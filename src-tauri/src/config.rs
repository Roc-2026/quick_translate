use serde::{Deserialize, Serialize};
use std::path::Path;

/// DeepSeek 现在只有 V4 两个模型：flash 便宜快，pro 强但贵。
/// 翻译是低延迟短文本场景，默认用 flash。
pub const DEFAULT_MODEL: &str = "deepseek-v4-flash";

/// 这些模型名已于 2026-07-24 15:59 UTC 彻底下线，再请求会直接报错。
/// 老版本的 config.json 里存的就是它们，载入时静默升级到 V4。
const RETIRED_MODELS: &[&str] = &["deepseek-chat", "deepseek-reasoner"];

pub const DEFAULT_TARGET_LANG: &str = "中文";
pub const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";
pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+T";

/// 问答窗快捷键。
///
/// macOS 上用 `Cmd+B`：⌘Space 被 Spotlight 占着，注册会直接失败。
/// 代价是全局抢占 ⌘B —— 各家编辑器的「加粗」会失效，嫌碍事就去设置里改。
/// 其他平台上 Ctrl+B 同理会抢掉加粗，且没有 Spotlight 的顾虑，直接加 Alt 让开。
#[cfg(target_os = "macos")]
pub const DEFAULT_ASK_HOTKEY: &str = "Cmd+B";
#[cfg(not(target_os = "macos"))]
pub const DEFAULT_ASK_HOTKEY: &str = "Ctrl+Alt+B";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// DeepSeek API Key（也可用环境变量 DEEPSEEK_API_KEY 覆盖）
    pub api_key: String,
    /// 模型名，`deepseek-v4-flash` 或 `deepseek-v4-pro`
    pub model: String,
    /// 目标语言
    pub target_lang: String,
    /// 接口地址
    pub base_url: String,
    /// 划词翻译的全局快捷键，如 "Ctrl+Alt+T"
    pub hotkey: String,
    /// 唤起问答窗的全局快捷键，如 "Cmd+B"
    pub ask_hotkey: String,
    /// 唤起问答窗时是否顺带取词，把选中内容作为引用上下文
    pub ask_include_selection: bool,
    /// 问答是否开启 V4 的思考模式。开了更聪明但首字延迟明显变长
    pub ask_thinking: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: DEFAULT_MODEL.into(),
            target_lang: DEFAULT_TARGET_LANG.into(),
            base_url: DEFAULT_BASE_URL.into(),
            hotkey: DEFAULT_HOTKEY.into(),
            ask_hotkey: DEFAULT_ASK_HOTKEY.into(),
            ask_include_selection: true,
            ask_thinking: false,
        }
    }
}

impl Config {
    /// 从配置目录读取 config.json；不存在则写入模板。环境变量优先覆盖 api_key。
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("config.json");
        let mut cfg = match std::fs::read_to_string(&path) {
            Ok(txt) => serde_json::from_str::<Config>(&txt).unwrap_or_default(),
            Err(_) => {
                let def = Config::default();
                let _ = std::fs::create_dir_all(dir);
                if let Ok(txt) = serde_json::to_string_pretty(&def) {
                    let _ = std::fs::write(&path, txt);
                }
                def
            }
        };

        // 迁移必须赶在环境变量覆盖之前落盘，否则会把 DEEPSEEK_API_KEY 的值写进配置文件
        if cfg.migrate() {
            if let Ok(txt) = serde_json::to_string_pretty(&cfg) {
                let _ = std::fs::write(&path, txt);
            }
        }

        if let Ok(k) = std::env::var("DEEPSEEK_API_KEY") {
            if !k.trim().is_empty() {
                cfg.api_key = k.trim().to_string();
            }
        }
        cfg
    }

    /// 把过期字段升级到当前可用的值，返回是否真的改动过（改了才需要回写）。
    fn migrate(&mut self) -> bool {
        let mut changed = false;

        let m = self.model.trim();
        if m.is_empty() || RETIRED_MODELS.iter().any(|r| r.eq_ignore_ascii_case(m)) {
            self.model = DEFAULT_MODEL.to_string();
            changed = true;
        }

        // 早于问答窗的配置文件里没有这两个键，serde 会填成默认值；
        // 但若用户手工清空了值，这里兜一下，免得注册一个空快捷键
        if self.ask_hotkey.trim().is_empty() {
            self.ask_hotkey = DEFAULT_ASK_HOTKEY.to_string();
            changed = true;
        }
        if self.hotkey.trim().is_empty() {
            self.hotkey = DEFAULT_HOTKEY.to_string();
            changed = true;
        }

        changed
    }
}
