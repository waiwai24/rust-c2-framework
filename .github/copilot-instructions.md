<!-- Use this file to provide workspace-specific custom instructions to Copilot. For more details, visit https://code.visualstudio.com/docs/copilot/copilot-customization#_use-a-githubcopilotinstructionsmd-file -->

# Rust C2 Framework Project

这是一个使用Rust编写的C2框架项目，包含以下组件：

## 项目结构
- `client/` - 客户端代码（在被害者主机上运行）
- `server/` - 服务端代码（运行在服务器上）
- `web/` - Web前端界面（用于控制台）
- `common/` - 共享代码和协议定义

## 主要功能
- 反弹Shell连接
- 远程命令执行
- Web界面控制台
- 客户端与服务端通信

## 编程规范
- 使用异步编程模型（tokio）
- 遵循Rust最佳实践
- 注重安全性和错误处理
- 使用适当的加密和认证机制

## 依赖管理
- 使用cargo进行依赖管理
- 重点关注网络编程、Web框架和加密库
