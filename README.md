# Rust C2 Framework

<!-- markdownlint-disable MD033 -->
<img src="web/static/rust-c2.png" alt="rust-c2" width="400"/>
<!-- markdownlint-enable MD033 -->

一个使用Rust语言从零重构的现代化命令与控制（C2）框架，旨在提供一个高性能、安全且模块化的平台，用于远程系统管理和渗透测试。该框架包含独立的客户端、服务端以及一个直观的Web管理界面。

## 功能特性

- 🚀 **跨平台支持**: 客户端和服务端均支持Linux操作系统。
- 🔐 **端到端加密通信**: 采用AES-256-GCM加密算法，确保所有通信数据的机密性和完整性。
- 🌐 **直观的Web管理界面**: 基于Axum Web框架和Askama模板引擎构建，提供现代化的用户体验，便于操作和监控。
- ⚡ **高性能异步架构**: 利用Tokio运行时构建，实现高效的并发处理和低延迟通信。
- 💻 **实时命令执行**: 支持远程执行系统命令，并即时获取执行结果。
- 🖥️ **交互式反弹Shell**: 提供稳定的反向Shell会话，实现对受控主机的深度交互。
- 📊 **客户端状态监控**: 实时展示连接客户端的详细信息和活动状态。
- 📝 **灵活的配置管理**: 通过TOML配置文件轻松调整客户端和服务端的各项参数，无需重新编译。
- 📦 **Cargo工作区**: 采用Rust的Cargo工作区管理模式，清晰划分 `common`、`client` 和 `server` 等模块，便于开发和维护。

## 项目结构

本项目采用Cargo工作区管理，结构清晰，各模块职责明确：

```shell
rust-c2-framework/
├── common/               # 通用库：包含协议定义、加密工具、配置解析、错误处理等共享组件。
│   ├── src/
│   └── Cargo.toml
├── client/               # 客户端Crate：负责与服务端通信、执行命令、提供反弹Shell等功能。
│   ├── src/
│   └── Cargo.toml
├── server/               # 服务端Crate：处理客户端连接、管理会话、提供Web管理界面和API服务。
│   ├── src/
│   ├── templates/        # Web界面HTML模板文件。
│   └── Cargo.toml
├── web/
│   └── static/           # Web管理界面的静态资源（CSS、JavaScript、图片等）。
└── Cargo.toml            # Cargo工作区配置文件，定义了各个Crate的依赖关系和构建方式。
```

## 快速开始

### 1. 配置

服务端和客户端的配置通过TOML文件进行管理，允许在不重新编译的情况下灵活修改。

**服务端配置 (`server_config.toml`)**:

在运行服务端二进制文件的相同目录下创建此文件。如果文件不存在，服务端将无法启动。

```toml
host = "0.0.0.0"
port = 8080
encryption_key = "a_very_secret_key_that_is_32_bytes" # ⚠️ 务必替换为32字节的强密钥！
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

使用Cargo编译整个工作区或单独的Crate：

```bash
# 编译整个工作区（推荐）
cargo build --release

# 或者分别编译服务端和客户端
cargo build --release --bin server
cargo build --release --bin client
```

编译后的二进制文件位于 `target/release/` 目录下。

### 3. 静态编译 (MUSL)

为了生成不依赖系统库的独立可执行文件，可以使用 `musl` 目标进行静态链接编译。

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

UPX (Ultimate Packer for eXecutables) 可以进一步减小静态链接二进制文件的大小。

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
# 启动服务端（推荐使用 Cargo）
cargo run --bin server

# 或者直接运行编译后的二进制文件
./target/release/server
```

服务端启动后，通过浏览器访问 `http://localhost:8080` 即可进入Web管理界面。默认登录凭据为 `admin` / `password`。

### 6. 启动客户端

```bash
# 连接到服务端（默认连接 http://127.0.0.1:8080）
cargo run --bin client

# 或者指定服务端地址
cargo run --bin client http://your-server-ip:8080

# 或者直接运行编译后的二进制文件
./target/release/client http://your-server-ip:8080
```

## 开发指南

### 模块化设计

本项目遵循清晰的模块化设计原则，便于理解和扩展：

- **`common`**: 核心共享库，包含所有跨Crate通用的代码，如 `message` 协议定义、`config` 结构、`error` 类型、`crypto` 加密工具和 `network` 辅助函数。
- **`server`**: 服务端核心逻辑。
  - `main.rs`: 程序入口，负责初始化服务器、设置路由和启动服务。
  - `state.rs`: 定义应用程序的共享状态 `AppState`，管理全局数据。
  - `managers/`: 包含 `ClientManager`（管理客户端连接）和 `ShellManager`（处理反弹Shell会话）等核心业务逻辑管理器。
  - `handlers/`: 包含 `api.rs`（处理RESTful API请求）和 `web.rs`（处理Web页面渲染请求）。
  - `audit.rs`: 负责审计日志的记录功能。
  - `auth.rs`: 实现Web界面的用户认证逻辑。
- **`client`**: 客户端核心逻辑，负责与服务端建立连接、执行接收到的命令、管理文件、提供反弹Shell功能等。

## 免责声明

⚠️ **重要提醒**: 本项目仅用于教育和研究目的。请勿将其用于任何非法或恶意活动。使用者需要承担使用本软件的全部责任。
