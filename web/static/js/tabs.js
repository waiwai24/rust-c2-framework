/* Tab System Management */

class TabManager {
    constructor() {
        this.initializeTabs();
    }

    initializeTabs() {
        document.addEventListener('DOMContentLoaded', () => {
            const tabItems = document.querySelectorAll('.tab-item');
            const tabContents = document.querySelectorAll('.tab-content');
            
            tabItems.forEach(item => {
                item.addEventListener('click', (e) => {
                    this.switchTab(e.target);
                });
            });
        });
    }

    switchTab(clickedTab) {
        const tabId = clickedTab.getAttribute('data-tab');
        const tabItems = document.querySelectorAll('.tab-item');
        const tabContents = document.querySelectorAll('.tab-content');
        
        // Remove all active classes
        tabItems.forEach(tab => tab.classList.remove('active'));
        tabContents.forEach(content => content.classList.remove('active'));
        
        // Add active class to current tab
        clickedTab.classList.add('active');
        const targetContent = document.getElementById(`${tabId}-tab`);
        if (targetContent) {
            targetContent.classList.add('active');
        }
        
        // Execute tab-specific initialization
        this.onTabSwitch(tabId);
    }

    onTabSwitch(tabId) {
        switch (tabId) {
            case 'logs':
                if (typeof loadLogs === 'function') {
                    loadLogs();
                }
                break;
            case 'notes':
                if (typeof loadNotes === 'function') {
                    loadNotes();
                }
                break;
            case 'settings':
                if (typeof loadSettings === 'function') {
                    loadSettings();
                }
                break;
            case 'file':
                this.initializeFileManager();
                break;
            case 'reverse-shell':
                this.initializeReverseShell();
                break;
            case 'command':
                // Command tab doesn't need special initialization
                break;
        }
    }

    initializeFileManager() {
        const uploadSection = document.querySelector('.file-upload-section');
        const listSection = document.querySelector('.file-list-section');
        
        if (uploadSection) uploadSection.style.display = 'block';
        if (listSection) listSection.style.display = 'block';
        
        // Note: Don't auto-list directory on tab switch, let user click Browse button
    }

    initializeReverseShell() {
        const statusElement = document.getElementById('reverseShellStatus');
        if (statusElement) {
            statusElement.textContent = '等待启动...';
        }
    }
}

// Initialize tab manager
const tabManager = new TabManager();