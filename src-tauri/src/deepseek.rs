use futures_util::StreamExt;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

/// 调用 DeepSeek 流式接口，把译文分片通过事件发给前端。
pub async fn translate_stream(
    app: AppHandle,
    base_url: String,
    api_key: String,
    model: String,
    target_lang: String,
    text: String,
) {
    let sys = format!(
        "你是专业翻译引擎。请把用户输入的文本翻译成{target_lang}。\
         只输出译文本身，不要任何解释、注释、拼音或原文复述。尽量保留原文的换行与段落结构。"
    );
    let body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": sys},
            {"role": "user", "content": text}
        ],
        "stream": true,
        "temperature": 1.0
    });

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = match client
        .post(&url)
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = app.emit("tr://error", format!("请求失败：{e}"));
            return;
        }
    };

    if !resp.status().is_success() {
        let code = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        let _ = app.emit("tr://error", format!("接口返回 {code}：{detail}"));
        return;
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();

    while let Some(item) = stream.next().await {
        let chunk = match item {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("tr://error", format!("数据流中断：{e}"));
                return;
            }
        };
        buf.push_str(&String::from_utf8_lossy(&chunk));

        // 按行解析 SSE：形如 `data: {...}`
        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf.drain(..=pos);
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                let _ = app.emit("tr://done", ());
                return;
            }
            if data.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(data) {
                if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
                    if !delta.is_empty() {
                        let _ = app.emit("tr://chunk", delta);
                    }
                }
            }
        }
    }

    let _ = app.emit("tr://done", ());
}
