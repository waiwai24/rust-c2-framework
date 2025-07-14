# Rust C2 Framework

<!-- markdownlint-disable MD033 -->
<div style="display: flex; justify-content: center;">
    <img src="web/static/rust-c2.png" alt="rust-c2" width="200"/>
</div>
<p align="center">
    <img alt="Language" src="https://img.shields.io/github/languages/top/waiwai24/rust-c2-framework">
    <img alt="Commit" src="https://img.shields.io/github/commit-activity/m/waiwai24/rust-c2-framework">
	<img alt="Bitbucket open issues" src="https://img.shields.io/github/issues/waiwai24/rust-c2-framework">
    <img alt="GitHub" src="https://img.shields.io/github/license/waiwai24/rust-c2-framework">
    <img alt="GitHub Repo stars" src="https://img.shields.io/github/stars/waiwai24/rust-c2-framework">
</p>
<!-- markdownlint-enable MD033 -->

现代化、安全、高性能的Command & Control解决方案 🦀,一个使用Rust语言从零重构的现代化命令与控制（C2）框架，旨在提供一个高性能、安全且模块化的平台，用于远程系统管理和渗透测试。该框架包含独立的客户端、服务端以及一个直观的Web管理界面。

## 🚀 功能特性

### 核心功能

- 🔗 **Linux平台支持**: 客户端仅支持Linux操作系统，服务端支持Windows，Linux系统
- 🔐 **端到端加密通信**: 采用AES-256-GCM加密算法，确保部分通信数据的机密性和完整性
- 🌐 **现代化Web管理界面**: 基于Axum Web框架和Askama模板引擎，提供响应式用户体验
- ⚡ **高性能异步架构**: 利用Tokio运行时构建，实现高效的并发处理和低延迟通信
- 📝 **灵活的配置管理**: 通过TOML配置文件轻松调整各项参数，无需重新编译

### 高级功能

- 💻 **实时命令执行**: 支持远程执行系统命令，并即时获取执行结果，支持加密传输
- 🖥️ **交互式反弹Shell**: 提供稳定的反向Shell会话，实现对受控主机的深度交互
- 📁 **完整文件管理**: 文件浏览、上传、下载、删除，支持大文件分块传输
- 📊 **实时客户端监控**: 展示连接客户端的详细信息、活动状态和系统信息
- 🔍 **审计日志系统**: 记录所有操作活动，支持日志分类和实时查看
- 📝 **笔记管理**: 内置笔记系统，便于记录渗透测试过程和发现

### 安全特性

- 🛡️ **按需启动监听**: 反弹Shell监听器按需启动，减少攻击面
- 🔑 **身份验证**: Web界面用户认证和会话管理
- 📋 **操作审计**: 完整的操作审计链，支持安全合规
- 🔒 **进程隐藏**: 客户端进程隐藏技术
- 🔍 **抗逆向**: 代码混淆和反调试技术
- 📦 **反沙箱**: 沙箱环境检测和规避

## 📋 项目结构

本项目采用Cargo工作区管理，结构清晰，各模块职责明确：

```text
rust-c2-framework/
├── Cargo.toml                    # Workspace配置文件
├── server_config.toml            # 服务器配置文件
├── common/          # 共享库（协议、加密、配置）
├── server/          # 服务端（Web界面、API、客户端管理）
├── client/          # 客户端（命令执行、文件操作、反弹Shell）
└── web/static/      # 前端资源（CSS、JS、图片）
```

## ⚙️ 配置文件

### 服务端配置 (`server_config.toml`)

```toml
# 服务器网络配置
host = "0.0.0.0"                              # 服务器监听地址
port = 8080                                   # Web服务端口
reverse_shell_port = 31229                    # 反弹Shell专用端口

# 安全配置
encryption_key = "your-32-byte-secret-key-here!!!!"  # AES-256密钥（必须32字节）

# 客户端管理
client_timeout = 60                           # 客户端超时时间（秒）
max_clients = 1000                           # 最大客户端连接数

# 日志配置
log_file = "c2_server.log"                   # 日志文件路径
enable_audit = true                          # 启用审计日志

# Web界面配置
[web]
enabled = true                               # 启用Web界面
static_dir = "web/static"                    # 静态文件目录
template_dir = "server/templates"            # 模板文件目录
enable_cors = true                           # 启用CORS
refresh_interval = 5                         # 自动刷新间隔（秒）

# 身份验证配置
[auth]
username = "Rust-Admin"                      # 登录用户名
password = "Passwd@RustC2"                   # 登录密码
```

## 🚀 快速开始

### 1. 安装Rust环境

```bash
# 安装Rust（如果尚未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 验证安装
rustc --version
cargo --version
```

### 2. 克隆和编译项目

```bash
# 克隆项目
git clone https://github.com/waiwai24/rust-c2-framework
cd rust-c2-framework

# 编译整个工作区（推荐）
cargo build --release

# 或者分别编译各个组件
cargo build --release --bin server
cargo build --release --bin client
```

编译后的二进制文件位于 `target/release/` 目录下。

### 3. 静态编译（可选）

为了生成不依赖系统库的独立可执行文件：

```bash
# 添加musl目标
rustup target add x86_64-unknown-linux-musl

# 静态编译
cargo build --release --target x86_64-unknown-linux-musl

# 使用UPX压缩（可选）
sudo apt-get install upx
upx --best target/x86_64-unknown-linux-musl/release/server
upx --best target/x86_64-unknown-linux-musl/release/client
```

### 4. 配置和启动

#### 配置服务器

```bash
# 使用cargo运行（开发环境）
cargo run --bin server

# 或直接运行二进制文件（生产环境）
./target/release/server
```

服务器启动后，访问 `http://localhost:8080` 进入Web管理界面。

默认登录凭据：

- 用户名: `Rust-Admin`
- 密码: `Passwd@RustC2`

#### 配置和启动客户端

确保加密密钥与服务器一致，启动客户端：

```bash
# 使用配置文件
cargo run --bin client

# 或指定服务器地址
cargo run --bin client http://your-server-ip:8080

# 直接运行二进制文件
./target/release/client
```

客户端云沙箱检测（测试时间：2025/7/14）：

<!-- markdownlint-disable MD033 -->
<div style="display: flex; justify-content: center; gap: 20px;">
    <img src="img/sandbox1.png" alt="sandbox1" width="640">
    <img src="img/sandbox2.png" alt="sandbox2" width="640">
</div>
<!-- markdownlint-disable MD033 -->

详细功能使用指南请参考 [Function.md](Function.md)。

## 🔄 更新日志

### v0.1.0 (最新版本)

- ✅ 完整的C2框架实现
- ✅ 反弹Shell功能（按需启动监听器）
- ✅ 文件管理系统（上传/下载/浏览）
- ✅ 实时Web界面
- ✅ AES-256-GCM加密通信
- ✅ 客户端状态监控
- ✅ 审计日志系统
- ✅ 笔记管理功能
- ✅ WebSocket实时通信
- ✅ 模块化前端设计
- ✅ 响应式界面布局

### 已知问题

- Linux平台的完整测试需要进一步验证
- 大文件传输的进度显示有待优化
- 传输加密有待完善

### 计划功能

- [ ] Windows客户端支持
- [ ] macOS客户端支持  
- [ ] Android客户端支持
- [ ] 插件系统
- [ ] 隧道功能

## ⚠️ 免责声明

**重要提醒**: 本项目仅用于教育和研究目的。请勿将其用于任何非法或恶意活动。使用者需要承担使用本软件的全部责任。

### 合法使用声明

- ✅ 网络安全教育和培训
- ✅ 渗透测试（已获得授权）
- ✅ 红队演练（合规环境）
- ❌ 未经授权的系统访问
- ❌ 恶意软件传播
- ❌ 违法犯罪活动

使用本工具前，请确保：

1. 获得目标系统所有者的明确授权
2. 遵守当地法律法规
3. 仅在合法合规的环境中使用
4. 承担相应的法律责任

## 📄 许可证

本项目采用 MIT 许可证。详细信息请参见 [LICENSE](LICENSE) 文件。

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=waiwai24/rust-c2-framework&type=Date)](https://www.star-history.com/#waiwai24/rust-c2-framework&Date)
