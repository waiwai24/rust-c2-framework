/* Dashboard Management Functions */

// Dashboard data refresh
async function refreshClientList() {
    if (isUpdating) return;
    isUpdating = true;
    
    try {
        const response = await makeApiRequest('/api/clients/display');
        if (response.ok) {
            const data = await response.json();
            updateDashboard(data);
        }
    } catch (error) {
        console.error('刷新失败:', error);
        showNotification('刷新客户端列表失败', 'error');
    } finally {
        isUpdating = false;
    }
}

// Update dashboard data
function updateDashboard(data) {
    // Update statistics
    const statCards = document.querySelectorAll('.stat-card h3');
    if (statCards.length >= 3) {
        statCards[0].textContent = data.total_clients;
        statCards[1].textContent = data.online_clients_count;
        statCards[2].textContent = data.os_types_count;
    }
    
    // Update client table
    updateClientTable(data.clients);
}

// Update client table
function updateClientTable(clients) {
    const clientsSection = document.querySelector('.clients-section');
    let tbody = document.querySelector('.clients-table tbody');
    let table = document.querySelector('.clients-table');
    let noClientsDiv = document.querySelector('.no-clients');
    
    if (clients.length === 0) {
        // Hide table
        if (table) table.style.display = 'none';
        
        // Create or show no clients message
        if (!noClientsDiv) {
            noClientsDiv = document.createElement('div');
            noClientsDiv.className = 'no-clients';
            noClientsDiv.innerHTML = '<h3>暂无客户端连接</h3><p>等待客户端连接到服务器...</p>';
            clientsSection.appendChild(noClientsDiv);
        }
        noClientsDiv.style.display = 'block';
    } else {
        // Hide no clients message
        if (noClientsDiv) noClientsDiv.style.display = 'none';
        
        // Create table if it doesn't exist
        if (!table) {
            table = document.createElement('table');
            table.className = 'clients-table';
            table.innerHTML = `
                <thead>
                    <tr>
                        <th>客户端ID</th>
                        <th>主机名</th>
                        <th>用户名</th>
                        <th>操作系统</th>
                        <th>IP地址</th>
                        <th>连接时间</th>
                        <th>最后活动</th>
                        <th>状态</th>
                        <th>操作</th>
                    </tr>
                </thead>
                <tbody></tbody>
            `;
            clientsSection.appendChild(table);
            tbody = table.querySelector('tbody');
        }
        
        // Show table
        table.style.display = 'table';
        
        // Update table content
        if (tbody) {
            tbody.innerHTML = clients.map(client => `
                <tr>
                    <td><code>${client.id.substring(0, 8)}...</code></td>
                    <td>${client.hostname}</td>
                    <td>${client.username}</td>
                    <td>${client.os} (${client.arch})</td>
                    <td>${client.ip}</td>
                    <td>${formatDateTime(client.connected_at)}</td>
                    <td>${formatDateTime(client.last_seen)}</td>
                    <td>
                        <span class="status ${client.is_online ? 'online' : 'offline'}">
                            ${client.is_online ? '在线' : '离线'}
                        </span>
                    </td>
                    <td>
                        <a href="/client/${client.id}" class="btn btn-sm">管理</a>
                        <button class="btn btn-danger btn-sm" onclick="deleteClient('${client.id}', '${client.hostname}')">删除</button>
                    </td>
                </tr>
            `).join('');
        }
    }
}

// Delete client function
async function deleteClient(clientId, hostname) {
    if (confirm(`确定要删除客户端 "${hostname}" 吗？\n\n此操作将:\n- 删除客户端记录\n- 清除相关命令历史\n- 清除文件操作记录\n\n此操作不可撤销！`)) {
        try {
            const response = await makeApiRequest(`/api/clients/${clientId}`, {
                method: 'DELETE'
            });
            
            if (response.ok) {
                showNotification('客户端删除成功', 'success');
                await refreshClientList();
            } else if (response.status === 404) {
                showNotification('客户端不存在', 'error');
                await refreshClientList();
            } else {
                showNotification(`删除客户端失败: HTTP ${response.status}`, 'error');
            }
        } catch (error) {
            console.error('删除客户端时出错:', error);
            showNotification('删除客户端时出错: ' + error.message, 'error');
        }
    }
}

// Set up automatic refresh
function setupAutoRefresh() {
    const refreshInterval = parseInt(document.body.dataset.refreshInterval || "5") || 5;
    
    setTimeout(() => {
        setInterval(() => {
            // Only refresh when clients tab is active
            if (document.querySelector('.tab-item[data-tab="clients"]')?.classList.contains('active')) {
                refreshClientList();
            }
        }, refreshInterval * 1000);
    }, 1000);
}

// Initialize dashboard
document.addEventListener('DOMContentLoaded', setupAutoRefresh);