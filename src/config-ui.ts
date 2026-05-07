import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AppConfig,
  classificationText,
  MonitorStatus,
  reminderText,
  secondsText,
} from "./status";

let currentConfig: AppConfig | null = null;
let saveTimer: number | undefined;

export async function renderConfigUi(root: HTMLElement): Promise<void> {
  root.innerHTML = `
    <main class="app-shell">
      <header class="titlebar">
        <div class="titlebar-brand">
          <h1>Study Guard</h1>
        </div>
        <nav class="tabbar" aria-label="主界面导航">
          ${tabButton("status", "状态", true)}
          ${tabButton("lists", "名单")}
          ${tabButton("rules", "规则")}
        </nav>
        <div class="window-controls">
          <button class="window-control minimize" id="windowMinimize" type="button" aria-label="最小化">${icon("remove")}</button>
          <button class="window-control maximize" id="windowMaximize" type="button" aria-label="最大化">${icon("crop_square")}</button>
          <button class="window-control close" id="windowClose" type="button" aria-label="关闭">${icon("close")}</button>
        </div>
      </header>

      <section class="workspace">
        <section class="tab-panel is-active" id="tab-status" data-tab-panel="status">
          <div class="panel status-tool">
            <div class="panel-header">
              <h2>状态监控</h2>
              <div class="monitor-actions">
                <button id="applyMockUrl" class="primary small">${icon("play_arrow")}应用 URL</button>
                <button id="clearMockUrl" class="small">${icon("backspace")}清空</button>
                <button id="togglePaused" class="danger small">${icon("pause")}暂停监控</button>
              </div>
            </div>

            <label class="field mock-field">
              <span>Mock 当前 URL</span>
              <input id="mockUrl" type="url" placeholder="https://www.bilibili.com/video/..." />
            </label>

            <div class="startup-error" id="startupError" hidden></div>

            <section class="status-panel">
              ${statusItem("当前 URL", "statusUrlInline", "无")}
              ${statusItem("分类", "statusClassification", "等待")}
              ${statusItem("当前提醒", "statusReminder", "无提醒")}
              ${statusItem("空闲时间", "statusIdle", "00:00:00")}
              ${statusItem("分心持续", "statusDistracting", "00:00:00")}
              ${statusItem("URL 来源", "statusUrlSource", "none")}
            </section>
          </div>
        </section>

        <section class="tab-panel" id="tab-lists" data-tab-panel="lists" hidden>
          <div class="panel list-panel">
            <div class="panel-header">
              <h2>名单管理</h2>
            </div>
            <div class="list-grid">
              ${textareaField("videoWhitelist", "视频白名单", "每行一个完整 URL、BV 号或关键片段")}
              ${textareaField("upWhitelist", "UP 主白名单", "每行一个 UP 主空间 URL、UID")}
              ${textareaField("domainBlacklist", "黑名单域名", "每行一个域名，子域名也会命中")}
            </div>
          </div>
        </section>

        <section class="tab-panel" id="tab-rules" data-tab-panel="rules" hidden>
          <div class="panel rules-panel">
            <div class="panel-header">
              <h2>提醒规则设置</h2>
            </div>
            <div class="rule-box" id="ruleText"></div>

            <div class="rules-layout">
              <section class="rule-group">
                <h3>浏览器扩展</h3>
                ${textField("extensionId", "扩展 ID")}

                <h3>时间阈值</h3>
                ${numberField("bannerDelaySeconds", "一级提醒延迟", "秒", 1, 600)}
                <div class="two-col">
                  ${numberField("overlayDistractingMinutes", "二级分心阈值", "分钟", 1, 60)}
                  ${numberField("idleMinutes", "空闲阈值", "分钟", 1, 60)}
                </div>

                <h3>声音控制</h3>
                <label class="check-row">
                  <span>开启二级提醒声音/朗读</span>
                  <input id="overlaySoundEnabled" type="checkbox" />
                </label>
                ${textField("overlayVoiceText", "默认朗读文案")}
                <div class="two-col">
                  ${numberField("overlaySoundBurstSeconds", "连续朗读时间", "秒", 1, 600)}
                  ${numberField("overlaySoundPauseMinutes", "朗读暂停时间", "分钟", 1, 60)}
                </div>
                ${pathField("overlaySoundPath", "二级提醒音效路径")}
              </section>

              <section class="rule-group">
                <h3>文案配置</h3>
                ${textField("bannerText", "一级提醒文案")}
                ${textField("overlayText", "二级提醒文案")}
                ${pathField("overlayImagePath", "二级提醒图片路径")}
              </section>
            </div>
          </div>
        </section>
      </section>

      <footer class="action-bar">
        <div class="footer-tests">
          <button id="testBanner">${icon("notifications")}测试一级提醒</button>
          <button id="testOverlay">${icon("campaign")}测试二级提醒</button>
          <button id="closeOverlay" class="muted">${icon("layers_clear")}关闭测试覆盖层</button>
        </div>
        <div class="footer-save">
          <span id="saveHint" class="save-hint"></span>
          <button id="saveConfig" class="primary">${icon("save")}保存配置</button>
        </div>
      </footer>
    </main>
  `;

  setupTitlebar();
  disableFindShortcut();
  setupTabs();

  try {
    currentConfig = await invoke<AppConfig>("get_config");
    fillForm(currentConfig);
    bindEvents();
    updateRuleText();
    updateStatus(await invoke<MonitorStatus>("get_status"));

    await listen<MonitorStatus>("monitor-status-changed", (event) => {
      updateStatus(event.payload);
    });
  } catch (error) {
    showStartupError(error);
  }
}

function tabButton(name: string, label: string, active = false): string {
  return `
    <button
      class="tab-button${active ? " is-active" : ""}"
      type="button"
      data-tab="${name}"
      aria-controls="tab-${name}"
      aria-selected="${active}"
    >${label}</button>
  `;
}

function statusItem(label: string, id: string, fallback: string): string {
  return `
    <div>
      <span>${label}</span>
      <strong id="${id}">${fallback}</strong>
    </div>
  `;
}

function icon(name: string): string {
  return `<span class="material-symbols-outlined" aria-hidden="true">${name}</span>`;
}

function disableFindShortcut(): void {
  window.addEventListener(
    "keydown",
    (event) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
        event.preventDefault();
        event.stopPropagation();
      }
    },
    { capture: true },
  );
}

function setupTitlebar(): void {
  const titlebar = document.querySelector<HTMLElement>(".titlebar");
  if (!titlebar) {
    return;
  }

  const appWindow = getCurrentWindow();

  document.getElementById("windowMinimize")?.addEventListener("click", () => {
    void appWindow.minimize().catch((error) => console.error("minimize failed", error));
  });
  document.getElementById("windowMaximize")?.addEventListener("click", () => {
    void appWindow
      .toggleMaximize()
      .catch((error) => console.error("toggleMaximize failed", error));
  });
  document.getElementById("windowClose")?.addEventListener("click", () => {
    void appWindow.close().catch((error) => console.error("close failed", error));
  });

  titlebar.addEventListener("mousedown", (event) => {
    if (
      event.button !== 0 ||
      event.detail > 1 ||
      (event.target as HTMLElement).closest(".window-controls, .tabbar")
    ) {
      return;
    }
    void appWindow
      .startDragging()
      .catch((error) => console.error("startDragging failed", error));
  });

  titlebar.addEventListener("dblclick", (event) => {
    if (!(event.target as HTMLElement).closest(".window-controls, .tabbar")) {
      void appWindow
        .toggleMaximize()
        .catch((error) => console.error("toggleMaximize failed", error));
    }
  });
}

function setupTabs(): void {
  const buttons = Array.from(document.querySelectorAll<HTMLButtonElement>("[data-tab]"));
  const panels = Array.from(document.querySelectorAll<HTMLElement>("[data-tab-panel]"));

  for (const button of buttons) {
    button.addEventListener("click", () => {
      const target = button.dataset.tab;
      for (const item of buttons) {
        const active = item === button;
        item.classList.toggle("is-active", active);
        item.setAttribute("aria-selected", String(active));
      }
      for (const panel of panels) {
        const active = panel.dataset.tabPanel === target;
        panel.hidden = !active;
        panel.classList.toggle("is-active", active);
      }
    });
  }
}

function textareaField(id: keyof AppConfig, label: string, help: string): string {
  return `
    <label class="field">
      <span>${label}</span>
      <textarea id="${id}" rows="6" placeholder="${help}"></textarea>
    </label>
  `;
}

function textField(id: keyof AppConfig, label: string): string {
  return `
    <label class="field">
      <span>${label}</span>
      <input id="${id}" type="text" />
    </label>
  `;
}

function pathField(id: keyof AppConfig, label: string): string {
  return `
    <label class="field path-field">
      <span>${label}</span>
      <span class="path-control">
        <input id="${id}" type="text" />
        <button class="path-button" type="button" data-path-target="${id}" aria-label="选择${label}">${icon("folder_open")}</button>
      </span>
    </label>
  `;
}

function numberField(
  id: keyof AppConfig,
  label: string,
  suffix: string,
  min: number,
  max: number,
): string {
  return `
    <label class="field">
      <span>${label}</span>
      <div class="number-wrap">
        <input id="${id}" type="number" min="${min}" max="${max}" step="1" />
        <em>${suffix}</em>
      </div>
    </label>
  `;
}

function bindEvents(): void {
  byId("saveConfig").addEventListener("click", saveConfig);
  byId("testBanner").addEventListener("click", () => invoke("test_banner"));
  byId("testOverlay").addEventListener("click", () => invoke("test_overlay"));
  byId("closeOverlay").addEventListener("click", () => invoke("close_overlay_for_test"));
  byId("applyMockUrl").addEventListener("click", applyMockUrl);
  byId("clearMockUrl").addEventListener("click", async () => {
    (byId("mockUrl") as HTMLInputElement).value = "";
    updateStatus(await invoke<MonitorStatus>("set_mock_url", { url: null }));
  });
  byId("togglePaused").addEventListener("click", togglePaused);
  for (const button of document.querySelectorAll<HTMLButtonElement>("[data-path-target]")) {
    button.addEventListener("click", async () => {
      await choosePath(button.dataset.pathTarget || "");
    });
  }

  for (const id of ["idleMinutes", "overlayDistractingMinutes", "bannerDelaySeconds"]) {
    byId(id).addEventListener("input", updateRuleText);
  }
}

async function choosePath(targetId: string): Promise<void> {
  if (targetId !== "overlayImagePath" && targetId !== "overlaySoundPath") {
    return;
  }

  const input = byId(targetId) as HTMLInputElement;
  const selected = await open({
    title: targetId === "overlayImagePath" ? "选择二级提醒图片" : "选择二级提醒音效",
    multiple: false,
    directory: false,
    defaultPath: input.value || undefined,
    filters:
      targetId === "overlayImagePath"
        ? [{ name: "图片文件", extensions: ["png", "jpg", "jpeg", "webp", "bmp", "gif"] }]
        : [{ name: "音频文件", extensions: ["wav", "mp3", "ogg", "flac", "m4a", "aac"] }],
  });

  if (typeof selected === "string") {
    input.value = selected;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }
}

function fillForm(config: AppConfig): void {
  setInput("extensionId", config.extensionId);
  setTextArea("videoWhitelist", config.videoWhitelist);
  setTextArea("upWhitelist", config.upWhitelist);
  setTextArea("domainBlacklist", config.domainBlacklist);
  setInput("idleMinutes", String(config.idleMinutes));
  setInput("overlayDistractingMinutes", String(config.overlayDistractingMinutes));
  setInput("bannerDelaySeconds", String(config.bannerDelaySeconds));
  setInput("bannerText", config.bannerText);
  setInput("overlayText", config.overlayText);
  setInput("overlayImagePath", config.overlayImagePath);
  setChecked("overlaySoundEnabled", config.overlaySoundEnabled);
  setInput("overlaySoundPath", config.overlaySoundPath);
  setInput("overlayVoiceText", config.overlayVoiceText);
  setInput("overlaySoundBurstSeconds", String(config.overlaySoundBurstSeconds));
  setInput("overlaySoundPauseMinutes", String(config.overlaySoundPauseMinutes));
}

async function saveConfig(): Promise<void> {
  if (!currentConfig) {
    return;
  }

  const next: AppConfig = {
    ...currentConfig,
    extensionId: readValue("extensionId"),
    videoWhitelist: readLines("videoWhitelist"),
    upWhitelist: readLines("upWhitelist"),
    domainBlacklist: readLines("domainBlacklist"),
    idleMinutes: readNumber("idleMinutes"),
    overlayDistractingMinutes: readNumber("overlayDistractingMinutes"),
    bannerDelaySeconds: readNumber("bannerDelaySeconds"),
    bannerText: readValue("bannerText"),
    overlayText: readValue("overlayText"),
    overlayImagePath: readValue("overlayImagePath"),
    overlaySoundEnabled: (byId("overlaySoundEnabled") as HTMLInputElement).checked,
    overlaySoundPath: readValue("overlaySoundPath"),
    overlayVoiceText: readValue("overlayVoiceText"),
    overlaySoundBurstSeconds: readNumber("overlaySoundBurstSeconds"),
    overlaySoundPauseMinutes: readNumber("overlaySoundPauseMinutes"),
  };

  currentConfig = await invoke<AppConfig>("save_config", { config: next });
  fillForm(currentConfig);
  updateRuleText();
  showSavedHint();
}

async function applyMockUrl(): Promise<void> {
  const value = readValue("mockUrl");
  const status = await invoke<MonitorStatus>("set_mock_url", {
    url: value ? value : null,
  });
  updateStatus(status);
}

async function togglePaused(): Promise<void> {
  const paused = byId("togglePaused").dataset.paused !== "true";
  const status = await invoke<MonitorStatus>("set_paused", { paused });
  updateStatus(status);
}

function updateRuleText(): void {
  const idle = readNumber("idleMinutes");
  const distracting = readNumber("overlayDistractingMinutes");
  const banner = readNumber("bannerDelaySeconds");
  byId("ruleText").innerHTML = `
    <p>一级提醒：当前 URL 命中黑名单域名或 B 站非白名单内容，并持续 ${banner} 秒后出现。</p>
    <p>二级提醒：分心内容持续 ${distracting} 分钟，或鼠标键盘空闲达到 ${idle} 分钟后出现。</p>
  `;
}

function updateStatus(status: MonitorStatus): void {
  byId("statusUrlInline").textContent = status.currentUrl || "无";
  byId("statusClassification").textContent = classificationText[status.classification];
  byId("statusReminder").textContent = reminderText[status.activeReminder];
  byId("statusIdle").textContent = secondsText(status.idleSeconds);
  byId("statusDistracting").textContent = secondsText(status.distractingSeconds);
  byId("statusUrlSource").textContent = status.urlSource;

  const pauseButton = byId("togglePaused");
  pauseButton.innerHTML = `${icon(status.paused ? "play_arrow" : "pause")}${status.paused ? "恢复监控" : "暂停监控"}`;
  pauseButton.dataset.paused = String(status.paused);
}

function showStartupError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  const errorBox = byId("startupError");
  errorBox.hidden = false;
  errorBox.innerHTML = `
    <p>后端命令调用失败：${escapeHtml(message)}</p>
    <p>请关闭应用后重新启动；如果仍然出现，请保留这个错误信息。</p>
  `;
  byId("ruleText").innerHTML = `
    <p class="error-text">后端命令调用失败：${escapeHtml(message)}</p>
    <p class="error-text">请关闭应用后重新启动；如果仍然出现，请保留这个错误信息。</p>
  `;
}

function showSavedHint(): void {
  const hint = byId("saveHint");
  hint.textContent = `最后保存于 ${new Date().toLocaleTimeString("zh-CN", {
    hour12: false,
  })}`;
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    hint.textContent = "";
  }, 3500);
}

function setTextArea(id: string, values: string[]): void {
  (byId(id) as HTMLTextAreaElement).value = values.join("\n");
}

function setInput(id: string, value: string): void {
  const element = byId(id) as HTMLInputElement | HTMLTextAreaElement;
  element.value = value;
}

function setChecked(id: string, value: boolean): void {
  (byId(id) as HTMLInputElement).checked = value;
}

function readLines(id: string): string[] {
  return (byId(id) as HTMLTextAreaElement).value
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

function readValue(id: string): string {
  const element = byId(id) as HTMLInputElement | HTMLTextAreaElement;
  return element.value.trim();
}

function readNumber(id: string): number {
  const value = Number((byId(id) as HTMLInputElement).value);
  return Number.isFinite(value) ? value : 0;
}

function byId(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (!element) {
    throw new Error(`Missing element #${id}`);
  }
  return element;
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
