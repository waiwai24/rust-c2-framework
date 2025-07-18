# 功能详细文档

## 🎯 Web管理界面功能

### 1. 仪表盘 (`/`)

- **客户端概览**: 在线客户端数量、操作系统分布统计
- **客户端列表**: 显示所有连接的客户端信息
- **实时状态**: 客户端连接状态、最后见时间

![client](img/client.png)

### 2. 客户端详情页 (`/client/{id}`)

- **基本信息**: 主机名、用户名、IP地址、操作系统
- **硬件信息**: CPU品牌、频率、核心数、内存大小
- **存储信息**: 磁盘总容量、可用空间
- **功能标签页**:
  - 📟 **命令执行**: 实时命令执行和结果显示
  - 🖥️ **反弹Shell**: 交互式Shell终端
  - 📁 **文件管理**: 文件浏览、上传、下载
  - 📋 **日志查看**: 操作日志和系统日志
  - 📝 **笔记**: 渗透测试笔记和发现记录

## 🔧 核心功能使用

### 1. 命令执行

```javascript
// 在命令执行面板中输入命令
whoami
uname -a
ps aux
netstat -tulpn
```

![execute-command](img/execute-command.png)

### 2. 反弹Shell

1. 点击"启动反弹Shell"按钮
2. 系统自动启动监听器并发送连接指令给客户端
3. 在终端界面进行交互式操作

![reverse-shell](img/reverse-shell.png)

### 3. 文件管理

- **浏览目录**: 点击目录名称进入
- **上传文件**: 拖拽文件到上传区域或点击选择文件
- **下载文件**: 点击文件名或下载按钮
- **删除文件**: 选择文件后点击删除按钮

![file](img/file.png)

### 4. 日志管理

- **实时日志**: 自动更新的操作日志
- **日志过滤**: 按类型、级别过滤日志
- **日志清理**: 清除历史日志记录

![log](img/log.png)

## 🔒 安全特性详解

### 1. 通信加密

- **AES-256-GCM**: 对所有敏感数据进行加密传输
- **密钥管理**: 服务器和客户端共享密钥认证
- **完整性验证**: GCM模式提供数据完整性检查

### 2. 身份验证

- **Web界面认证**: 用户名/密码登录验证
- **会话管理**: 基于Cookie的安全会话
- **路由保护**: 中间件层面的访问控制

### 3. 审计和日志

- **操作审计**: 记录所有客户端操作和命令执行
- **连接跟踪**: 详细的连接建立和断开日志
- **错误记录**: 系统错误和异常的完整记录

### 4. 进程安全

- **按需启动**: 反弹Shell监听器仅在需要时启动
- **进程隐藏**: 客户端进程隐藏技术
- **内存安全**: Rust的内存安全保证

### 编译优化配置

项目已配置编译优化，在 `Cargo.toml` 中定义：

```toml
[profile.release]
lto = true                # 链接时优化
codegen-units = 1         # 单个代码生成单元
panic = "abort"           # panic时直接终止
strip = true              # 剥离调试符号
opt-level = "z"           # 优化文件大小
```

## 📊 性能特性

### 系统性能

- **并发连接**: 支持1000+并发客户端连接
- **内存占用**: 服务端内存占用 < 50MB
- **启动时间**: 服务端启动时间 < 3秒
- **响应延迟**: API响应延迟 < 100ms

### 网络性能

- **文件传输**: 支持100MB+文件分块传输
- **实时通信**: WebSocket延迟 < 50ms
- **加密性能**: AES-256-GCM加密/解密 > 100MB/s

### 资源优化

- **二进制大小**: Release版本 < 10MB
- **静态链接**: 支持完全静态链接部署
- **交叉编译**: 支持多平台交叉编译

## 🐛 故障排除

### 日志调试

启用详细日志：

```bash
# 设置日志级别
export RUST_LOG=debug
cargo run --bin server

# 或使用trace级别
export RUST_LOG=trace
cargo run --bin server
```

## 📄 API文档

### RESTful API端点

#### 客户端管理

- `GET /api/clients` - 获取所有客户端列表
- `GET /api/clients/display` - 获取显示用客户端信息
- `DELETE /api/clients/{id}` - 删除指定客户端
- `GET /api/clients/{id}/results` - 获取客户端命令结果

#### 命令执行

- `POST /api/clients/{id}/commands` - 发送命令到客户端
- `POST /api/clients/{id}/reverse_shell` - 启动反弹Shell

#### 反弹Shell管理

- `GET /api/reverse_shells` - 列出活跃的反弹Shell连接
- `POST /api/reverse_shells/{id}/close` - 关闭反弹Shell连接
- `GET /ws/shell/{id}` - WebSocket连接到反弹Shell

#### 文件操作

- `POST /api/files/list` - 列出目录内容
- `POST /api/files/delete` - 删除文件或目录
- `GET /api/files/download/{path}` - 下载文件
- `POST /api/files/upload/{path}` - 上传文件

#### 日志和笔记

- `GET /api/logs` - 获取系统日志
- `POST /api/logs/clear` - 清理日志
- `GET /api/notes` - 获取所有笔记
- `POST /api/notes` - 创建新笔记
- `PUT /api/notes/{id}` - 更新笔记
- `DELETE /api/notes/{id}` - 删除笔记

#### 客户端API端点

- `POST /api/register` - 客户端注册
- `POST /api/heartbeat` - 心跳保持
- `GET /api/commands/{id}` - 获取待执行命令
- `POST /api/command_result` - 提交命令执行结果
- `POST /api/file_operation_response/{id}` - 提交文件操作结果
