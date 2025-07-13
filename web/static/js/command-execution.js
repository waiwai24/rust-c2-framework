/* Command Execution Module */

// Command execution functionality
async function executeCommand() {
    const command = document.getElementById('commandInput').value.trim();
    if (!command) {
        showNotification('请输入命令', 'error');
        return;
    }

    try {
        const response = await makeApiRequest(`/api/clients/${getClientId()}/commands`, {
            method: 'POST',
            body: JSON.stringify({
                client_id: getClientId(),
                command: command,
                args: []
            })
        });

        if (response.ok) {
            showNotification('命令已发送', 'success');
            document.getElementById('commandInput').value = '';
            // Refresh results after 1 second
            setTimeout(refreshResults, 1000);
        } else {
            showNotification('发送命令失败', 'error');
        }
    } catch (error) {
        showNotification('发送命令时出错: ' + error.message, 'error');
    }
}

// Quick command shortcuts
function setCommand(command) {
    document.getElementById('commandInput').value = command;
}

// Make setCommand globally available
window.setCommand = setCommand;
window.executeCommand = executeCommand;

// Refresh command results
function refreshResults() {
    location.reload();
}

// Helper function to get client ID
function getClientId() {
    return document.body.dataset.clientId || '';
}

// Keyboard event handlers
document.addEventListener('DOMContentLoaded', function() {
    const commandInput = document.getElementById('commandInput');
    if (commandInput) {
        commandInput.addEventListener('keypress', function(e) {
            if (e.key === 'Enter') {
                executeCommand();
            }
        });
    }
    
    // Auto-refresh results every 15 seconds when command tab is active
    setInterval(() => {
        if (document.querySelector('.tab-item[data-tab="command"]')?.classList.contains('active')) {
            refreshResults();
        }
    }, 15000);
});