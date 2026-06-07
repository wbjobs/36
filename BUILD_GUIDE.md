# 跨平台本地邮件客户端 - 构建指南

## 项目概述

这是一个基于 Tauri 框架的跨平台本地邮件客户端，使用 Vue3 + TypeScript 作为前端，Rust 作为后端。

### 主要功能
- ✅ 多账户管理（最多 5 个账户）
- ✅ IMAP 协议收取邮件
- ✅ SQLite 本地存储（邮件、附件、账户、标签）
- ✅ FTS5 全文搜索（支持搜索正文、主题、发件人）
- ✅ 自动分类（工作、个人、订阅、垃圾）
- ✅ 系统托盘图标和未读邮件通知

---

## 一、环境配置

### 1. Windows 编译环境配置

#### 方案 A：使用 MSVC 工具链（推荐）
```powershell
# 1. 安装 Visual Studio Build Tools
# 下载地址：https://visualstudio.microsoft.com/downloads/
# 安装时勾选 "使用 C++ 的桌面开发"

# 2. 设置 Rust 默认工具链为 MSVC
rustup default stable-x86_64-pc-windows-msvc

# 3. 验证
rustc --version
cargo --version
```

#### 方案 B：使用 MinGW 工具链
```powershell
# 1. 安装 MinGW-w64
winget install --id BrechtSanders.WinLibs.POSIX.UCRT -e

# 2. 添加到 PATH（假设安装到 C:\Program Files\winlibs\mingw64\bin）
$env:PATH = "C:\Program Files\winlibs\mingw64\bin;$env:PATH"
[Environment]::SetEnvironmentVariable("PATH", $env:PATH, "User")

# 3. 设置 Rust 默认工具链为 GNU
rustup default stable-x86_64-pc-windows-gnu

# 4. 验证
gcc --version
dlltool --version
```

### 2. Node.js 环境
```powershell
# 安装 Node.js 18+
# 下载地址：https://nodejs.org/

# 验证
node --version
npm --version
```

### 3. WebView2（Windows）
Windows 11 已预装，Windows 10 可能需要安装：
https://developer.microsoft.com/microsoft-edge/webview2/

---

## 二、依赖安装

### 1. 前端依赖
```powershell
cd e:\trae3\6
npm install
```

### 2. Rust 依赖
Rust 依赖会在构建时自动下载。

---

## 三、生成应用图标

### 方法 1：使用 Tauri 内置工具
```powershell
# 安装 Tauri CLI
npm install -g @tauri-apps/cli

# 生成图标
tauri icon src-tauri/icons/icon.svg
```

### 方法 2：使用脚本
```powershell
# 安装 sharp
npm install sharp -D

# 运行图标生成脚本
node scripts/generate-icons.js
```

### 需要的图标文件
确保 `src-tauri/icons/` 目录下有以下文件：
- `32x32.png` - 系统托盘图标
- `128x128.png`
- `128x128@2x.png`
- `icon.png` (512x512)
- `icon.ico` - Windows 图标
- `icon.icns` - macOS 图标

---

## 四、开发构建

### 1. 前端构建（已完成）
```powershell
npm run build
```

### 2. Tauri 开发模式
```powershell
npm run tauri dev
```

### 3. Tauri 生产构建
```powershell
npm run tauri build
```

构建完成后，安装包将在 `src-tauri/target/release/bundle/` 目录下。

---

## 五、常见问题

### 1. 编译错误：`dlltool.exe not found`
**原因**：MinGW 工具链不完整

**解决方案**：
```powershell
# 方法 1：使用完整的 MinGW 安装
winget install BrechtSanders.WinLibs.POSIX.UCRT -e

# 方法 2：切换到 MSVC 工具链
rustup default stable-x86_64-pc-windows-msvc
```

### 2. 编译错误：`link.exe not found`
**原因**：缺少 MSVC 构建工具

**解决方案**：
安装 Visual Studio Build Tools，勾选 "使用 C++ 的桌面开发"

### 3. IMAP 连接失败
**常见原因**：
- 邮箱需要使用授权码而非登录密码
- IMAP 服务未开启
- 防火墙阻止连接

**主流邮箱设置**：
| 邮箱 | IMAP 服务器 | 端口 | SSL |
|------|------------|------|-----|
| QQ邮箱 | imap.qq.com | 993 | ✅ |
| 163邮箱 | imap.163.com | 993 | ✅ |
| Gmail | imap.gmail.com | 993 | ✅ |
| Outlook | outlook.office365.com | 993 | ✅ |

### 4. 数据库文件位置
- Windows: `%APPDATA%\mail-client\mail.db`
- macOS: `~/Library/Application Support/mail-client/mail.db`
- Linux: `~/.config/mail-client/mail.db`

---

## 六、项目结构

```
e:\trae3\6\
├── src/                          # 前端源码
│   ├── api/index.ts             # API 层（Tauri IPC 调用）
│   ├── components/
│   │   └── Sidebar.vue          # 侧边栏组件
│   ├── stores/mail.ts           # Pinia 状态管理
│   ├── types/index.ts           # TypeScript 类型定义
│   ├── views/
│   │   ├── Inbox.vue            # 收件箱页面
│   │   └── Accounts.vue         # 账户管理页面
│   ├── router/index.ts          # Vue Router 配置
│   ├── App.vue                  # 根组件
│   └── main.ts                  # 入口文件
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── lib.rs               # 主库文件
│   │   ├── db.rs                # 数据库模块
│   │   ├── imap_client.rs       # IMAP 客户端
│   │   ├── classifier.rs        # 邮件自动分类器
│   │   ├── commands.rs          # Tauri IPC 命令
│   │   ├── tray.rs              # 系统托盘模块
│   │   ├── models.rs            # 数据模型
│   │   └── error.rs             # 错误定义
│   ├── Cargo.toml               # Rust 依赖
│   ├── tauri.conf.json          # Tauri 配置
│   └── icons/                   # 应用图标
├── scripts/
│   └── generate-icons.js        # 图标生成脚本
├── dist/                         # 前端构建输出（已生成）
├── package.json                  # 前端依赖
├── vite.config.ts               # Vite 配置
└── BUILD_GUIDE.md               # 本文档
```

---

## 七、数据库设计

### 表结构
1. **accounts** - 账户表
   - id, name, email, imap_server, imap_port, username, password, use_ssl

2. **emails** - 邮件表
   - id, account_id, message_id, subject, sender_name, sender_email, recipients, date, body_text, body_html, is_read, is_flagged, uid

3. **attachments** - 附件表
   - id, email_id, filename, content_type, size, content_id, file_path

4. **tags** - 标签表
   - id, name, color, is_system
   - 系统标签：工作、个人、订阅、垃圾

5. **email_tags** - 邮件标签关联表
   - email_id, tag_id

6. **emails_fts** - FTS5 全文搜索虚拟表
   - subject, sender_name, sender_email, body_text

---

## 八、自动分类规则

### 垃圾邮件
- 关键词：spam、垃圾、中奖、免费、赚钱等
- 域名：包含 spam、junk、unknown

### 订阅邮件
- 关键词：newsletter、订阅、unsubscribe、资讯、推广等
- 域名：newsletter、mailgun、sendgrid、mailchimp

### 工作邮件
- 域名：@work、@company、@corp、@business
- 关键词：meeting、会议、report、报告、deadline、project 等

### 个人邮件
- 域名：@gmail、@qq、@163、@outlook、@hotmail、@yahoo 等常见个人邮箱

---

## 九、后续优化建议

1. **SMTP 发送功能**：添加邮件发送支持
2. **邮件加密**：支持 PGP/GPG 加密
3. **多线程同步**：优化大量邮件的同步性能
4. **离线模式**：完善离线浏览体验
5. **规则引擎**：支持用户自定义分类规则
6. **邮件导出**：支持导出为 EML/PDF 格式
7. **深色模式**：添加深色主题支持

---

## 十、技术栈

### 前端
- Vue 3 + TypeScript
- Pinia（状态管理）
- Vue Router（路由）
- Vite（构建工具）
- Day.js（日期处理）

### 后端
- Rust
- Tauri（桌面框架）
- rusqlite（SQLite 绑定）
- imap（IMAP 客户端）
- mail-parser（邮件解析）
- tokio（异步运行时）
- native-tls（TLS 加密）
- regex（正则表达式）

---

**开发完成时间**：2025年
**许可证**：MIT
