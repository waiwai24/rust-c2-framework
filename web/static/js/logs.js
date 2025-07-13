/* Logs Management Module */

// Load and display logs
async function loadLogs() {
    const logsList = document.getElementById('logsList');
    logsList.innerHTML = '<div class="no-results"><p>加载日志中...</p></div>';
    
    try {
        const response = await makeApiRequest('/api/logs', {
            method: 'GET'
        });
        
        if (response.ok) {
            const logs = await response.text();
            displayLogs(logs);
        } else {
            logsList.innerHTML = '<div class="no-results"><p>无法加载日志文件</p></div>';
        }
    } catch (error) {
        console.error('加载日志失败:', error);
        logsList.innerHTML = '<div class="no-results"><p>加载日志时出错</p></div>';
    }
}

// Display logs with formatting and stats
function displayLogs(logsText) {
    const logsList = document.getElementById('logsList');
    const lines = logsText.split('\n').filter(line => line.trim());
    
    if (lines.length === 0) {
        logsList.innerHTML = '<div class="no-results"><p>暂无日志记录</p></div>';
        updateLogStats([]);
        populateTimeSelectors([]);
        return;
    }
    
    // Parse log entries and sort by time (newest first)
    const logEntries = lines.map(line => {
        const timestampMatch = line.match(/\[(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})\]/);
        return {
            text: line,
            timestamp: timestampMatch ? new Date(timestampMatch[1]) : new Date(0),
            timestampStr: timestampMatch ? timestampMatch[1] : null,
            class: getLogClass(line)
        };
    }).sort((a, b) => b.timestamp - a.timestamp);
    
    const logsHtml = logEntries.map(entry => `
        <div class="log-entry ${entry.class}" title="点击复制到剪贴板" onclick="copyLogEntry(this)">
            ${escapeHtml(entry.text)}
        </div>
    `).join('');
    
    logsList.innerHTML = logsHtml;
    
    // Display log statistics
    updateLogStats(logEntries);
    
    // Populate time selectors
    populateTimeSelectors(logEntries);
}

// Populate time range selectors
function populateTimeSelectors(logEntries) {
    const startTimeSelect = document.getElementById('startTime');
    const endTimeSelect = document.getElementById('endTime');
    
    // Save current selections
    const currentStartTime = startTimeSelect.value;
    const currentEndTime = endTimeSelect.value;
    
    // Clear selectors
    startTimeSelect.innerHTML = '<option value="">请选择开始时间</option>';
    endTimeSelect.innerHTML = '<option value="">请选择结束时间</option>';
    
    if (logEntries.length === 0) return;
    
    // Extract all valid timestamps
    const timestamps = logEntries
        .filter(entry => entry.timestampStr)
        .map(entry => ({
            value: entry.timestampStr,
            display: formatTimestampForDisplay(entry.timestampStr),
            date: entry.timestamp
        }));
    
    // Remove duplicates and sort
    const uniqueTimestamps = timestamps.filter((timestamp, index, arr) => 
        arr.findIndex(t => t.value === timestamp.value) === index
    );
    
    // Sort for start time (earliest first)
    const startTimestamps = [...uniqueTimestamps].sort((a, b) => a.date - b.date);
    // Sort for end time (latest first)
    const endTimestamps = [...uniqueTimestamps].sort((a, b) => b.date - a.date);
    
    // Populate start time selector
    startTimestamps.forEach(timestamp => {
        const option = document.createElement('option');
        option.value = timestamp.value;
        option.textContent = timestamp.display;
        startTimeSelect.appendChild(option);
    });
    
    // Populate end time selector
    endTimestamps.forEach(timestamp => {
        const option = document.createElement('option');
        option.value = timestamp.value;
        option.textContent = timestamp.display;
        endTimeSelect.appendChild(option);
    });
    
    // Restore previous selections
    if (currentStartTime) startTimeSelect.value = currentStartTime;
    if (currentEndTime) endTimeSelect.value = currentEndTime;
}

// Update log statistics display
function updateLogStats(logEntries) {
    const stats = {
        total: logEntries.length,
        client: logEntries.filter(e => e.text.includes('CLIENT_')).length,
        command: logEntries.filter(e => e.text.includes('COMMAND_')).length,
        auth: logEntries.filter(e => e.text.includes('AUTH_')).length,
        session: logEntries.filter(e => e.text.includes('SESSION_')).length,
        file: logEntries.filter(e => e.text.includes('FILE_')).length,
        shell: logEntries.filter(e => e.text.includes('SHELL_SESSION')).length,
        websocket: logEntries.filter(e => e.text.includes('WEBSOCKET_')).length,
        error: logEntries.filter(e => e.text.includes('ERROR')).length
    };
    
    // Display statistics outside logs container in fixed position
    let statsElement = document.getElementById('logStats');
    if (!statsElement) {
        statsElement = document.createElement('div');
        statsElement.id = 'logStats';
        statsElement.className = 'log-stats';
        // Insert before logs-container, not inside
        const logsContainer = document.querySelector('.logs-container');
        logsContainer.parentNode.insertBefore(statsElement, logsContainer);
    }
    
    statsElement.innerHTML = `
        <div class="stats-summary">
            <span class="stat-item">总计: <strong>${stats.total}</strong></span>
            <span class="stat-item stat-client">客户端: <strong>${stats.client}</strong></span>
            <span class="stat-item stat-command">命令: <strong>${stats.command}</strong></span>
            <span class="stat-item stat-auth">认证: <strong>${stats.auth}</strong></span>
            <span class="stat-item stat-session">会话: <strong>${stats.session}</strong></span>
            <span class="stat-item stat-file">文件: <strong>${stats.file}</strong></span>
            <span class="stat-item stat-shell">Shell: <strong>${stats.shell}</strong></span>
            <span class="stat-item stat-websocket">WebSocket: <strong>${stats.websocket}</strong></span>
            <span class="stat-item stat-error">错误: <strong>${stats.error}</strong></span>
        </div>
    `;
}

// Get CSS class for log line based on content
function getLogClass(logLine) {
    // Client related
    if (logLine.includes('CLIENT_CONNECT')) return 'log-client_connect';
    if (logLine.includes('CLIENT_DISCONNECT')) return 'log-client_disconnect';
    if (logLine.includes('CLIENT_DELETE')) return 'log-client_delete';
    if (logLine.includes('CLIENT_REGISTER')) return 'log-client_register';
    if (logLine.includes('CLIENT_TIMEOUT')) return 'log-client_timeout';
    
    // Command related
    if (logLine.includes('COMMAND_EXECUTE')) return 'log-command_execute';
    if (logLine.includes('COMMAND_RESULT')) return 'log-command_result';
    
    // Authentication related
    if (logLine.includes('AUTH_FAILURE')) return 'log-auth_failure';
    
    // Session related
    if (logLine.includes('SESSION_CREATE')) return 'log-session_create';
    if (logLine.includes('SESSION_LOGIN')) return 'log-session_login';
    if (logLine.includes('SESSION_LOGOUT')) return 'log-session_logout';
    if (logLine.includes('SESSION_VALIDATE')) return 'log-session_validate';
    
    // File operations
    if (logLine.includes('FILE_UPLOAD')) return 'log-file_upload';
    if (logLine.includes('FILE_DOWNLOAD')) return 'log-file_download';
    if (logLine.includes('FILE_DELETE')) return 'log-file_delete';
    if (logLine.includes('FILE_LIST')) return 'log-file_list';
    
    // Shell sessions
    if (logLine.includes('SHELL_SESSION')) return 'log-shell_session';
    
    // WebSocket related
    if (logLine.includes('WEBSOCKET_CONNECT')) return 'log-websocket_connect';
    if (logLine.includes('WEBSOCKET_DISCONNECT')) return 'log-websocket_disconnect';
    if (logLine.includes('WEBSOCKET_MESSAGE')) return 'log-websocket_message';
    if (logLine.includes('WEBSOCKET_ERROR')) return 'log-websocket_error';
    
    // General categories (backward compatibility)
    if (logLine.includes('CLIENT_')) return 'log-client';
    if (logLine.includes('COMMAND_')) return 'log-command';
    if (logLine.includes('AUTH_')) return 'log-auth';
    if (logLine.includes('SESSION_')) return 'log-session';
    if (logLine.includes('FILE_')) return 'log-file';
    if (logLine.includes('WEBSOCKET_')) return 'log-websocket';
    
    // Errors
    if (logLine.includes('ERROR')) return 'log-error';
    
    return 'log-info';
}

// Copy log entry to clipboard
function copyLogEntry(element) {
    const text = element.textContent;
    navigator.clipboard.writeText(text).then(() => {
        // Show copy success feedback
        const originalBg = element.style.backgroundColor;
        element.style.backgroundColor = 'rgba(56, 139, 253, 0.3)';
        setTimeout(() => {
            element.style.backgroundColor = originalBg;
        }, 300);
    }).catch(err => {
        console.error('复制失败:', err);
    });
}

// Refresh logs
async function refreshLogs() {
    await loadLogs();
}

// Filter logs based on level and time range
function filterLogs() {
    const level = document.getElementById('logLevel').value;
    const startTime = document.getElementById('startTime').value;
    const endTime = document.getElementById('endTime').value;
    const logEntries = document.querySelectorAll('.log-entry');
    
    // Convert times to Date objects
    const startDate = startTime ? new Date(startTime) : null;
    const endDate = endTime ? new Date(endTime) : null;
    
    logEntries.forEach(entry => {
        let show = true;
        const logText = entry.textContent;
        
        // Filter by log level/type
        if (level !== 'all' && !logText.includes(level)) {
            show = false;
        }
        
        // Filter by time range - handle log format [2025-07-12 01:39:48]
        if (startDate || endDate) {
            const logDateMatch = logText.match(/\[(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})\]/);
            if (logDateMatch) {
                const logTime = new Date(logDateMatch[1]);
                
                // Check if within time range
                if (startDate && logTime < startDate) {
                    show = false;
                }
                if (endDate && logTime > endDate) {
                    show = false;
                }
            } else {
                // If time cannot be parsed and time filter is set, hide
                if (startDate || endDate) {
                    show = false;
                }
            }
        }
        
        entry.style.display = show ? 'block' : 'none';
    });
    
    // Show filter result statistics
    const visibleEntries = Array.from(logEntries).filter(entry => entry.style.display !== 'none');
    const totalEntries = logEntries.length;
    
    // Display result statistics in filter area
    let statusElement = document.getElementById('filterStatus');
    if (!statusElement) {
        statusElement = document.createElement('span');
        statusElement.id = 'filterStatus';
        statusElement.style.color = '#8b949e';
        statusElement.style.fontSize = '0.85em';
        statusElement.style.marginLeft = 'auto';
        document.querySelector('.logs-filters').appendChild(statusElement);
    }
    
    if (level !== 'all' || startTime || endTime) {
        const timeRangeText = getTimeRangeText(startTime, endTime);
        statusElement.textContent = `显示 ${visibleEntries.length} / ${totalEntries} 条日志${timeRangeText}`;
    } else {
        statusElement.textContent = '';
    }
}

// Get time range text for display
function getTimeRangeText(startTime, endTime) {
    if (!startTime && !endTime) return '';
    
    const formatDisplayTime = (timeStr) => {
        const date = new Date(timeStr);
        return date.toLocaleString('zh-CN', {
            month: '2-digit',
            day: '2-digit',
            hour: '2-digit',
            minute: '2-digit'
        });
    };
    
    if (startTime && endTime) {
        return ` (${formatDisplayTime(startTime)} ~ ${formatDisplayTime(endTime)})`;
    } else if (startTime) {
        return ` (${formatDisplayTime(startTime)} 之后)`;
    } else {
        return ` (${formatDisplayTime(endTime)} 之前)`;
    }
}

// Set quick time range
function setTimeRange(range) {
    // Quick time range functionality now only clears selectors, user needs to manually select
    clearFilters();
    
    // Show hint information
    const statusElement = document.getElementById('filterStatus');
    if (statusElement) {
        switch (range) {
            case 'last-hour':
                statusElement.textContent = '请从下拉列表中选择最近1小时的时间范围';
                break;
            case 'last-day':
                statusElement.textContent = '请从下拉列表中选择最近24小时的时间范围';
                break;
            case 'today':
                statusElement.textContent = '请从下拉列表中选择今天的时间范围';
                break;
            case 'yesterday':
                statusElement.textContent = '请从下拉列表中选择昨天的时间范围';
                break;
        }
        
        // Clear hint after 3 seconds
        setTimeout(() => {
            if (statusElement.textContent.includes('请从下拉列表中选择')) {
                statusElement.textContent = '';
            }
        }, 3000);
    }
}

// Clear all filters
function clearFilters() {
    // Reset filter controls
    document.getElementById('logLevel').value = 'all';
    document.getElementById('startTime').value = '';
    document.getElementById('endTime').value = '';
    
    // Show all log entries
    const logEntries = document.querySelectorAll('.log-entry');
    logEntries.forEach(entry => {
        entry.style.display = 'block';
    });
    
    // Clear filter status display
    const statusElement = document.getElementById('filterStatus');
    if (statusElement) {
        statusElement.textContent = '';
    }
}