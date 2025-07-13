/* Settings Management Module */

// Load settings from localStorage
function loadSettings() {
    // Load settings from localStorage or use defaults
    const settings = JSON.parse(localStorage.getItem('c2-settings') || '{}');
    
    if (settings.refreshInterval) {
        document.getElementById('refreshInterval').value = settings.refreshInterval;
    }
    
    // Load other settings if available
    if (settings.serverHost) {
        document.getElementById('serverHost').value = settings.serverHost;
    }
    if (settings.serverPort) {
        document.getElementById('serverPort').value = settings.serverPort;
    }
    if (settings.clientTimeout) {
        document.getElementById('clientTimeout').value = settings.clientTimeout;
    }
    if (settings.maxClients) {
        document.getElementById('maxClients').value = settings.maxClients;
    }
    if (settings.enableAuth !== undefined) {
        document.getElementById('enableAuth').checked = settings.enableAuth;
    }
    if (settings.enableAudit !== undefined) {
        document.getElementById('enableAudit').checked = settings.enableAudit;
    }
}

// Save settings to localStorage
function saveSettings() {
    const settings = {
        serverHost: document.getElementById('serverHost').value,
        serverPort: document.getElementById('serverPort').value,
        clientTimeout: document.getElementById('clientTimeout').value,
        refreshInterval: document.getElementById('refreshInterval').value,
        maxClients: document.getElementById('maxClients').value,
        enableAuth: document.getElementById('enableAuth').checked,
        enableAudit: document.getElementById('enableAudit').checked
    };
    
    localStorage.setItem('c2-settings', JSON.stringify(settings));
    showNotification('设置已保存到本地存储', 'success');
}

// Reset settings to defaults
function resetSettings() {
    if (confirm('确定要重置为默认设置吗？')) {
        localStorage.removeItem('c2-settings');
        showNotification('设置已重置', 'success');
        setTimeout(() => {
            location.reload();
        }, 1000);
    }
}

// Restart server (placeholder - needs backend API support)
function restartServer() {
    if (confirm('确定要重启服务器吗？这将断开所有客户端连接。')) {
        showNotification('重启服务器功能需要后端API支持', 'error');
    }
}