# Rust C2 Framework

   <img src="web/static/rust-c2.png" alt="rust-c2" width="400"/>

一个使用Rust编写的、经过重构的命令与控制(C2)框架，具有清晰的模块化结构，包含客户端、服务端和Web管理界面。

## 功能特性

- 🚀 **平台支持**: 支持Linux
- 🔐 **安全通信**: 使用AES-256-GCM加密通信
- 🌐 **Web管理界面**: 现代化的Web控制台，使用Axum和Askama构建
- 🔄 **异步架构**: 基于Tokio的高性能异步架构
- 💻 **远程命令执行**: 实时命令执行和结果反馈
- 🖥️ **反弹Shell**: 交互式Shell会话
- 📊 **实时监控**: 客户端状态实时监控
- 📝 **配置管理**: 通过`toml`文件轻松配置客户端和服务端
- 📦 **工作区结构**: 使用Cargo工作区管理多个crate

## 项目结构

项目现在是一个Cargo工作区，结构如下：

```shell
rust-c2-framework/
├── common/               # 通用库 (协议, 加密, 配置, 错误处理)
│   ├── src/
│   └── Cargo.toml
├── client/               # 客户端Crate
│   ├── src/
│   └── Cargo.toml
├── server/               # 服务端Crate
│   ├── src/
│   ├── templates/        # Web模板
│   └── Cargo.toml
├── web/
│   └── static/           # 静态文件 (CSS, JS等)
└── Cargo.toml            # 工作区配置
```

## 快速开始

### 1. 配置

服务端会在运行时从配置文件中读取配置。这意味着您可以在不重新编译的情况下修改这些文件来更改配置。

**服务端配置 (`server_config.toml`)**:

在运行服务端二进制文件的相同目录下，创建一个名为 `server_config.toml` 的文件。如果文件不存在，服务端将无法启动并报错。

```toml
host = "0.0.0.0"
port = 8080
encryption_key = "a_very_secret_key_that_is_32_bytes"
client_timeout = 300
max_clients = 1000
log_file = "c2_server.log"
enable_audit = true

[web]
enabled = true
static_dir = "web/static"
template_dir = "server/templates" # 注意路径
enable_cors = true
```

### 2. 编译项目

```bash
# 编译整个工作区
cargo build --release

# 或者分别编译
cargo build --release --bin server
cargo build --release --bin client
```

### 3. 静态编译 (MUSL)

为了生成完全静态链接的二进制文件，可以使用 `musl` 目标。这对于创建独立的可执行文件非常有用，无需依赖系统库。

**先决条件**:
安装 `musl` 工具链：

```bash
rustup target add x86_64-unknown-linux-musl
```

**编译**:

```bash
# 编译所有组件为静态链接
cargo build --release --target x86_64-unknown-linux-musl

# 或者分别编译
cargo build --release --bin server --target x86_64-unknown-linux-musl
cargo build --release --bin client --target x86_64-unknown-linux-musl
```

编译后的二进制文件位于 `target/x86_64-unknown-linux-musl/release/` 目录下。

### 4. 使用 UPX 压缩 (可选)

UPX (Ultimate Packer for eXecutables) 可以进一步压缩静态链接的二进制文件，减小其大小。

**先决条件**:
安装 UPX：

```bash
sudo apt-get install upx
```

**压缩**:

```bash
# 压缩服务端二进制文件
upx --best target/x86_64-unknown-linux-musl/release/server

# 压缩客户端二进制文件
upx --best target/x86_64-unknown-linux-musl/release/client
```

### 5. 启动服务端

```bash
# 启动服务端
cargo run --bin server

# 或者运行编译后的二进制文件
./target/release/server
```

服务端启动后，可以通过浏览器访问 `http://localhost:8080` 查看Web管理界面。默认登录凭据为 `admin` / `password`。

### 6. 启动客户端

```bash
# 连接到服务端（默认连接 http://127.0.0.1:8080）
cargo run --bin client

# 或者指定服务端地址
cargo run --bin client http://your-server-ip:8080

# 或者运行编译后的二进制文件
./target/release/client http://your-server-ip:8080
```

## 开发指南

### 模块化设计

- **`common`**: 包含所有共享代码，如 `message` 协议、`config` 结构、`error` 类型、`crypto` 工具和 `network` 辅助函数。
- **`server`**: 包含所有服务端逻辑。
  - `main.rs`: 程序入口，设置路由和启动服务器。
  - `state.rs`: 定义共享的 `AppState`。
  - `managers/`: 包含 `ClientManager` 和 `ShellManager`，用于处理核心业务逻辑。
  - `handlers/`: 包含 `api.rs` 和 `web.rs`，分别处理API请求和Web页面渲染。
  - `audit.rs`: 审计日志记录。
  - `auth.rs`: Web界面认证。
- **`client`**: 包含所有客户端逻辑。

## 免责声明

⚠️ **重要提醒**: 本项目仅用于教育和研究目的。请勿将其用于任何非法或恶意活动。使用者需要承担使用本软件的全部责任。
