# BurnRate Tauri Technical Solution

## 目标

实现一个常驻 macOS 右上角菜单栏的套餐监控工具，支持：

- Zhipu 当前 5 小时窗口
- MiniMax 当前套餐窗口
- Kimi 余额与本机历史变化

## 技术选型

- 桌面壳：Tauri 2
- 后端：Rust
- 前端：React + TypeScript + Vite
- 本地存储：SQLite
- 自动启动：tauri-plugin-autostart
- 窗口定位：tauri-plugin-positioner

## 架构

- Rust 负责：
  - provider 请求
  - 历史快照落盘
  - 后台定时刷新
  - tray 与窗口控制
  - 设置保存
- React 负责：
  - popover 展示
  - 设置页交互
  - 本地状态管理

## 数据流

1. 用户在设置页录入 API Key 和接口配置
2. Rust 后台按刷新间隔轮询 provider
3. 成功数据写入 SQLite
4. Rust 发出 `dashboard://updated` 事件
5. React 更新 popover 和设置页状态

## 关键模块

- `src-tauri/src/providers/`
- `src-tauri/src/storage/`
- `src-tauri/src/app_state.rs`
- `src-tauri/src/tray.rs`
- `src/store/useBurnRateStore.ts`
- `src/features/dashboard/summary.ts`

## 当前实现边界

- 已支持菜单栏、浮层、设置页、自动刷新、本地统计
- 已支持 debug 构建出 `.app`
- Kimi 当前按余额接口接入，周期配额接口仍保留扩展位
