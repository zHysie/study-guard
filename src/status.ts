export type Classification = "studying" | "distracting" | "waiting";
export type ActiveReminder = "none" | "banner" | "overlay";

export interface MonitorStatus {
  classification: Classification;
  currentUrl: string | null;
  activeReminder: ActiveReminder;
  idleSeconds: number;
  distractingSeconds: number;
  paused: boolean;
  urlSource: string;
}

export interface AppConfig {
  extensionId: string;
  videoWhitelist: string[];
  upWhitelist: string[];
  domainBlacklist: string[];
  idleMinutes: number;
  overlayDistractingMinutes: number;
  bannerDelaySeconds: number;
  checkIntervalSeconds: number;
  bannerText: string;
  overlayText: string;
  overlayImagePath: string;
  overlaySoundEnabled: boolean;
  overlaySoundPath: string;
  overlayVoiceText: string;
}

export const classificationText: Record<Classification, string> = {
  studying: "学习中",
  distracting: "分心",
  waiting: "等待",
};

export const reminderText: Record<ActiveReminder, string> = {
  none: "无提醒",
  banner: "一级提醒",
  overlay: "二级提醒",
};

export function secondsText(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = seconds % 60;
  return [hours, minutes, rest].map((part) => String(part).padStart(2, "0")).join(":");
}
