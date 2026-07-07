import { createApp } from "vue";
import "./styles.css";
import { getCurrentWindow } from "@tauri-apps/api/window";

// 同一份前端服务两个窗口：按窗口 label 决定渲染浮窗还是设置界面
const label = getCurrentWindow().label;

if (label === "settings") {
  import("./Settings.vue").then((m) => createApp(m.default).mount("#app"));
} else {
  import("./App.vue").then((m) => createApp(m.default).mount("#app"));
}
