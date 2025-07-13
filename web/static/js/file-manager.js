/* File Manager Module */

// Global state
let currentPath = '/';

// Main file operations
async function listDirectory() {
    const path = document.getElementById('currentPath').value.trim();
    if (!path) {
        showNotification('请输入有效路径', 'error');
        return;
    }
    
    currentPath = path;
    document.getElementById('loadingFiles').style.display = 'block';
    document.querySelector('.file-upload-section').style.display = 'none';
    document.querySelector('.file-list-section').style.display = 'none';
    document.getElementById('fileList').innerHTML = '';
    
    try {
        console.log('Sending request to list directory:', path);
        
        const response = await makeApiRequest('/api/files/list', {
            method: 'POST',
            body: JSON.stringify({
                client_id: getClientId(),
                path: path,
                recursive: false
            })
        });
        
        console.log('Response status:', response.status);
        
        if (response.ok) {
            const data = await response.json();
            console.log('Full response data:', data);
            
            if (data.success) {
                renderFileList(data);
            } else {
                console.error('Server returned error:', data.message);
                showNotification(`获取文件列表失败: ${data.message}`, 'error');
                document.getElementById('fileList').innerHTML = '<tr><td colspan="7" class="no-results">获取文件列表失败</td></tr>';
            }
        } else {
            console.error('HTTP error:', response.status);
            const errorText = await response.text();
            console.error('Error response body:', errorText);
            
            try {
                const error = JSON.parse(errorText);
                showNotification(`获取文件列表失败: ${error.message || 'Unknown error'}`, 'error');
            } catch (e) {
                showNotification(`获取文件列表失败: HTTP ${response.status}`, 'error');
            }
            document.getElementById('fileList').innerHTML = '<tr><td colspan="7" class="no-results">获取文件列表失败</td></tr>';
        }
    } catch (error) {
        console.error('Error fetching file list:', error);
        showNotification('获取文件列表时出错: ' + error.message, 'error');
        document.getElementById('fileList').innerHTML = '<tr><td colspan="7" class="no-results">网络连接错误</td></tr>';
    } finally {
        document.getElementById('loadingFiles').style.display = 'none';
        document.querySelector('.file-upload-section').style.display = 'block';
        document.querySelector('.file-list-section').style.display = 'block';
    }
}

// Render file list in table
function renderFileList(serverResponse) {
    const fileList = document.getElementById('fileList');
    fileList.innerHTML = '';

    console.log('Rendering file list with server response:', serverResponse);

    // Extract entries from the server response
    let entries = [];
    if (serverResponse && serverResponse.data && serverResponse.data.entries) {
        entries = serverResponse.data.entries;
    }

    console.log('Extracted entries:', entries);

    if (!Array.isArray(entries) || entries.length === 0) {
        fileList.innerHTML = '<tr><td colspan="7" class="no-results">目录为空</td></tr>';
        return;
    }

    // Sort: directories first, then files, by name
    entries.sort((a, b) => {
        if (a.is_dir !== b.is_dir) {
            return a.is_dir ? -1 : 1;
        }
        return a.name.localeCompare(b.name, 'zh-CN');
    });

    entries.forEach(entry => {
        console.log('Processing entry:', entry);

        const tr = document.createElement('tr');

        // Name column with icon
        const nameCell = document.createElement('td');
        if (entry.is_dir) {
            nameCell.innerHTML = `<span class="file-icon">📁</span>
                                <span class="directory" onclick="navigateToDirectory('${escapeHtml(entry.path)}')">${escapeHtml(entry.name)}</span>`;
        } else {
            nameCell.innerHTML = `<span class="file-icon">📄</span> ${escapeHtml(entry.name)}`;
        }
        nameCell.title = entry.path;

        // Type column
        const typeCell = document.createElement('td');
        typeCell.textContent = entry.is_dir ? '目录' : '文件';

        // Size column
        const sizeCell = document.createElement('td');
        if (entry.is_dir) {
            sizeCell.textContent = '-';
        } else {
            const fileSize = entry.size;
            if (typeof fileSize === 'number') {
                sizeCell.textContent = formatFileSize(fileSize);
            } else {
                sizeCell.textContent = '0 Bytes';
            }
        }

        // Permissions column
        const permissionsCell = document.createElement('td');
        if (entry.permissions) {
            permissionsCell.innerHTML = `<span class="permissions">${escapeHtml(entry.permissions)}</span>`;
        } else {
            permissionsCell.textContent = '-';
        }

        // Owner column
        const ownerCell = document.createElement('td');
        if (entry.owner && entry.group) {
            ownerCell.innerHTML = `<span class="owner-group">${escapeHtml(entry.owner)}:${escapeHtml(entry.group)}</span>`;
        } else if (entry.owner) {
            ownerCell.innerHTML = `<span class="owner-group">${escapeHtml(entry.owner)}</span>`;
        } else {
            ownerCell.innerHTML = '<span class="owner-group">-</span>';
        }

        // Modified time column
        const timeCell = document.createElement('td');
        if (entry.modified) {
            let modifiedDate;
            if (typeof entry.modified === 'object' && entry.modified.tv_sec !== undefined) {
                modifiedDate = new Date(entry.modified.tv_sec * 1000);
            } else if (typeof entry.modified === 'object' && entry.modified.secs_since_epoch !== undefined) {
                modifiedDate = new Date(entry.modified.secs_since_epoch * 1000);
            } else if (typeof entry.modified === 'string') {
                modifiedDate = new Date(entry.modified);
            } else if (typeof entry.modified === 'number') {
                modifiedDate = new Date(entry.modified);
            } else {
                modifiedDate = null;
            }

            if (modifiedDate && !isNaN(modifiedDate.getTime())) {
                timeCell.textContent = modifiedDate.toLocaleString('zh-CN', {
                    year: 'numeric',
                    month: '2-digit',
                    day: '2-digit',
                    hour: '2-digit',
                    minute: '2-digit'
                });
            } else {
                timeCell.textContent = '-';
            }
        } else {
            timeCell.textContent = '-';
        }

        // Actions column
        const actionCell = document.createElement('td');
        actionCell.className = 'file-actions';

        if (entry.is_dir) {
            actionCell.innerHTML = `
                <button class="btn btn-sm" onclick="navigateToDirectory('${escapeHtml(entry.path)}')">打开</button>
                <button class="btn btn-sm btn-danger" onclick="deletePath('${escapeHtml(entry.path)}')">删除</button>
            `;
        } else {
            actionCell.innerHTML = `
                <a href="/api/files/download/${encodeURIComponent(entry.path)}?client_id=${getClientId()}" class="btn btn-sm">下载</a>
                <button class="btn btn-sm btn-danger" onclick="deletePath('${escapeHtml(entry.path)}')">删除</button>
            `;
        }
        
        tr.appendChild(nameCell);
        tr.appendChild(typeCell);
        tr.appendChild(sizeCell);
        tr.appendChild(permissionsCell);
        tr.appendChild(ownerCell);
        tr.appendChild(timeCell);
        tr.appendChild(actionCell);
        
        fileList.appendChild(tr);
    });
}

// Navigation functions
function navigateToDirectory(path) {
    document.getElementById('currentPath').value = path;
    listDirectory();
}

function goToParentDirectory() {
    const currentPath = document.getElementById('currentPath').value.trim();
    if (currentPath === '/' || currentPath === '') {
        return;
    }
    
    const pathParts = currentPath.split('/').filter(Boolean);
    pathParts.pop();
    const parentPath = pathParts.length === 0 ? '/' : '/' + pathParts.join('/');
    
    document.getElementById('currentPath').value = parentPath;
    listDirectory();
}

// File operations
async function uploadFile() {
    const fileInput = document.getElementById('fileUpload');
    const currentPath = document.getElementById('currentPath').value.trim();
    
    if (!fileInput.files || fileInput.files.length === 0) {
        showNotification('请选择要上传的文件', 'error');
        return;
    }
    
    const file = fileInput.files[0];
    const targetPath = `${currentPath}/${file.name}`.replace(/\/\//g, '/');
    
    try {
        const response = await fetch(`/api/files/upload/${encodeURIComponent(targetPath)}?client_id=${getClientId()}`, {
            method: 'POST',
            credentials: 'same-origin',
            body: file
        });
        
        if (response.ok) {
            const result = await response.json();
            if (result.success) {
                showNotification('文件上传成功', 'success');
                fileInput.value = '';
                document.getElementById('fileNameDisplay').textContent = '未选择任何文件';
                listDirectory();
            } else {
                showNotification(`上传失败: ${result.message}`, 'error');
            }
        } else {
            const errorText = await response.text();
            try {
                const error = JSON.parse(errorText);
                showNotification(`上传失败: ${error.message || 'Unknown error'}`, 'error');
            } catch (e) {
                showNotification(`上传失败: HTTP ${response.status}`, 'error');
            }
        }
    } catch (error) {
        showNotification('上传时出错: ' + error.message, 'error');
    }
}

async function deletePath(path) {
    if (!confirm(`确定要删除 ${path} 吗？`)) {
        return;
    }
    
    try {
        const response = await makeApiRequest('/api/files/delete', {
            method: 'POST',
            body: JSON.stringify({
                client_id: getClientId(),
                path: path
            })
        });
        
        if (response.ok) {
            const result = await response.json();
            if (result.success) {
                showNotification('删除成功', 'success');
                listDirectory();
            } else {
                showNotification(`删除失败: ${result.message}`, 'error');
            }
        } else {
            const errorText = await response.text();
            try {
                const error = JSON.parse(errorText);
                showNotification(`删除失败: ${error.message || 'Unknown error'}`, 'error');
            } catch (e) {
                showNotification(`删除失败: HTTP ${response.status}`, 'error');
            }
        }
    } catch (error) {
        showNotification('删除时出错: ' + error.message, 'error');
    }
}

// File input handlers
function updateFileName(input) {
    const fileNameDisplay = document.getElementById('fileNameDisplay');
    if (input.files && input.files.length > 0) {
        fileNameDisplay.textContent = input.files[0].name;
    } else {
        fileNameDisplay.textContent = '未选择任何文件';
    }
}

// Make updateFileName globally available
window.updateFileName = updateFileName;
window.listDirectory = listDirectory;
window.uploadFile = uploadFile;
window.goToParentDirectory = goToParentDirectory;

// Helper function to get client ID from template
function getClientId() {
    return document.body.dataset.clientId || '';
}

// Keyboard event handlers
document.addEventListener('DOMContentLoaded', function() {
    const currentPathInput = document.getElementById('currentPath');
    if (currentPathInput) {
        currentPathInput.addEventListener('keypress', function(e) {
            if (e.key === 'Enter') {
                listDirectory();
            }
        });
    }
});