// API Configuration
const API_BASE = window.location.origin;
const API_V1 = `${API_BASE}/api/v1`;

// State Management
const state = {
    token: localStorage.getItem('jwt_token'),
    arrays: [],
    events: [],
    refreshInterval: null,
    isAuthenticated: false
};

// Utility Functions
const formatBytes = (bytes) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
};

const formatDate = (date) => {
    return new Date(date).toLocaleString();
};

const formatUptime = (seconds) => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    
    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h ${minutes}m`;
    return `${minutes}m`;
};

// API Functions
const api = {
    async request(endpoint, options = {}) {
        const headers = {
            'Content-Type': 'application/json',
            ...options.headers
        };

        if (state.token) {
            headers['Authorization'] = `Bearer ${state.token}`;
        }

        try {
            const response = await fetch(`${API_V1}${endpoint}`, {
                ...options,
                headers
            });

            if (response.status === 401) {
                handleLogout();
                throw new Error('Unauthorized');
            }

            if (!response.ok) {
                const error = await response.json().catch(() => ({}));
                throw new Error(error.error || `HTTP ${response.status}`);
            }

            return await response.json();
        } catch (error) {
            console.error('API Error:', error);
            throw error;
        }
    },

    async login(username, password) {
        const response = await fetch(`${API_V1}/auth/login`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password })
        });

        if (!response.ok) {
            const error = await response.json().catch(() => ({}));
            throw new Error(error.error || 'Login failed');
        }

        return await response.json();
    },

    async getArrays() {
        return await this.request('/arrays');
    },

    async getArrayDetails(name) {
        return await this.request(`/arrays/${encodeURIComponent(name)}`);
    },

    async createArray(data) {
        return await this.request('/arrays', {
            method: 'POST',
            body: JSON.stringify(data)
        });
    },

    async stopArray(name) {
        return await this.request(`/arrays/${encodeURIComponent(name)}`, {
            method: 'DELETE'
        });
    },

    async manageArray(name, action, device) {
        return await this.request(`/arrays/${encodeURIComponent(name)}/manage`, {
            method: 'POST',
            body: JSON.stringify({ action, device })
        });
    },

    async scrubArray(name) {
        return await this.request(`/arrays/${encodeURIComponent(name)}/scrub`, {
            method: 'POST'
        });
    },

    async getHealth() {
        return await this.request('/health');
    }
};

// Event Management
const addEvent = (message, type = 'info') => {
    const event = {
        id: Date.now(),
        message,
        type,
        timestamp: new Date().toISOString()
    };

    state.events.unshift(event);
    if (state.events.length > 50) {
        state.events.pop();
    }

    renderEvents();
};

const clearEvents = () => {
    state.events = [];
    renderEvents();
};

// Rendering Functions
const renderStats = (arrays) => {
    const healthy = arrays.filter(a => a.state === 'active' || a.state === 'clean').length;
    const degraded = arrays.filter(a => a.state === 'degraded').length;
    const failed = arrays.filter(a => a.state === 'failed' || a.state === 'inactive').length;
    const totalDevices = arrays.reduce((sum, a) => sum + (a.raid_devices || 0), 0);

    document.getElementById('healthyArrays').textContent = healthy;
    document.getElementById('degradedArrays').textContent = degraded;
    document.getElementById('failedArrays').textContent = failed;
    document.getElementById('totalDevices').textContent = totalDevices;
    document.getElementById('lastUpdate').textContent = formatDate(new Date());
};

const renderArrays = (arrays) => {
    const container = document.getElementById('arraysContainer');
    
    if (arrays.length === 0) {
        container.innerHTML = `
            <div class="loading">
                <i class="fas fa-inbox"></i>
                <p>No RAID arrays found</p>
            </div>
        `;
        return;
    }

    container.innerHTML = arrays.map(array => {
        const statusClass = array.state === 'active' || array.state === 'clean' ? 'active' : 
                           array.state === 'degraded' ? 'degraded' : 'failed';
        
        const progress = array.sync_completed || 100;
        const isSyncing = array.sync_action && array.sync_action !== 'idle';

        return `
            <div class="array-item status-${statusClass}">
                <div class="array-header">
                    <div class="array-name">
                        <i class="fas fa-database"></i>
                        ${array.name}
                    </div>
                    <span class="array-status status-${statusClass}">
                        ${array.state}
                    </span>
                </div>
                <div class="array-info">
                    <div class="info-item">
                        <div class="info-label">RAID Level</div>
                        <div class="info-value">RAID ${array.level}</div>
                    </div>
                    <div class="info-item">
                        <div class="info-label">Devices</div>
                        <div class="info-value">${array.raid_devices || 0} / ${array.raid_devices || 0}</div>
                    </div>
                    <div class="info-item">
                        <div class="info-label">Size</div>
                        <div class="info-value">${formatBytes((array.array_size || 0) * 1024)}</div>
                    </div>
                    <div class="info-item">
                        <div class="info-label">Chunk Size</div>
                        <div class="info-value">${array.chunk_size || 'N/A'} KB</div>
                    </div>
                </div>
                ${isSyncing ? `
                    <div class="array-progress">
                        <div class="info-label">Sync Progress: ${progress.toFixed(1)}%</div>
                        <div class="progress-bar">
                            <div class="progress-fill" style="width: ${progress}%"></div>
                        </div>
                    </div>
                ` : ''}
                <div class="array-actions">
                    <button class="btn btn-secondary btn-sm" onclick="showArrayDetails('${array.name}')">
                        <i class="fas fa-info-circle"></i> Details
                    </button>
                    <button class="btn btn-success btn-sm" onclick="scrubArray('${array.name}')">
                        <i class="fas fa-broom"></i> Scrub
                    </button>
                    <button class="btn btn-danger btn-sm" onclick="stopArray('${array.name}')">
                        <i class="fas fa-stop"></i> Stop
                    </button>
                </div>
            </div>
        `;
    }).join('');
};

const renderEvents = () => {
    const container = document.getElementById('eventsContainer');
    
    if (state.events.length === 0) {
        container.innerHTML = `
            <div class="event-item event-info">
                <div class="event-icon"><i class="fas fa-info-circle"></i></div>
                <div class="event-content">
                    <div class="event-message">No events yet</div>
                    <div class="event-time">${formatDate(new Date())}</div>
                </div>
            </div>
        `;
        return;
    }

    container.innerHTML = state.events.map(event => `
        <div class="event-item event-${event.type}">
            <div class="event-icon">
                <i class="fas fa-${
                    event.type === 'success' ? 'check-circle' :
                    event.type === 'warning' ? 'exclamation-triangle' :
                    event.type === 'error' ? 'times-circle' : 'info-circle'
                }"></i>
            </div>
            <div class="event-content">
                <div class="event-message">${event.message}</div>
                <div class="event-time">${formatDate(event.timestamp)}</div>
            </div>
        </div>
    `).join('');
};

const renderArrayDetails = (details) => {
    const container = document.getElementById('arrayDetailsContent');
    
    container.innerHTML = `
        <div class="detail-section">
            <h3><i class="fas fa-info-circle"></i> General Information</h3>
            <div class="detail-grid">
                <div class="detail-item">
                    <div class="detail-label">Array Name</div>
                    <div class="detail-value">${details.name}</div>
                </div>
                <div class="detail-item">
                    <div class="detail-label">RAID Level</div>
                    <div class="detail-value">RAID ${details.level}</div>
                </div>
                <div class="detail-item">
                    <div class="detail-label">State</div>
                    <div class="detail-value">${details.state}</div>
                </div>
                <div class="detail-item">
                    <div class="detail-label">Array Size</div>
                    <div class="detail-value">${formatBytes((details.array_size || 0) * 1024)}</div>
                </div>
                <div class="detail-item">
                    <div class="detail-label">Used Size</div>
                    <div class="detail-value">${formatBytes((details.used_dev_size || 0) * 1024)}</div>
                </div>
                <div class="detail-item">
                    <div class="detail-label">Chunk Size</div>
                    <div class="detail-value">${details.chunk_size || 'N/A'} KB</div>
                </div>
                <div class="detail-item">
                    <div class="detail-label">Layout</div>
                    <div class="detail-value">${details.layout || 'N/A'}</div>
                </div>
                <div class="detail-item">
                    <div class="detail-label">Metadata Version</div>
                    <div class="detail-value">${details.metadata_version || 'N/A'}</div>
                </div>
            </div>
        </div>

        ${details.devices && details.devices.length > 0 ? `
            <div class="detail-section">
                <h3><i class="fas fa-hdd"></i> Devices</h3>
                <div class="device-list">
                    ${details.devices.map(device => `
                        <div class="device-item">
                            <div class="device-info">
                                <div class="device-icon">
                                    <i class="fas fa-hdd"></i>
                                </div>
                                <div>
                                    <div class="device-name">${device.name || device}</div>
                                    <div class="device-state">${device.state || 'active'}</div>
                                </div>
                            </div>
                        </div>
                    `).join('')}
                </div>
            </div>
        ` : ''}

        ${details.sync_action && details.sync_action !== 'idle' ? `
            <div class="detail-section">
                <h3><i class="fas fa-sync"></i> Sync Status</h3>
                <div class="detail-grid">
                    <div class="detail-item">
                        <div class="detail-label">Action</div>
                        <div class="detail-value">${details.sync_action}</div>
                    </div>
                    <div class="detail-item">
                        <div class="detail-label">Progress</div>
                        <div class="detail-value">${(details.sync_completed || 0).toFixed(1)}%</div>
                    </div>
                </div>
            </div>
        ` : ''}
    `;
};

// Action Handlers
const handleLogin = async (e) => {
    e.preventDefault();
    
    const username = document.getElementById('username').value;
    const password = document.getElementById('password').value;
    const errorEl = document.getElementById('loginError');

    try {
        const response = await api.login(username, password);
        state.token = response.token;
        localStorage.setItem('jwt_token', response.token);
        state.isAuthenticated = true;
        
        document.getElementById('loginModal').classList.remove('active');
        addEvent('Successfully logged in', 'success');
        await loadDashboard();
    } catch (error) {
        errorEl.textContent = error.message;
        errorEl.classList.add('active');
    }
};

const handleLogout = () => {
    state.token = null;
    state.isAuthenticated = false;
    localStorage.removeItem('jwt_token');
    
    if (state.refreshInterval) {
        clearInterval(state.refreshInterval);
        state.refreshInterval = null;
    }
    
    document.getElementById('loginModal').classList.add('active');
    addEvent('Logged out', 'info');
};

const handleCreateArray = async (e) => {
    e.preventDefault();
    
    const formData = new FormData(e.target);
    const data = {
        name: formData.get('name'),
        level: parseInt(formData.get('level')),
        devices: formData.get('devices').split(',').map(d => d.trim()),
        chunk_size: parseInt(formData.get('chunk_size')),
        metadata: formData.get('metadata'),
        dry_run: formData.get('dry_run') === 'on'
    };

    const errorEl = document.getElementById('createError');

    try {
        await api.createArray(data);
        document.getElementById('createArrayModal').classList.remove('active');
        addEvent(`Array ${data.name} ${data.dry_run ? 'validated' : 'created'} successfully`, 'success');
        await loadArrays();
        e.target.reset();
    } catch (error) {
        errorEl.textContent = error.message;
        errorEl.classList.add('active');
    }
};

const showArrayDetails = async (name) => {
    const modal = document.getElementById('arrayDetailsModal');
    const content = document.getElementById('arrayDetailsContent');
    
    modal.classList.add('active');
    content.innerHTML = '<div class="loading"><i class="fas fa-spinner fa-spin"></i> Loading details...</div>';
    
    try {
        const details = await api.getArrayDetails(name);
        renderArrayDetails(details);
    } catch (error) {
        content.innerHTML = `<div class="error-message active">${error.message}</div>`;
    }
};

const scrubArray = async (name) => {
    if (!confirm(`Start scrub operation on ${name}?`)) return;
    
    try {
        await api.scrubArray(name);
        addEvent(`Scrub started on ${name}`, 'success');
        await loadArrays();
    } catch (error) {
        addEvent(`Failed to scrub ${name}: ${error.message}`, 'error');
    }
};

const stopArray = async (name) => {
    if (!confirm(`Stop array ${name}? This will make the array unavailable.`)) return;
    
    try {
        await api.stopArray(name);
        addEvent(`Array ${name} stopped`, 'warning');
        await loadArrays();
    } catch (error) {
        addEvent(`Failed to stop ${name}: ${error.message}`, 'error');
    }
};

// Data Loading
const loadArrays = async () => {
    try {
        const arrays = await api.getArrays();
        state.arrays = arrays;
        renderArrays(arrays);
        renderStats(arrays);
    } catch (error) {
        console.error('Failed to load arrays:', error);
        addEvent('Failed to load arrays', 'error');
    }
};

const loadDashboard = async () => {
    await loadArrays();
    
    // Start auto-refresh
    if (state.refreshInterval) {
        clearInterval(state.refreshInterval);
    }
    state.refreshInterval = setInterval(loadArrays, 10000); // Refresh every 10 seconds
};

// Event Listeners
document.addEventListener('DOMContentLoaded', () => {
    // Initialize time
    document.getElementById('initTime').textContent = formatDate(new Date());
    
    // Login form
    document.getElementById('loginForm').addEventListener('submit', handleLogin);
    
    // Logout button
    document.getElementById('logoutBtn').addEventListener('click', handleLogout);
    
    // Refresh button
    document.getElementById('refreshBtn').addEventListener('click', loadArrays);
    
    // Create array modal
    document.getElementById('createArrayBtn').addEventListener('click', () => {
        document.getElementById('createArrayModal').classList.add('active');
    });
    
    document.getElementById('closeCreateModal').addEventListener('click', () => {
        document.getElementById('createArrayModal').classList.remove('active');
    });
    
    document.getElementById('cancelCreateBtn').addEventListener('click', () => {
        document.getElementById('createArrayModal').classList.remove('active');
    });
    
    document.getElementById('createArrayForm').addEventListener('submit', handleCreateArray);
    
    // Array details modal
    document.getElementById('closeDetailsModal').addEventListener('click', () => {
        document.getElementById('arrayDetailsModal').classList.remove('active');
    });
    
    // Clear events
    document.getElementById('clearEventsBtn').addEventListener('click', clearEvents);
    
    // Close modals on background click
    document.querySelectorAll('.modal').forEach(modal => {
        modal.addEventListener('click', (e) => {
            if (e.target === modal) {
                modal.classList.remove('active');
            }
        });
    });
    
    // Check authentication
    if (state.token) {
        state.isAuthenticated = true;
        loadDashboard();
    } else {
        document.getElementById('loginModal').classList.add('active');
    }
});

// Make functions globally available
window.showArrayDetails = showArrayDetails;
window.scrubArray = scrubArray;
window.stopArray = stopArray;
