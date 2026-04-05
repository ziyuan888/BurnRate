# BurnRate

![Version](https://img.shields.io/badge/version-0.2.2-blue?logo=semver)
![macOS](https://img.shields.io/badge/platform-macOS-111827?logo=apple&logoColor=white)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![React 19](https://img.shields.io/badge/React-19-149ECA?logo=react&logoColor=white)
![Rust 2021](https://img.shields.io/badge/Rust-2021-000000?logo=rust)
![License](https://img.shields.io/badge/license-Apache_2.0-4D7C0F)

BurnRate 是一个基于 `Tauri 2 + Rust + React` 的 macOS 菜单栏应用，用来监控三类套餐的使用情况：

它的目标很直接：把 AI 套餐的剩余额度、当前窗口状态和最近 7/30 天变化，固定放进菜单栏里，减少手动登录各家后台反复查看的成本。

- 智谱清言（Zhipu）5 小时窗口
- MiniMax 当前套餐窗口
- Kimi 账户余额，或 Kimi Code 的 7 天额度 / 5 小时频限

当前版本已经具备：

- 菜单栏 tray 常驻，tooltip 显示各 provider 实时状态
- **Glassmorphism 毛玻璃风格** Popover 仪表盘
  - 顶部标题栏显示当前时间，快捷图标按钮（刷新 / 设置 / 关闭）
  - **独立 provider 卡片**：每个 provider 以独立卡片展示，包含状态色徽章
  - 双进度条：主进度条（5小时窗口/当前周期）+ 次进度条（7天额度/总配额）
  - 用量百分比显示，>80% 黄色警告，>95% 红色危险
  - 指标网格：模型用量（tokens，自动切换 K/M 后缀）和工具调用次数
  - 工具调用详情列表（需 MCP/Coding API 数据支持）
  - 总体状态汇总
  - 底部操作栏：立即刷新、退出
  - 卡片入场动画、进度条填充动画、刷新按钮旋转动画
- 套餐卡片显示下一次重置时间（如 `今天 15:30` / `明天 00:15`）
- Popover 内直接进入设置表单并录入套餐密钥
- **独立设置窗口**（glassmorphism 面板风格）
- **iOS 风格开关**：设置页内可直接 toggle 启用/禁用各 provider
- Rust 侧轮询、落盘、后台自动刷新
- 本地 SQLite 历史统计
- 用量估算（基于 provider 配额比例推算 tokens / 消息数）
- API Key 本地 SQLite 保存（当前不走系统钥匙串）
- `tauri build --debug` 可生成 `.app`

## 技术栈

- 桌面容器：`Tauri 2`
- 前端界面：`React 19 + TypeScript + Zustand + Vite`，Glassmorphism 毛玻璃 UI
- 后端能力：`Rust + reqwest + rusqlite + tokio`
- 本地存储：`SQLite`
- 动画：`CSS Keyframes`（入场动画、进度条、旋转刷新）

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

现在可以直接点击菜单栏弹层里顶部标题栏的齿轮图标进入配置页，无需先打开独立窗口。

默认行为：

- `Zhipu` 使用官方 5 小时窗口接口
- `MiniMax` 使用 coding plan remains 接口
- `Kimi` 默认按余额接口处理；如果你在设置页填入的是控制台里的 `Bearer Token`，则会自动切到 Kimi Code 用量接口，读取 7 天额度和 5 小时频限

当前版本的密钥存储方式：

- provider 的 API Key / Token 会保存在应用本地生成的 `burnrate.db` 里
- 当前实现是 SQLite 持久化，不走系统钥匙串
- 当前实现也不是加密存储，因此不要把运行时生成的本地数据库文件分享给别人

## 获取 Kimi Token

如果你只填普通 `Kimi API Key`，主页面会显示账户余额。

如果你想显示 `Kimi Code` 的：

- `7 天额度`
- `5 小时频限`
- `下次重置时间`

需要填入 `Kimi 控制台 Bearer Token`。

获取步骤：

1. 打开 `https://www.kimi.com/code/console?from=kfc_overview_topbar` 并完成登录
2. 打开浏览器开发者工具
3. 切到 `Network` 面板，并勾选 `Preserve log`
4. 刷新控制台页面
5. 在请求列表里搜索 `GetUsages` 或 `BillingService`
6. 点开 `https://www.kimi.com/apiv2/kimi.gateway.billing.v1.BillingService/GetUsages`
7. 在 `Request Headers` 中找到 `Authorization`
8. 复制其中的 `Bearer ...` 整段内容，或只复制后面的 token
9. 回到 BurnRate 设置页，在 `Kimi` 的 `API Key / 控制台 Token` 输入框里粘贴并保存
10. 回到主页面点击 `立即刷新`

拿到正确 token 后，Kimi 卡片会切换为：

- 标题：`5 小时窗口`
- 当前值：5 小时频限已使用比例
- 说明：`本周额度 xx%`
- 下次重置：5 小时窗口的下一次重置时间

注意事项：

- 这个 token 本质上是 `Kimi 控制台登录态`，不要分享给别人
- 如果 token 过期、退出登录或控制台风控失效，BurnRate 会刷新失败，此时重新按上面的步骤获取一次即可
- 这是基于当前控制台请求抓取的接入方式，不属于公开稳定 API，后续如果 Kimi 改了控制台接口，获取步骤可能需要跟着调整

## 统计口径

- `当前窗口`：优先显示官方当前返回值
- `下次重置`：优先显示 provider 返回的下一次重置时间，卡片里会按本地时区展示成 `今天 HH:mm`、`明天 HH:mm` 或 `MM-DD HH:mm`；如果当前快照暂时没带 reset 字段，会优先沿用最近历史里的 reset 节点继续推导，避免主页面卡片突然空白
- `7 天 / 30 天`：按本机历史快照做滚动聚合
- `Kimi`：普通 API Key 仍按余额接口展示；如果要显示 Kimi Code 的“本周用量 / 5 小时频限”，需要填控制台登录态里的 `Bearer Token`
