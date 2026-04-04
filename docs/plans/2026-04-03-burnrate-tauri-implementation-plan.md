# BurnRate Tauri Implementation Plan

> 当前仓库已完成首轮落地，后续迭代按以下方向推进。

## 已完成

1. 初始化 Tauri 2 + React + Rust 骨架
2. 将默认窗口改造成 `popover + settings`
3. 接入 tray 与窗口定位
4. 实现三家 provider 的首版适配器
5. 实现 SQLite 快照与 7/30 天汇总
6. 实现 React popover 和设置页
7. 打通 `npm test`、`cargo test`、`npm run tauri build -- --debug`

## 下一步

1. 用真实套餐密钥做端到端联调
2. 为 provider fetch 增加更细的错误分类
3. 优化 Kimi 的可配置接口说明
4. 增加更细的 UI 组件测试
5. 评估签名、公证和自动更新链路

## 验收命令

```bash
npm test
. ./.cargo/env && cd src-tauri && cargo test
. ./.cargo/env && npm run tauri build -- --debug
```
