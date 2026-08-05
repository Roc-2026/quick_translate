<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface Msg {
  role: "user" | "assistant";
  /** 真正发给 API 的内容：user 这一侧会把引用原文一起包进去 */
  content: string;
  /** 界面上显示的问题本体（不含引用块） */
  display?: string;
  /** 该轮引用的原文，界面上单独渲染成一条 */
  quote?: string;
  /** 思考模式下的推理过程，默认折叠 */
  reasoning?: string;
}

const messages = ref<Msg[]>([]);
const input = ref("");
/** 待附带的引用原文。唤起时由 Rust 取词填入，也可以点 ✕ 去掉 */
const quote = ref("");
const model = ref("");
const loading = ref(false);
const errorMsg = ref("");
const copiedIdx = ref(-1);
const openReasoning = ref<number[]>([]);

const bodyEl = ref<HTMLElement | null>(null);
const inputEl = ref<HTMLTextAreaElement | null>(null);

const unlisteners: UnlistenFn[] = [];

async function hide() {
  await invoke("hide_window");
}

async function scrollToBottom() {
  await nextTick();
  if (bodyEl.value) bodyEl.value.scrollTop = bodyEl.value.scrollHeight;
}

function focusInput() {
  nextTick(() => inputEl.value?.focus());
}

/** 取最后一条助手消息，流式分片都往它上面追加 */
function tail(): Msg | null {
  const last = messages.value[messages.value.length - 1];
  return last && last.role === "assistant" ? last : null;
}

async function send() {
  const q = input.value.trim();
  if (!q || loading.value) return;

  const quoted = quote.value.trim();
  messages.value.push({
    role: "user",
    content: quoted ? `【引用原文】\n${quoted}\n\n【我的问题】\n${q}` : q,
    display: q,
    quote: quoted || undefined,
  });
  input.value = "";
  quote.value = "";
  errorMsg.value = "";

  // 历史要在插入空的助手占位之前取，否则会把空消息也发上去
  const payload = messages.value.map((m) => ({ role: m.role, content: m.content }));
  messages.value.push({ role: "assistant", content: "", reasoning: "" });
  loading.value = true;
  await scrollToBottom();

  try {
    await invoke("ask_send", { messages: payload });
  } catch (e) {
    loading.value = false;
    errorMsg.value = String(e);
  }
}

function newChat() {
  messages.value = [];
  openReasoning.value = [];
  errorMsg.value = "";
  loading.value = false;
  focusInput();
}

async function copyAnswer(i: number) {
  const text = messages.value[i]?.content;
  if (!text) return;
  await invoke("copy_text", { text });
  copiedIdx.value = i;
  setTimeout(() => (copiedIdx.value = -1), 1200);
}

function toggleReasoning(i: number) {
  const at = openReasoning.value.indexOf(i);
  if (at >= 0) openReasoning.value.splice(at, 1);
  else openReasoning.value.push(i);
}

/**
 * Enter 发送、Shift+Enter 换行。
 *
 * `isComposing` 必须判：中文输入法用 Enter 上屏候选词，不拦的话每次选词都会误发。
 */
function onInputKey(e: KeyboardEvent) {
  if (e.key !== "Enter" || e.shiftKey || e.isComposing) return;
  e.preventDefault();
  send();
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") hide();
}

/**
 * 拦掉双击标题栏的「缩放窗口」。
 *
 * Tauri 的 drag region 脚本在 macOS 上是在 document 的 mouseup 里触发 zoom 的
 * （Windows/Linux 走 mousedown），双击把这个浮窗放大成一整块很怪。
 */
function swallowTitleBarZoom(e: MouseEvent) {
  if (e.detail >= 2) e.stopPropagation();
}

onMounted(async () => {
  window.addEventListener("keydown", onKey);

  unlisteners.push(
    await listen<{ quote: string; model?: string }>("ask://open", (e) => {
      // 只覆盖引用条，不动已有对话 —— 追问时上下文得留着
      quote.value = e.payload.quote ?? "";
      if (e.payload.model) model.value = e.payload.model;
      focusInput();
    })
  );

  unlisteners.push(
    await listen<string>("ask://chunk", async (e) => {
      const m = tail();
      if (!m) return;
      m.content += e.payload;
      await scrollToBottom();
    })
  );

  unlisteners.push(
    await listen<string>("ask://reasoning", async (e) => {
      const m = tail();
      if (!m) return;
      m.reasoning = (m.reasoning ?? "") + e.payload;
      await scrollToBottom();
    })
  );

  unlisteners.push(
    await listen("ask://done", () => {
      loading.value = false;
    })
  );

  unlisteners.push(
    await listen<string>("ask://error", (e) => {
      loading.value = false;
      errorMsg.value = e.payload;
    })
  );

  focusInput();
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKey);
  unlisteners.forEach((u) => u());
});
</script>

<template>
  <div class="card">
    <header class="bar" data-tauri-drag-region="deep" @mouseup="swallowTitleBarZoom">
      <span class="title">
        <span class="dot" :class="{ spin: loading }"></span>
        问 DeepSeek
      </span>
      <span class="bar-right">
        <button class="ghost-btn" title="清空上下文，重新开始" @click="newChat">
          新对话
        </button>
        <button class="icon-btn" title="关闭 (Esc)" @click="hide">✕</button>
      </span>
    </header>

    <main ref="bodyEl" class="body">
      <p v-if="!messages.length" class="hint">
        选中文本后按快捷键唤起，选中的内容会自动带进来当上下文。<br />
        Enter 发送，Shift+Enter 换行，Esc 关闭。
      </p>

      <div v-for="(m, i) in messages" :key="i" class="msg" :class="m.role">
        <template v-if="m.role === 'user'">
          <div v-if="m.quote" class="quote-in-msg">{{ m.quote }}</div>
          <div class="bubble">{{ m.display ?? m.content }}</div>
        </template>

        <template v-else>
          <div v-if="m.reasoning" class="reasoning">
            <button class="reasoning-head" @click="toggleReasoning(i)">
              {{ openReasoning.includes(i) ? "▾" : "▸" }} 思考过程
            </button>
            <div v-if="openReasoning.includes(i)" class="reasoning-body">
              {{ m.reasoning }}
            </div>
          </div>
          <div class="answer">
            {{ m.content
            }}<span v-if="loading && i === messages.length - 1" class="caret"></span>
          </div>
          <button
            v-if="m.content"
            class="copy-inline"
            @click="copyAnswer(i)"
          >
            {{ copiedIdx === i ? "已复制" : "复制" }}
          </button>
        </template>
      </div>

      <p v-if="errorMsg" class="error">{{ errorMsg }}</p>
    </main>

    <footer class="foot">
      <div v-if="quote" class="quote-chip">
        <span class="quote-label">引用</span>
        <span class="quote-text">{{ quote }}</span>
        <button class="icon-btn tiny" title="不带这段原文" @click="quote = ''">✕</button>
      </div>

      <div class="composer">
        <textarea
          ref="inputEl"
          v-model="input"
          class="input"
          rows="2"
          placeholder="问点什么…"
          @keydown="onInputKey"
        ></textarea>
        <button class="send-btn" :disabled="!input.trim() || loading" @click="send">
          {{ loading ? "回答中" : "发送" }}
        </button>
      </div>

      <div class="meta">
        <span class="model">{{ model }}</span>
        <span class="tip">Enter 发送 · Shift+Enter 换行</span>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.card {
  height: 100%;
  margin: 10px;
  display: flex;
  flex-direction: column;
  border-radius: 16px;
  background: rgba(24, 24, 28, 0.92);
  backdrop-filter: blur(18px) saturate(160%);
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
  overflow: hidden;
  color: #eceef2;
}

.bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  cursor: default;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.04), transparent);
}

.title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.02em;
}

.bar-right {
  display: flex;
  align-items: center;
  gap: 6px;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #6b7280;
}
.dot.spin {
  background: conic-gradient(#6366f1, #38bdf8, #6366f1);
  animation: rot 0.9s linear infinite;
}
@keyframes rot {
  to {
    transform: rotate(360deg);
  }
}

.icon-btn {
  border: none;
  background: transparent;
  color: #9aa0aa;
  font-size: 14px;
  cursor: pointer;
  width: 24px;
  height: 24px;
  border-radius: 6px;
  transition: background 0.15s, color 0.15s;
}
.icon-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
}
.icon-btn.tiny {
  width: 18px;
  height: 18px;
  font-size: 11px;
  flex: none;
}

.ghost-btn {
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: transparent;
  color: #a8adb8;
  font-size: 11px;
  padding: 3px 9px;
  border-radius: 7px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.ghost-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
}

.body {
  flex: 1;
  overflow-y: auto;
  padding: 4px 12px 12px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.hint {
  color: #6b7280;
  font-size: 12px;
  line-height: 1.8;
  margin: auto 0;
}

.msg {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.msg.user {
  align-items: flex-end;
}

.bubble {
  max-width: 88%;
  padding: 8px 12px;
  border-radius: 12px 12px 3px 12px;
  background: linear-gradient(135deg, #4f46e5, #2f7fd4);
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  user-select: text;
}

.quote-in-msg {
  max-width: 88%;
  padding: 6px 10px;
  border-left: 2px solid rgba(99, 102, 241, 0.7);
  background: rgba(255, 255, 255, 0.04);
  border-radius: 0 8px 8px 0;
  font-size: 11px;
  line-height: 1.6;
  color: #a8adb8;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 84px;
  overflow-y: auto;
}

.answer {
  font-size: 14px;
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-word;
  user-select: text;
}

.reasoning {
  border-left: 2px solid rgba(255, 255, 255, 0.12);
  padding-left: 8px;
}
.reasoning-head {
  border: none;
  background: transparent;
  color: #6b7280;
  font-size: 11px;
  cursor: pointer;
  padding: 0;
}
.reasoning-body {
  margin-top: 4px;
  font-size: 11px;
  line-height: 1.7;
  color: #8b8f99;
  white-space: pre-wrap;
  word-break: break-word;
}

.copy-inline {
  align-self: flex-start;
  border: none;
  background: transparent;
  color: #6b7280;
  font-size: 11px;
  cursor: pointer;
  padding: 0;
}
.copy-inline:hover {
  color: #a8adb8;
}

.caret {
  display: inline-block;
  width: 7px;
  height: 15px;
  margin-left: 2px;
  vertical-align: text-bottom;
  background: #6366f1;
  border-radius: 1px;
  animation: blink 1s steps(2) infinite;
}
@keyframes blink {
  50% {
    opacity: 0;
  }
}

.error {
  color: #f87171;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
}

.foot {
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  padding: 8px 12px 10px;
}

.quote-chip {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
  padding: 5px 8px;
  border-radius: 8px;
  background: rgba(99, 102, 241, 0.12);
  border: 1px solid rgba(99, 102, 241, 0.25);
  font-size: 11px;
  color: #a8adb8;
}
.quote-label {
  flex: none;
  color: #8b8fff;
}
.quote-text {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.composer {
  display: flex;
  gap: 8px;
  align-items: flex-end;
}

.input {
  flex: 1;
  resize: none;
  max-height: 120px;
  padding: 8px 10px;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.05);
  color: #eceef2;
  font-size: 13px;
  line-height: 1.6;
  font-family: inherit;
  outline: none;
}
.input:focus {
  border-color: rgba(99, 102, 241, 0.6);
}

.send-btn {
  flex: none;
  border: none;
  border-radius: 10px;
  padding: 9px 16px;
  font-size: 12px;
  font-weight: 600;
  color: #fff;
  background: linear-gradient(135deg, #6366f1, #38bdf8);
  cursor: pointer;
  transition: opacity 0.15s, transform 0.1s;
}
.send-btn:hover:not(:disabled) {
  transform: translateY(-1px);
}
.send-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.meta {
  display: flex;
  justify-content: space-between;
  margin-top: 6px;
  font-size: 10px;
  color: #6b7280;
}
.model {
  font-family: ui-monospace, "Cascadia Code", monospace;
}

.body::-webkit-scrollbar,
.quote-in-msg::-webkit-scrollbar,
.input::-webkit-scrollbar {
  width: 6px;
}
.body::-webkit-scrollbar-thumb,
.quote-in-msg::-webkit-scrollbar-thumb,
.input::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.14);
  border-radius: 3px;
}
</style>
