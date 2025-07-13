/* Reverse Shell Module */

// Global WebSocket and connection state
let shellWebSocket = null;
let currentShellConnectionId = null;

// Start reverse shell connection
async function startReverseShell() {
    if (confirm('确定要启动反弹Shell吗？')) {
        try {
            const response = await makeApiRequest(`/api/clients/${getClientId()}/reverse_shell`, {
                method: 'POST',
                body: JSON.stringify({})
            });

            if (response.ok) {
                const data = await response.json();
                
                showNotification('反弹Shell请求已发送', 'success');
                document.getElementById('reverseShellStatus').textContent = '请求已发送，正在等待连接...';
                document.getElementById('reverseShellSessionId').textContent = '等待连接';
                document.getElementById('connectionStatus').className = 'status-indicator connecting';
                document.getElementById('connectionStatusDot').className = 'status-dot connecting';
                document.getElementById('connectionStatusText').textContent = '连接中';
                
                // Wait for client to establish connection
                setTimeout(async () => {
                    await checkForReverseShellConnections();
                }, 3000);

            } else {
                showNotification('发送反弹Shell请求失败', 'error');
                document.getElementById('reverseShellStatus').textContent = '启动失败';
                document.getElementById('reverseShellSessionId').textContent = '无';
                document.getElementById('connectionStatus').className = 'status-indicator';
                document.getElementById('connectionStatusDot').className = 'status-dot';
                document.getElementById('connectionStatusText').textContent = '启动失败';
            }
        } catch (error) {
            showNotification('发送请求时出错: ' + error.message, 'error');
            document.getElementById('reverseShellStatus').textContent = '启动失败: ' + error.message;
            document.getElementById('reverseShellSessionId').textContent = '无';
            document.getElementById('connectionStatus').className = 'status-indicator';
            document.getElementById('connectionStatusDot').className = 'status-dot';
            document.getElementById('connectionStatusText').textContent = '启动失败';
        }
    }
}

// Check for reverse shell connections
async function checkForReverseShellConnections() {
    try {
        const response = await makeApiRequest('/api/reverse_shells', {
            method: 'GET'
        });

        if (response.ok) {
            const data = await response.json();
            if (data.connections && data.connections.length > 0) {
                // Use the latest connection
                const connectionId = data.connections[data.connections.length - 1];
                document.getElementById('reverseShellSessionId').textContent = connectionId;
                
                // Establish WebSocket connection
                connectWebSocket(connectionId);
            } else {
                document.getElementById('reverseShellStatus').textContent = '等待连接建立...';
                // Continue polling
                setTimeout(async () => {
                    await checkForReverseShellConnections();
                }, 2000);
            }
        } else {
            document.getElementById('reverseShellStatus').textContent = '获取连接列表失败';
        }
    } catch (error) {
        console.error('检查连接失败:', error);
        document.getElementById('reverseShellStatus').textContent = '检查连接失败';
    }
}

// Establish WebSocket connection for shell
function connectWebSocket(connectionId) {
    // Save current connection ID
    currentShellConnectionId = connectionId;
    
    if (shellWebSocket) {
        shellWebSocket.close();
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws/shell/${connectionId}`;
    shellWebSocket = new WebSocket(wsUrl);

    const shellOutput = document.getElementById('shellOutput');
    const shellInput = document.getElementById('shellInput');
    const shellInputLine = document.getElementById('shellInputLine');
    const shellTerminal = document.getElementById('shellTerminal');

    // Command history
    let commandHistory = [];
    let historyIndex = -1;

    shellWebSocket.onopen = () => {
        document.getElementById('reverseShellStatus').textContent = '已连接';
        document.getElementById('connectionStatus').className = 'status-indicator connected';
        document.getElementById('connectionStatusDot').className = 'status-dot connected';
        document.getElementById('connectionStatusText').textContent = '已连接';
        
        // Switch button states
        document.getElementById('startShellBtn').style.display = 'none';
        document.getElementById('stopShellBtn').style.display = 'flex';
        
        // Clear output and show input line at top
        shellOutput.textContent = '';
        shellInputLine.style.display = 'flex';
        shellInput.focus();
        
        // Scroll to top
        shellTerminal.scrollTop = 0;
        
        // Focus terminal
        shellTerminal.focus();
    };

    shellWebSocket.onmessage = (event) => {
        const data = event.data;
        shellOutput.textContent += data;
        scrollToBottom();
        
        // Show input line after receiving data
        if (!shellInputLine.style.display || shellInputLine.style.display === 'none') {
            setTimeout(() => {
                shellInputLine.style.display = 'flex';
                shellInput.focus();
            }, 100);
        }
    };

    shellWebSocket.onclose = () => {
        document.getElementById('reverseShellStatus').textContent = '已断开';
        document.getElementById('connectionStatus').className = 'status-indicator';
        document.getElementById('connectionStatusDot').className = 'status-dot';
        document.getElementById('connectionStatusText').textContent = '已断开';
        
        // Reset button states
        document.getElementById('startShellBtn').style.display = 'flex';
        document.getElementById('stopShellBtn').style.display = 'none';
        currentShellConnectionId = null;
        
        shellInputLine.style.display = 'none';
        const shellWelcome = document.getElementById('shellWelcome');
        if (shellWelcome) {
            shellWelcome.style.display = 'none';
        }
        
        const disconnectMsg = document.createElement('div');
        disconnectMsg.className = 'shell-error';
        disconnectMsg.textContent = 'Shell连接已断开。';
        shellOutput.appendChild(disconnectMsg);
        scrollToBottom();
    };

    shellWebSocket.onerror = (error) => {
        console.error('WebSocket Error:', error);
        document.getElementById('reverseShellStatus').textContent = '连接错误';
        document.getElementById('connectionStatus').className = 'status-indicator';
        document.getElementById('connectionStatusDot').className = 'status-dot';
        document.getElementById('connectionStatusText').textContent = '连接错误';
        
        // Reset button states
        document.getElementById('startShellBtn').style.display = 'flex';
        document.getElementById('stopShellBtn').style.display = 'none';
        currentShellConnectionId = null;
        
        const errorMsg = document.createElement('div');
        errorMsg.className = 'shell-error';
        errorMsg.textContent = `连接错误: ${error.message || '未知错误'}`;
        shellOutput.appendChild(errorMsg);
        scrollToBottom();
    };

    // Keyboard event handling
    function handleKeyDown(e) {
        if (!shellWebSocket || shellWebSocket.readyState !== WebSocket.OPEN) {
            return;
        }

        switch (e.key) {
            case 'Enter':
                e.preventDefault();
                const command = shellInput.textContent;
                if (command.trim()) {
                    // Add to history
                    commandHistory.push(command);
                    historyIndex = commandHistory.length;
                    
                    // Send command
                    shellWebSocket.send(command + '\n');
                    
                    // Show command in output
                    shellOutput.textContent += command + '\n';
                    
                    // Clear input
                    shellInput.textContent = '';
                    
                    // Hide input line until response
                    shellInputLine.style.display = 'none';
                }
                break;
                
            case 'ArrowUp':
                e.preventDefault();
                if (historyIndex > 0) {
                    historyIndex--;
                    shellInput.textContent = commandHistory[historyIndex];
                    moveCaretToEnd(shellInput);
                }
                break;
                
            case 'ArrowDown':
                e.preventDefault();
                if (historyIndex < commandHistory.length - 1) {
                    historyIndex++;
                    shellInput.textContent = commandHistory[historyIndex];
                    moveCaretToEnd(shellInput);
                } else {
                    historyIndex = commandHistory.length;
                    shellInput.textContent = '';
                }
                break;
                
            case 'Tab':
                e.preventDefault();
                // Tab completion could be implemented here
                break;
                
            case 'c':
                if (e.ctrlKey) {
                    e.preventDefault();
                    // Ctrl+C sends interrupt signal
                    shellWebSocket.send('\x03');
                    shellOutput.textContent += '^C\n$ ';
                    shellInput.textContent = '';
                    shellInputLine.style.display = 'flex';
                }
                break;
        }
    }

    function moveCaretToEnd(el) {
        if (typeof window.getSelection != "undefined" && typeof document.createRange != "undefined") {
            var range = document.createRange();
            range.selectNodeContents(el);
            range.collapse(false);
            var sel = window.getSelection();
            sel.removeAllRanges();
            sel.addRange(range);
        }
    }

    function scrollToBottom() {
        // Only scroll to bottom when there is actual output content
        if (shellOutput.textContent.trim().length > 0) {
            shellTerminal.scrollTop = shellTerminal.scrollHeight;
        }
    }

    // Add event listeners
    shellInput.addEventListener('keydown', handleKeyDown);
    shellTerminal.addEventListener('click', () => {
        shellInput.focus();
    });
    
    // Keep focus
    setInterval(() => {
        if (shellInputLine.style.display !== 'none' && document.activeElement !== shellInput) {
            shellInput.focus();
        }
    }, 100);
}

// Stop reverse shell connection
async function stopReverseShell() {
    if (!currentShellConnectionId) {
        showNotification('没有活动的Shell连接', 'error');
        return;
    }

    if (confirm('确定要关闭当前的Shell连接吗？')) {
        try {
            const response = await makeApiRequest(`/api/reverse_shells/${currentShellConnectionId}/close`, {
                method: 'POST'
            });

            if (response.ok) {
                const data = await response.json();
                showNotification(data.message, 'success');
                
                // Close WebSocket connection
                if (shellWebSocket) {
                    shellWebSocket.close();
                    shellWebSocket = null;
                }
                
                // Reset UI state
                resetShellUI();
                currentShellConnectionId = null;
                
            } else {
                const errorText = await response.text();
                showNotification('关闭Shell连接失败: ' + errorText, 'error');
            }
        } catch (error) {
            showNotification('关闭Shell连接时出错: ' + error.message, 'error');
        }
    }
}

// Reset shell UI to initial state
function resetShellUI() {
    // Reset button states
    document.getElementById('startShellBtn').style.display = 'flex';
    document.getElementById('stopShellBtn').style.display = 'none';
    
    // Reset status display
    document.getElementById('reverseShellStatus').textContent = '未启动';
    document.getElementById('reverseShellSessionId').textContent = '无';
    document.getElementById('connectionStatus').className = 'status-indicator';
    document.getElementById('connectionStatusDot').className = 'status-dot';
    document.getElementById('connectionStatusText').textContent = '未连接';
    
    // Clear terminal content
    const shellOutput = document.getElementById('shellOutput');
    if (shellOutput) {
        shellOutput.textContent = '';
    }
    
    // Hide input line
    const shellInputLine = document.getElementById('shellInputLine');
    if (shellInputLine) {
        shellInputLine.style.display = 'none';
    }
}

// Helper function to get client ID
function getClientId() {
    return document.body.dataset.clientId || '';
}

// Make functions globally available
window.startReverseShell = startReverseShell;
window.stopReverseShell = stopReverseShell;