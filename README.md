# BurnRate

BurnRate 是一个基于 `Tauri 2 + Rust + React` 的 macOS 菜单栏应用，用来监控三类套餐的使用情况：

- 智谱清言（Zhipu）5 小时窗口
- MiniMax 当前套餐窗口
- Kimi 账户余额与本机历史变化

当前版本已经具备：

- 菜单栏 tray 常驻
- 浮层 Popover 摘要界面
- Popover 内直接进入设置表单并录入套餐密钥
- 独立设置窗口
- Rust 侧轮询、落盘、后台自动刷新
- 本地 SQLite 历史统计
- API Key 本机凭证保存
- `tauri build --debug` 可生成 `.app`

## 本地运行

如果你已经全局安装了 Rust，直接跳到第 2 步。

如果要复用当前仓库里的本地 Rust toolchain，先执行：

```bash
. ./.cargo/env
```

然后安装前端依赖并启动：

```bash
npm install
npm run tauri dev
```

## 构建

```bash
. ./.cargo/env
npm run tauri build -- --debug
```

调试构建产物会输出到：

```text
src-tauri/target/debug/bundle/macos/BurnRate.app
```

## 测试

前端测试：

```bash
npm test
```

Rust 测试：

```bash
. ./.cargo/env
cd src-tauri
cargo test
```

## 配置说明

每个 provider 都支持在设置页单独配置：

- `API Key`
- `接口地址`
- `模型提示`

现在可以直接点击菜单栏弹层里的 `设置` 进入配置页，无需先打开独立窗口。

默认行为：

- `Zhipu` 使用官方 5 小时窗口接口
- `MiniMax` 使用 coding plan remains 接口
- `Kimi` 默认按余额接口处理；如果你有更稳定的内部/代理接口，可以直接在设置页覆盖

## 统计口径

- `当前窗口`：优先显示官方当前返回值
- `7 天 / 30 天`：按本机历史快照做滚动聚合
- `Kimi`：当前展示余额与历史变化，不强行伪装成官方 5 小时/7 天配额
