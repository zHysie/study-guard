# 学习守门员

Windows 桌面专注工具。基于 Tauri 2 (Rust + TypeScript)，通过浏览器扩展获取当前标签页 URL，检测分心网站并弹出提醒，帮助保持学习专注。

## 功能

- **URL 分类**：B 站视频/UP 主白名单视为学习；黑名单域名及其子域名视为分心；其他网站为等待。
- **两级提醒**：分心持续 N 秒后显示顶部横幅（一级）；分心或空闲达到阈值后全屏覆盖（二级，可选声音/TTS）。
- **一级提醒延迟可配置**：默认 5 秒，支持 1–600 秒。
- **系统托盘**：右键菜单支持打开设置、暂停/恢复监控、退出。监控中/暂停时托盘图标不同。
- **空闲检测**：使用 Win32 `GetLastInputInfo`，不记录键盘输入，不使用全局钩子。
- **浏览器扩展**：Chrome/Edge 扩展通过 Native Messaging 将当前标签页 URL 发送给桌面端。扩展 ID 固定，桌面端启动时自动注册 NM Host，无需手动配置。
- **Mock URL**：配置界面可手动输入 URL 调试分类和提醒策略。
- **单实例**：重复启动时自动聚焦已有窗口。
- **纯 Rust 音频**：WAV 播放使用 rodio，TTS 朗读使用 SAPI，不依赖 PowerShell。
- **窗口可调整大小**：兼容不同分辨率/DPI 的显示器。

## 演示
![](docs\images\image.png)


## 环境要求

- Windows 10 / 11
- [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)（Windows 11 自带）
- Node.js（推荐 20+）
- Rust stable 工具链

安装 Rust：

```powershell
winget install Rustlang.Rustup
rustup default stable
```

## 开发

```powershell
# 安装前端依赖
npm install

# 启动开发模式
npm run tauri:dev

# 运行 Rust 测试
cargo test --manifest-path src-tauri/Cargo.toml
```

## 构建

```powershell
# 构建 NSIS 安装包
npx tauri build --bundles nsis
```

安装包输出路径：

```
src-tauri\target\release\bundle\nsis\学习守门员_0.1.0_x64-setup.exe
```

> `npx tauri build` 会自动编译前端和 Rust 后端，并打包 NSIS 安装程序。Native Messaging Host 功能已内置于主程序中（通过 `--native-messaging-host` 参数调用）。

## 接入 Chrome / Edge 当前 URL

1. 打开扩展管理页：
   - Chrome: `chrome://extensions`
   - Edge: `edge://extensions`
2. 开启"开发者模式"。
3. 点击"加载已解压的扩展"，选择项目的 `browser-extension` 文件夹。
4. 启动学习守门员，首次启动时自动注册 Native Messaging Host（写入注册表，无需管理员权限）。
5. 重启浏览器，切换标签页后状态面板即显示当前 URL。

> 扩展 ID 已通过 manifest 中的 `key` 固定为 `ehmcjemdmobgpaljappopgnmmlkjjcgo`，Native Messaging 注册指向主程序 `study_guardian.exe --native-messaging-host`。

## 使用 Mock URL 调试

1. 在"Mock 当前 URL"输入框中输入 URL，点击"应用 URL"。
2. 状态面板会实时显示分类结果、当前提醒、空闲时间和分心持续时间。
3. 点击"清空"恢复使用浏览器扩展的 URL。

Mock URL 优先级高于浏览器 URL。

示例：

| URL | 分类 | 说明 |
|---|---|---|
| `https://www.bilibili.com/video/BVxxxxxx` | 学习中 / 分心 | 取决于是否在白名单 |
| `https://github.com/tauri-apps/tauri` | 等待 | 非黑名单非 B 站 |
| `https://www.xiaohongshu.com/explore` | 分心 | 命中黑名单 |

## 配置文件

配置文件位于应用数据目录下的 `config.json`。字段使用 camelCase：

```json
{
  "videoWhitelist": [],
  "upWhitelist": [],
  "domainBlacklist": ["xiaohongshu.com", "douyin.com"],
  "idleMinutes": 1,
  "overlayDistractingMinutes": 5,
  "bannerDelaySeconds": 5,
  "checkIntervalSeconds": 2,
  "bannerText": "快去学习",
  "overlayText": "别刷了，回到教程",
  "overlayImagePath": "",
  "overlaySoundEnabled": true,
  "overlaySoundPath": "",
  "overlayVoiceText": "快点学习！",
  "overlaySoundBurstSeconds": 30,
  "overlaySoundPauseMinutes": 3
}
```

| 字段 | 范围 | 说明 |
|---|---|---|
| `bannerDelaySeconds` | 1–600 秒 | 一级提醒延迟，默认 5 秒 |
| `idleMinutes` | 1–60 分钟 | 空闲阈值 |
| `overlayDistractingMinutes` | 1–60 分钟 | 二级分心阈值 |
| `overlaySoundBurstSeconds` | 1–600 秒 | 二级提醒声音持续时长 |
| `overlaySoundPauseMinutes` | 1–60 分钟 | 声音暂停间隔 |
| `checkIntervalSeconds` | 固定 2 | 监控检查间隔，不可修改 |

## 架构

| 模块 | 职责 |
|---|---|
| `classifier.rs` | 纯 URL 分类，不依赖 Tauri 窗口 |
| `reminder_policy.rs` | 提醒策略状态机，不依赖 Tauri 窗口 |
| `idle.rs` | Win32 空闲检测封装 |
| `url_provider.rs` | URL 来源抽象（Mock + Native Messaging） |
| `monitor.rs` | 串联 URL、空闲、分类、策略，状态变化时推送事件 |
| `windows.rs` | 显示/隐藏预声明的提醒窗口 |
| `native_messaging.rs` | 处理浏览器 Native Messaging 协议 |
| `native_host.rs` | 首次启动时自动注册 NM Host（生成 manifest + 注册表） |
| `tray.rs` | 系统托盘菜单及图标切换 |
| `audio.rs` | TTS 朗读（SAPI）/ WAV 音效播放（rodio） |
| `config.rs` | 配置的加载、保存、校验 |

## 许可证

GNU General Public License v3.0 — 详见 [LICENSE](LICENSE)。
