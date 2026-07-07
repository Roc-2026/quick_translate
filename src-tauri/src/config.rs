use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// DeepSeek API Key（也可用环境变量 DEEPSEEK_API_KEY 覆盖）
    pub api_key: String,
    /// 模型名，翻译用 deepseek-chat 即可
    pub model: String,
    /// 目标语言
    pub target_lang: String,
    /// 接口地址
    pub base_url: String,
    /// 全局快捷键，如 "Ctrl+Alt+T"
    pub hotkey: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "deepseek-chat".into(),
            target_lang: "中文".into(),
            base_url: "https://api.deepseek.com".into(),
            hotkey: "Ctrl+Alt+T".into(),
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
        if let Ok(k) = std::env::var("DEEPSEEK_API_KEY") {
            if !k.trim().is_empty() {
                cfg.api_key = k.trim().to_string();
            }
        }
        cfg
    }
}
