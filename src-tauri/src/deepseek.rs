use futures_util::StreamExt;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

/// 一路流式会话对应的四个事件名。翻译浮窗和问答窗各占一组前缀，
/// 免得两边同时在跑的时候串台。
///
/// 用 `static` 而不是 `const`：调用方要把 `&TRANSLATE` 传进 `spawn` 出去的
/// async 块，`const` 取引用会生成临时值，`static` 直接给到 `&'static`。
pub struct Events {
    pub chunk: &'static str,
    pub reasoning: &'static str,
    pub done: &'static str,
    pub error: &'static str,
}

pub static TRANSLATE: Events = Events {
    chunk: "tr://chunk",
    reasoning: "tr://reasoning",
    done: "tr://done",
    error: "tr://error",
};

pub static ASK: Events = Events {
    chunk: "ask://chunk",
    reasoning: "ask://reasoning",
    done: "ask://done",
    error: "ask://error",
};

/// 一次流式请求要的全部参数。
pub struct ChatRequest {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    /// 已经组装好的 OpenAI 格式消息数组
    pub messages: Vec<Value>,
    /// 是否开启 V4 的思考模式
    pub thinking: bool,
}

/// 翻译：拼好 system prompt 后走通用流式通道。
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

    chat_stream(
        app,
        ChatRequest {
            base_url,
            api_key,
            model,
            messages: vec![
                json!({"role": "system", "content": sys}),
                json!({"role": "user", "content": text}),
            ],
            // 翻译要的是首字尽快出来，而 V4 的思考模式默认是开的，必须显式关掉
            thinking: false,
        },
        &TRANSLATE,
    )
    .await;
}

/// 调用 DeepSeek 流式接口，把分片通过事件发给前端。
pub async fn chat_stream(app: AppHandle, req: ChatRequest, ev: &Events) {
    let mut body = json!({
        "model": req.model,
        "messages": req.messages,
        "stream": true,
        // V4 默认开思考模式，且开着时 temperature 等采样参数会被静默忽略，
        // 所以这个字段必须每次显式给出
        "thinking": { "type": if req.thinking { "enabled" } else { "disabled" } },
    });
    if !req.thinking {
        body["temperature"] = json!(1.0);
    }

    let url = format!("{}/chat/completions", req.base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = match client
        .post(&url)
        .bearer_auth(&req.api_key)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = app.emit(ev.error, format!("请求失败：{e}"));
            return;
        }
    };

    if !resp.status().is_success() {
        let code = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        let mut msg = format!("接口返回 {code}：{detail}");
        // 老配置最容易撞上这个：deepseek-chat / deepseek-reasoner 已经下线了
        if detail.contains("Model Not Exist") || detail.contains("model_not_found") {
            msg.push_str(&format!(
                "\n\n模型 `{}` 不存在。DeepSeek 现在只有 deepseek-v4-flash 和 deepseek-v4-pro，\
                 请到设置里改一下。",
                req.model
            ));
        }
        let _ = app.emit(ev.error, msg);
        return;
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();

    while let Some(item) = stream.next().await {
        let chunk = match item {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit(ev.error, format!("数据流中断：{e}"));
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
                let _ = app.emit(ev.done, ());
                return;
            }
            if data.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(data) {
                let delta = &v["choices"][0]["delta"];
                // 思考模式下正文之前会先吐一大段 reasoning_content，
                // 不单独发出去的话前端看起来就是卡住不动
                if let Some(r) = delta["reasoning_content"].as_str() {
                    if !r.is_empty() {
                        let _ = app.emit(ev.reasoning, r);
                    }
                }
                if let Some(c) = delta["content"].as_str() {
                    if !c.is_empty() {
                        let _ = app.emit(ev.chunk, c);
                    }
                }
            }
        }
    }

    let _ = app.emit(ev.done, ());
}
