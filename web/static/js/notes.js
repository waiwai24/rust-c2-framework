/* Notes Management Module */

// Load and display notes
async function loadNotes() {
    const notesList = document.getElementById('notesList');
    
    try {
        const response = await makeApiRequest('/api/notes', {
            method: 'GET'
        });
        
        if (response.ok) {
            notes = await response.json();
        } else {
            console.error('Failed to load notes:', response.status);
            notes = [];
        }
    } catch (error) {
        console.error('Error loading notes:', error);
        notes = [];
    }
    
    if (notes.length === 0) {
        notesList.innerHTML = `
            <div class="no-results">
                <h3>暂无备忘录</h3>
                <p>点击"添加备忘录"按钮创建第一个备忘录</p>
            </div>
        `;
        return;
    }
    
    const notesHtml = notes.map((note, index) => `
        <div class="note-item">
            <div class="note-header">
                <h4>${escapeHtml(note.title)}</h4>
                <div class="note-actions">
                    <button class="btn btn-sm" onclick="editNote('${note.id}')">编辑</button>
                    <button class="btn btn-sm btn-danger" onclick="deleteNote('${note.id}')">删除</button>
                </div>
            </div>
            <div class="note-content">${escapeHtml(note.content)}</div>
            <div class="note-meta">创建时间: ${note.created_at}${note.updated_at ? ' | 更新时间: ' + note.updated_at : ''}</div>
        </div>
    `).join('');
    
    notesList.innerHTML = notesHtml;
}

// Add new note
function addNote() {
    document.getElementById('noteForm').style.display = 'block';
    document.getElementById('noteTitle').value = '';
    document.getElementById('noteContent').value = '';
    document.getElementById('noteTitle').focus();
}

// Save note
async function saveNote() {
    const title = document.getElementById('noteTitle').value.trim();
    const content = document.getElementById('noteContent').value.trim();
    
    if (!title || !content) {
        showNotification('请填写标题和内容', 'error');
        return;
    }
    
    const note = {
        id: '',
        title: title,
        content: content,
        created_at: '',
        updated_at: null
    };
    
    try {
        const response = await makeApiRequest('/api/notes', {
            method: 'POST',
            body: JSON.stringify(note)
        });
        
        if (response.ok) {
            cancelNote();
            await loadNotes();
            showNotification('备忘录保存成功', 'success');
        } else {
            showNotification('保存备忘录失败: ' + response.status, 'error');
        }
    } catch (error) {
        console.error('Error saving note:', error);
        showNotification('保存备忘录时出错: ' + error.message, 'error');
    }
}

// Cancel note editing
function cancelNote() {
    document.getElementById('noteForm').style.display = 'none';
}

// Edit existing note
function editNote(noteId) {
    const note = notes.find(n => n.id === noteId);
    if (!note) {
        showNotification('未找到该备忘录', 'error');
        return;
    }
    
    document.getElementById('noteTitle').value = note.title;
    document.getElementById('noteContent').value = note.content;
    document.getElementById('noteForm').style.display = 'block';
    
    // Update save button to update function
    const saveBtn = document.querySelector('#noteForm .btn');
    saveBtn.textContent = '更新';
    saveBtn.onclick = () => updateNote(noteId);
}

// Update existing note
async function updateNote(noteId) {
    const title = document.getElementById('noteTitle').value.trim();
    const content = document.getElementById('noteContent').value.trim();
    
    if (!title || !content) {
        showNotification('请填写标题和内容', 'error');
        return;
    }
    
    const note = {
        id: noteId,
        title: title,
        content: content,
        created_at: '',
        updated_at: null
    };
    
    try {
        const response = await makeApiRequest(`/api/notes/${noteId}`, {
            method: 'PUT',
            body: JSON.stringify(note)
        });
        
        if (response.ok) {
            // Restore save button
            const saveBtn = document.querySelector('#noteForm .btn');
            saveBtn.textContent = '保存';
            saveBtn.onclick = saveNote;
            
            cancelNote();
            await loadNotes();
            showNotification('备忘录更新成功', 'success');
        } else {
            showNotification('更新备忘录失败: ' + response.status, 'error');
        }
    } catch (error) {
        console.error('Error updating note:', error);
        showNotification('更新备忘录时出错: ' + error.message, 'error');
    }
}

// Delete note
async function deleteNote(noteId) {
    if (confirm('确定要删除这个备忘录吗？')) {
        try {
            const response = await makeApiRequest(`/api/notes/${noteId}`, {
                method: 'DELETE'
            });
            
            if (response.ok) {
                await loadNotes();
                showNotification('备忘录删除成功', 'success');
            } else {
                showNotification('删除备忘录失败: ' + response.status, 'error');
            }
        } catch (error) {
            console.error('Error deleting note:', error);
            showNotification('删除备忘录时出错: ' + error.message, 'error');
        }
    }
}