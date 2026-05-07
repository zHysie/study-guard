import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { renderConfigUi } from "./config-ui";
import { AppConfig } from "./status";
import "./styles.css";

const root = document.getElementById("app");
if (!root) {
  throw new Error("Missing #app root");
}

const view = new URLSearchParams(window.location.search).get("window") || "main";

if (view === "banner") {
  renderBanner(root);
} else if (view === "overlay") {
  renderOverlay(root);
} else {
  document.documentElement.className = "";
  document.body.className = "";
  root.className = "";
  renderConfigUi(root);
}

function renderBanner(rootElement: HTMLElement): void {
  document.documentElement.className = "banner-html";
  document.body.className = "banner-body";
  rootElement.className = "banner-root";
  rootElement.innerHTML = `<div class="banner-reminder" id="bannerText">快去学习</div>`;
  listen<AppConfig>("reminder-config", (event) => {
    const text = event.payload.bannerText || "快去学习";
    document.getElementById("bannerText")!.textContent = text;
  });
}

function renderOverlay(rootElement: HTMLElement): void {
  document.title = "";
  document.documentElement.className = "overlay-html";
  document.body.className = "overlay-body";
  rootElement.className = "overlay-root";
  rootElement.innerHTML = `
    <div class="overlay-reminder" id="overlaySurface">
      <div class="overlay-background" id="overlayBackground"></div>
      <div class="watermark-grid" id="watermarkGrid"></div>
    </div>
  `;
  setOverlayText("别刷了，回到教程");

  listen<AppConfig>("reminder-config", (event) => {
    setOverlayText(event.payload.overlayText || "别刷了，回到教程");
    const background = document.getElementById("overlayBackground") as HTMLElement;
    const imagePath = event.payload.overlayImagePath.trim();
    if (imagePath) {
      background.style.backgroundImage = `url("${convertFileSrc(imagePath)}")`;
    } else {
      background.style.backgroundImage = "";
    }
  });
}

function setOverlayText(text: string): void {
  const grid = document.getElementById("watermarkGrid");
  if (!grid) {
    return;
  }
  grid.innerHTML = Array.from({ length: 28 }, () => `<span>${escapeHtml(text)}</span>`).join("");
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
