# Rust C2 Framework

一个使用Rust编写的命令与控制(C2)框架，包含客户端、服务端和Web管理界面。

## 功能特性

- 🚀 **平台支持**: 目前仅支持Linux
- 🔐 **安全通信**: 使用AES-256-GCM加密通信
- 🌐 **Web管理界面**: 现代化的Web控制台
- 🔄 **异步架构**: 基于Tokio的高性能异步架构
- 💻 **远程命令执行**: 实时命令执行和结果反馈
- 🖥️ **反弹Shell**: 交互式Shell会话
- 📊 **实时监控**: 客户端状态实时监控
- 🔍 **系统信息收集**: 自动收集目标系统信息

## 项目结构

```shell
rust-c2-framework/
├── src/
│   ├── lib.rs              # 库文件
│   ├── common.rs           # 通用模块（协议、加密、网络工具）
│   └── bin/                # 可执行文件目录
├── client/
│   └── main.rs             # 客户端主程序
├── server/
│   └── main.rs             # 服务端主程序
├── templates/
│   ├── index.html          # 主页模板
│   └── client.html         # 客户端管理页面模板
├── web/
│   └── static/             # 静态文件
└── Cargo.toml              # 项目配置
```

## 快速开始

### 1. 编译项目

```bash
# 编译所有组件
cargo build --release

# 或者分别编译
cargo build --release --bin server
cargo build --release --bin client
```

### 2. 静态编译 (MUSL)

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

### 3. 使用 UPX 压缩 (可选)

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

### 4. 启动服务端

```bash
# 启动服务端（默认监听 0.0.0.0:8080）
cargo run --bin server

# 或者运行编译后的二进制文件
./target/release/server
```

服务端启动后，可以通过浏览器访问 `http://localhost:8080` 查看Web管理界面。

### 3. 启动客户端

```bash
# 连接到服务端（默认连接 http://127.0.0.1:8080）
cargo run --bin client

# 或者指定服务端地址
cargo run --bin client http://your-server-ip:8080

# 或者运行编译后的二进制文件
./target/release/client http://your-server-ip:8080
```

## 使用指南

### Web管理界面

1. **主页面**: 显示所有连接的客户端列表，包括状态、系统信息等
2. **客户端管理**: 点击"管理"按钮进入客户端详情页面
3. **命令执行**: 在客户端详情页面可以执行命令并查看结果
4. **快捷命令**: 提供常用命令的快捷按钮
5. **反弹Shell**: 启动交互式Shell会话

### 命令执行

支持的命令类型：

- 系统命令（如 `ls`, `pwd`, `whoami`）
- 网络命令（如 `netstat`, `ss`）
- 进程管理（如 `ps`, `kill`）
- 文件操作（如 `cat`, `find`）

### 安全特性

- 通信加密：所有客户端与服务端之间的通信都使用AES-256-GCM加密
- 身份验证：客户端注册机制，防止未授权连接
- 错误处理：完善的错误处理机制，确保程序稳定运行

## 开发指南

### 依赖项

主要依赖：

- `tokio`: 异步运行时
- `serde`: 序列化/反序列化
- `axum`: Web框架
- `reqwest`: HTTP客户端
- `aes-gcm`: AES-GCM加密
- `askama`: 模板引擎

### 扩展功能

可以通过以下方式扩展功能：

1. 在 `src/common.rs` 中添加新的消息类型
2. 在客户端和服务端添加相应的处理逻辑
3. 在Web界面添加新的管理功能

### 消息协议

系统使用基于JSON的消息协议，主要消息类型：

- `ClientRegister`: 客户端注册
- `Heartbeat`: 心跳消息
- `ExecuteCommand`: 命令执行
- `CommandResult`: 命令结果
- `ReverseShell`: 反弹Shell
- `ShellData`: Shell数据

## 免责声明

⚠️ **重要提醒**: 本项目仅用于教育和研究目的。请勿将其用于任何非法或恶意活动。使用者需要承担使用本软件的全部责任。

## 许可证

本项目采用MIT许可证。详情请参阅LICENSE文件。

## 贡献

欢迎提交Issue和Pull Request来改进此项目。

## 联系方式

如有问题或建议，请通过GitHub Issues联系。
