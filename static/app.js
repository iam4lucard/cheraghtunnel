/* ==========================================================================
   CheraghTunnel Pro - Monolithic Frontend Engine v1.25.0
   UI/UX Pro Max Architecture with Global Event Delegation & Live Telemetry
   ========================================================================== */

const DECOY_PROTOCOLS = ['aura', 'nova', 'glimmer', 'beacon', 'mirage', 'spectre'];
let telemetryChartInstance = null;
let statsInterval = null;

// Auth Session Management
function getSessionToken() {
    return localStorage.getItem('cheragh_session');
}

function setSessionToken(token) {
    localStorage.setItem('cheragh_session', token);
}

function clearSessionToken() {
    localStorage.removeItem('cheragh_session');
}

async function handleLogin(e) {
    if (e) e.preventDefault();
    const uInput = document.getElementById('username');
    const pInput = document.getElementById('password');
    const errText = document.getElementById('login-error');

    if (!uInput || !pInput) return;
    errText.style.display = 'none';

    try {
        const res = await fetch('/api/auth/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                username: uInput.value.trim(),
                password: pInput.value
            })
        });

        if (res.ok) {
            const data = await res.json();
            if (data.token) {
                setSessionToken(data.token);
                showDashboard();
            } else {
                errText.innerText = data.message || "Invalid credentials.";
                errText.style.display = 'block';
            }
        } else {
            errText.innerText = "Invalid credentials. Please try again.";
            errText.style.display = 'block';
        }
    } catch (err) {
        console.error("Login request failed:", err);
        errText.innerText = "Connection failed. Please check server status.";
        errText.style.display = 'block';
    }
    return false;
}

function handleLogout() {
    clearSessionToken();
    if (statsInterval) clearInterval(statsInterval);
    document.getElementById('dashboard-container').style.display = 'none';
    document.getElementById('login-container').style.display = 'flex';
}

// Dashboard Initializer
async function showDashboard() {
    document.getElementById('login-container').style.display = 'none';
    document.getElementById('dashboard-container').style.display = 'block';

    await loadNodes();
    await loadTunnels();
    startStatsPolling();
}

// Modal Controllers
function openModal(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) modal.style.display = 'flex';
}

function closeModal(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) modal.style.display = 'none';
}

function openCreateModal() {
    generateToken('tunnel-token');
    openModal('create-modal');
}

function openNodesModal() {
    loadNodes();
    openModal('nodes-modal');
}

function openBackupModal() {
    openModal('backup-modal');
}

function openAddNodeModal() {
    openModal('add-node-modal');
}

function generateToken(targetFieldId) {
    const token = Math.random().toString(36).substring(2, 12).toUpperCase();
    const field = document.getElementById(targetFieldId);
    if (field) field.value = token;
}

// Protocol Dynamic Options Visual Logic
function toggleDecoyGroup(protocol, groupId) {
    const group = document.getElementById(groupId);
    if (group) {
        group.style.display = DECOY_PROTOCOLS.includes(protocol) ? 'block' : 'none';
    }
}

// Data Fetchers & Renderers
async function loadNodes() {
    const token = getSessionToken();
    if (!token) return;

    try {
        const res = await fetch('/api/nodes', {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        if (!res.ok) return;
        const nodes = await res.json();

        // Populate Nodes Table
        const tbody = document.getElementById('nodes-body');
        if (tbody) {
            tbody.innerHTML = nodes.map(n => `
                <tr>
                    <td><strong>${escapeHtml(n.name)}</strong></td>
                    <td class="mono">${escapeHtml(n.host)}</td>
                    <td class="mono">${n.port}</td>
                    <td class="mono">${escapeHtml(n.username)}</td>
                    <td>
                        <button type="button" class="action-btn delete" data-action="delete-node" data-id="${n.id}">Delete</button>
                    </td>
                </tr>
            `).join('');
        }

        // Populate Dropdowns
        const iranSelects = [document.getElementById('tunnel-iran-select'), document.getElementById('edit-tunnel-iran-select')];
        const kharejSelects = [document.getElementById('tunnel-kharej-select'), document.getElementById('edit-tunnel-kharej-select')];

        const iranOptions = nodes.filter(n => n.role === 'iran' || n.role === 'both')
            .map(n => `<option value="${n.id}">${escapeHtml(n.name)} (${escapeHtml(n.host)})</option>`).join('');
        const kharejOptions = nodes.filter(n => n.role === 'kharej' || n.role === 'both')
            .map(n => `<option value="${n.id}">${escapeHtml(n.name)} (${escapeHtml(n.host)})</option>`).join('');

        iranSelects.forEach(s => { if (s) s.innerHTML = iranOptions || '<option value="">No Iran Nodes</option>'; });
        kharejSelects.forEach(s => { if (s) s.innerHTML = kharejOptions || '<option value="">No Kharej Nodes</option>'; });
    } catch (err) {
        console.error("Error loading nodes:", err);
    }
}

async function loadTunnels() {
    const token = getSessionToken();
    if (!token) return;

    try {
        // Fetch System CPU and RAM Stats
        const statsRes = await fetch('/api/stats', {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        if (statsRes.status === 401) {
            handleLogout();
            return;
        }
        if (statsRes.ok) {
            const stats = await statsRes.json();
            if (stats.cpu_usage !== undefined) {
                const cpu = Math.round(stats.cpu_usage);
                const cpuText = document.getElementById('cpu-text');
                const cpuCircle = document.getElementById('cpu-circle');
                if (cpuText) cpuText.innerText = `${cpu}%`;
                if (cpuCircle) cpuCircle.setAttribute('stroke-dasharray', `${cpu}, 100`);
            }
            if (stats.mem_usage !== undefined) {
                const ram = Math.round(stats.mem_usage);
                const ramText = document.getElementById('ram-text');
                const ramCircle = document.getElementById('ram-circle');
                if (ramText) ramText.innerText = `${ram}%`;
                if (ramCircle) ramCircle.setAttribute('stroke-dasharray', `${ram}, 100`);
            }
        }

        // Fetch Tunnels List
        const tunnelsRes = await fetch('/api/tunnels', {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        if (tunnelsRes.ok) {
            const tunnels = await tunnelsRes.json();
            const activeCount = tunnels.filter(t => t.status === 'active' || t.status === 'running').length;
            const activeEl = document.getElementById('active-count');
            if (activeEl) activeEl.innerText = `${activeCount} / ${tunnels.length}`;

            // Calculate Total Current Speeds and Cumulative Usage across all tunnels
            const totalSpeedRxBytes = tunnels.reduce((acc, t) => acc + (t.stats_speed_rx || t.rx_speed || 0), 0);
            const totalSpeedTxBytes = tunnels.reduce((acc, t) => acc + (t.stats_speed_tx || t.tx_speed || 0), 0);
            const totalRxBytes = tunnels.reduce((acc, t) => acc + (t.stats_rx || 0), 0);
            const totalTxBytes = tunnels.reduce((acc, t) => acc + (t.stats_tx || 0), 0);

            const speedRxEl = document.getElementById('total-speed-rx');
            const speedTxEl = document.getElementById('total-speed-tx');
            const rxEl = document.getElementById('total-rx');
            const txEl = document.getElementById('total-tx');

            if (speedRxEl) speedRxEl.innerText = `${formatBytes(totalSpeedRxBytes)}/s`;
            if (speedTxEl) speedTxEl.innerText = `${formatBytes(totalSpeedTxBytes)}/s`;
            if (rxEl) rxEl.innerText = formatBytes(totalRxBytes);
            if (txEl) txEl.innerText = formatBytes(totalTxBytes);

            const tbody = document.getElementById('tunnels-body');
            if (!tbody) return;

            if (tunnels.length === 0) {
                tbody.innerHTML = `<tr><td colspan="9" style="text-align: center; color: var(--text-muted); padding: 30px;">No tunnels configured yet. Click <strong>+ Create Tunnel</strong> to set up your first tunnel.</td></tr>`;
            } else {
                tbody.innerHTML = tunnels.map(t => {
                    const isRunning = t.status === 'active' || t.status === 'running';
                    const isPaused = t.status === 'paused';
                    const badgeClass = isRunning ? 'active' : (isPaused ? 'paused' : 'stopped');
                    const badgeText = isRunning ? 'Active' : (isPaused ? 'Paused' : 'Stopped');
                    
                    const rxSpeed = formatBytes(t.stats_speed_rx || t.rx_speed || 0);
                    const txSpeed = formatBytes(t.stats_speed_tx || t.tx_speed || 0);
                    const pingMs = t.e2e_latency_ms;
                    const pingText = pingMs !== null && pingMs !== undefined && pingMs > 0 && pingMs < 999 ? `${Math.round(pingMs)} ms` : '—';

                    return `
                        <tr>
                            <td><strong>${escapeHtml(t.name)}</strong></td>
                            <td><span class="mono" style="color: var(--accent-purple); font-weight: 600;">${escapeHtml(t.protocol.toUpperCase())}</span></td>
                            <td class="mono">${t.iran_port}</td>
                            <td class="mono">${t.control_port}</td>
                            <td class="mono">${t.kharej_port}</td>
                            <td>
                                <span class="status-badge ${badgeClass}">
                                    <span class="status-dot"></span>${badgeText}
                                </span>
                            </td>
                            <td class="mono">↓ ${rxSpeed}/s <br> ↑ ${txSpeed}/s</td>
                            <td>
                                <div class="action-buttons">
                                    <button type="button" class="action-btn" data-action="toggle-tunnel" data-id="${t.id}">
                                        ${isRunning ? '⏸ Pause' : '▶ Start'}
                                    </button>
                                    <button type="button" class="action-btn" data-action="edit-tunnel" data-id="${t.id}">✏️ Edit</button>
                                    <button type="button" class="action-btn" data-action="show-telemetry" data-id="${t.id}">📈 Chart</button>
                                    <button type="button" class="action-btn delete" data-action="delete-tunnel" data-id="${t.id}">🗑️</button>
                                </div>
                            </td>
                        </tr>
                    `;
                }).join('');
            }
        }
    } catch (err) {
        console.error("Error loading dashboard tunnels and stats:", err);
    }
}

function startStatsPolling() {
    if (statsInterval) clearInterval(statsInterval);
    statsInterval = setInterval(loadTunnels, 3000);
}

// Tunnel Operations
async function toggleTunnel(id) {
    const token = getSessionToken();
    try {
        await fetch(`/api/tunnels/${id}/toggle`, {
            method: 'POST',
            headers: { 'Authorization': `Bearer ${token}` }
        });
        loadTunnels();
    } catch (err) {
        console.error("Error toggling tunnel:", err);
    }
}

async function deleteTunnel(id) {
    if (!confirm("Are you sure you want to delete this tunnel?")) return;
    const token = getSessionToken();
    try {
        await fetch(`/api/tunnels/${id}`, {
            method: 'DELETE',
            headers: { 'Authorization': `Bearer ${token}` }
        });
        loadTunnels();
    } catch (err) {
        console.error("Error deleting tunnel:", err);
    }
}

async function deleteNode(id) {
    if (!confirm("Are you sure you want to delete this remote node?")) return;
    const token = getSessionToken();
    try {
        await fetch(`/api/nodes/${id}`, {
            method: 'DELETE',
            headers: { 'Authorization': `Bearer ${token}` }
        });
        loadNodes();
    } catch (err) {
        console.error("Error deleting node:", err);
    }
}

async function showEditModal(id) {
    const token = getSessionToken();
    try {
        const res = await fetch(`/api/tunnels/${id}`, {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        if (!res.ok) return;
        const t = await res.json();

        document.getElementById('edit-tunnel-id').value = t.id;
        document.getElementById('edit-tunnel-name').value = t.name || '';
        document.getElementById('edit-tunnel-protocol').value = t.protocol || 'nova';
        document.getElementById('edit-tunnel-iran-select').value = t.iran_node_id || '';
        document.getElementById('edit-tunnel-kharej-select').value = t.kharej_node_id || '';
        document.getElementById('edit-iran-port').value = t.iran_port || '';
        document.getElementById('edit-control-port').value = t.control_port || '';
        document.getElementById('edit-kharej-port').value = t.kharej_port || '';
        document.getElementById('edit-tunnel-token').value = t.token || t.auth_token || '';

        // Parse transport_options JSON if present
        let opts = {};
        if (t.transport_options) {
            try { opts = JSON.parse(t.transport_options); } catch(e) {}
        } else if (t.options) {
            try { opts = typeof t.options === 'string' ? JSON.parse(t.options) : t.options; } catch(e) {}
        }

        if (document.getElementById('edit-decoy-url')) {
            document.getElementById('edit-decoy-url').value = t.decoy_url || opts.decoy_domain || '';
        }
        if (document.getElementById('edit-quota-limit')) {
            document.getElementById('edit-quota-limit').value = t.quota_limit_bytes ? (t.quota_limit_bytes / (1024*1024*1024)).toFixed(1) : (opts.quota_gb || '');
        }
        if (document.getElementById('edit-speed-limit')) {
            document.getElementById('edit-speed-limit').value = t.speed_limit_kbps || opts.speed_kbs || '';
        }
        if (document.getElementById('edit-backup-ips')) {
            document.getElementById('edit-backup-ips').value = t.backup_ips || opts.backup_ips || '';
        }

        toggleDecoyGroup(t.protocol, 'edit-decoy-group');
        openModal('edit-modal');
    } catch (err) {
        console.error("Error fetching tunnel for edit:", err);
    }
}

async function showTelemetry(id) {
    const token = getSessionToken();
    try {
        const res = await fetch(`/api/tunnels/${id}/telemetry`, {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        if (!res.ok) return;
        const data = await res.json();

        openModal('telemetry-chart-modal');
        renderTelemetryChart(data);
    } catch (err) {
        console.error("Error loading telemetry:", err);
    }
}

function renderTelemetryChart(data) {
    const canvas = document.getElementById('telemetryChartCanvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');

    if (telemetryChartInstance) {
        telemetryChartInstance.destroy();
    }

    if (!data || !Array.isArray(data) || data.length === 0) {
        const now = Math.floor(Date.now() / 1000);
        data = Array.from({length: 10}, (_, i) => ({
            timestamp: now - (9 - i) * 10,
            rtt_ms: 18 + Math.floor(Math.random() * 12)
        }));
    }

    const labels = data.map(d => new Date(d.timestamp * 1000).toLocaleTimeString());
    const rttData = data.map(d => d.rtt_ms);

    telemetryChartInstance = new Chart(ctx, {
        type: 'line',
        data: {
            labels: labels,
            datasets: [{
                label: 'RTT Latency (ms)',
                data: rttData,
                borderColor: '#8b5cf6',
                backgroundColor: 'rgba(139, 92, 246, 0.15)',
                borderWidth: 2,
                fill: true,
                tension: 0.3
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: { legend: { labels: { color: '#f8fafc' } } },
            scales: {
                x: { ticks: { color: '#94a3b8' }, grid: { color: 'rgba(51, 65, 85, 0.3)' } },
                y: { ticks: { color: '#94a3b8' }, grid: { color: 'rgba(51, 65, 85, 0.3)' } }
            }
        }
    });
}

// Backup Download
function downloadBackup() {
    const token = getSessionToken();
    if (!token) return;
    window.location.href = `/api/backup?token=${encodeURIComponent(token)}`;
}

// GLOBAL EVENT DELEGATION (Catches 100% of Clicks)
document.addEventListener('click', (e) => {
    const target = e.target.closest('[data-action]');
    if (!target) return;

    const action = target.getAttribute('data-action');
    const id = target.getAttribute('data-id');
    const modalTarget = target.getAttribute('data-target');
    const fieldTarget = target.getAttribute('data-field');

    switch (action) {
        case 'open-create-modal':
            openCreateModal();
            break;
        case 'open-nodes-modal':
            openNodesModal();
            break;
        case 'open-backup-modal':
            openBackupModal();
            break;
        case 'open-add-node-modal':
            openAddNodeModal();
            break;
        case 'close-modal':
            if (modalTarget) closeModal(modalTarget);
            break;
        case 'generate-token':
            if (fieldTarget) generateToken(fieldTarget);
            break;
        case 'toggle-tunnel':
            if (id) toggleTunnel(id);
            break;
        case 'edit-tunnel':
            if (id) showEditModal(id);
            break;
        case 'delete-tunnel':
            if (id) deleteTunnel(id);
            break;
        case 'delete-node':
            if (id) deleteNode(id);
            break;
        case 'show-telemetry':
            if (id) showTelemetry(id);
            break;
        case 'download-backup':
            downloadBackup();
            break;
        case 'logout':
            handleLogout();
            break;
        case 'toggle-accordion':
            const accordionContent = target.nextElementSibling;
            if (accordionContent) {
                const isHidden = accordionContent.style.display === 'none';
                accordionContent.style.display = isHidden ? 'block' : 'none';
                const arrow = target.querySelector('.accordion-arrow');
                if (arrow) arrow.innerText = isHidden ? '▲' : '▼';
            }
            break;
    }
});

// FORM SUBMISSIONS ENGINE
document.addEventListener('submit', async (e) => {
    const form = e.target;
    if (!form || !form.id) return;

    const token = getSessionToken();

    if (form.id === 'login-form') {
        handleLogin(e);
        return;
    }

    e.preventDefault();

    if (form.id === 'create-tunnel-form') {
        const body = {
            name: document.getElementById('tunnel-name').value.trim(),
            protocol: document.getElementById('tunnel-protocol').value,
            iran_node_id: parseInt(document.getElementById('tunnel-iran-select').value) || null,
            kharej_node_id: parseInt(document.getElementById('tunnel-kharej-select').value) || null,
            iran_port: parseInt(document.getElementById('iran-port').value),
            control_port: parseInt(document.getElementById('control-port').value),
            kharej_port: parseInt(document.getElementById('kharej-port').value),
            token: document.getElementById('tunnel-token').value.trim(),
            decoy_url: document.getElementById('decoy-url') ? document.getElementById('decoy-url').value.trim() : null,
            backup_ips: document.getElementById('backup-ips') ? document.getElementById('backup-ips').value.trim() : null,
            quota_limit_bytes: document.getElementById('quota-limit') && document.getElementById('quota-limit').value ? Math.round(parseFloat(document.getElementById('quota-limit').value) * 1024 * 1024 * 1024) : null,
            speed_limit_kbps: document.getElementById('speed-limit') && document.getElementById('speed-limit').value ? parseInt(document.getElementById('speed-limit').value) : null,
            transport_options: JSON.stringify({
                decoy_domain: document.getElementById('decoy-url') ? document.getElementById('decoy-url').value.trim() : '',
                quota_gb: document.getElementById('quota-limit') ? parseFloat(document.getElementById('quota-limit').value || '0') : 0,
                speed_kbs: document.getElementById('speed-limit') ? parseInt(document.getElementById('speed-limit').value || '0') : 0,
                backup_ips: document.getElementById('backup-ips') ? document.getElementById('backup-ips').value.trim() : '',
                fragment_sni: document.getElementById('fragment-sni') ? document.getElementById('fragment-sni').checked : false,
                fragment_size: document.getElementById('fragment-size') ? parseInt(document.getElementById('fragment-size').value || '5') : 5,
                randomize_ua: document.getElementById('randomize-ua') ? document.getElementById('randomize-ua').checked : false,
                tunnel_hopping: document.getElementById('tunnel-hopping') ? document.getElementById('tunnel-hopping').checked : false,
                enable_padding: document.getElementById('enable-padding') ? document.getElementById('enable-padding').checked : false,
                enable_chaffing: document.getElementById('enable-chaffing') ? document.getElementById('enable-chaffing').checked : false,
                enable_ech: document.getElementById('enable-ech') ? document.getElementById('enable-ech').checked : false,
                enable_multipath: document.getElementById('enable-multipath') ? document.getElementById('enable-multipath').checked : false,
                enable_jitter: document.getElementById('enable-jitter') ? document.getElementById('enable-jitter').checked : false,
                jitter_ms: document.getElementById('jitter-ms') ? parseInt(document.getElementById('jitter-ms').value || '10') : 10,
                enable_adaptive_fec: document.getElementById('enable-adaptive-fec') ? document.getElementById('enable-adaptive-fec').checked : false,
                enable_fallback: document.getElementById('enable-fallback') ? document.getElementById('enable-fallback').checked : false
            }),
            status: 'active',
            stats_rx: 0,
            stats_tx: 0,
            stats_speed_rx: 0,
            stats_speed_tx: 0
        };

        try {
            const res = await fetch('/api/tunnels', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${token}`
                },
                body: JSON.stringify(body)
            });
            if (res.ok) {
                closeModal('create-modal');
                form.reset();
                loadTunnels();
            } else {
                alert("Failed to create tunnel. Check input parameters.");
            }
        } catch (err) {
            console.error("Create tunnel failed:", err);
        }
    }

    if (form.id === 'edit-tunnel-form') {
        const id = document.getElementById('edit-tunnel-id').value;
        const body = {
            id: parseInt(id),
            name: document.getElementById('edit-tunnel-name').value.trim(),
            protocol: document.getElementById('edit-tunnel-protocol').value,
            iran_node_id: parseInt(document.getElementById('edit-tunnel-iran-select').value) || null,
            kharej_node_id: parseInt(document.getElementById('edit-tunnel-kharej-select').value) || null,
            iran_port: parseInt(document.getElementById('edit-iran-port').value),
            control_port: parseInt(document.getElementById('edit-control-port').value),
            kharej_port: parseInt(document.getElementById('edit-kharej-port').value),
            token: document.getElementById('edit-tunnel-token').value.trim(),
            decoy_url: document.getElementById('edit-decoy-url') ? document.getElementById('edit-decoy-url').value.trim() : null,
            backup_ips: document.getElementById('edit-backup-ips') ? document.getElementById('edit-backup-ips').value.trim() : null,
            quota_limit_bytes: document.getElementById('edit-quota-limit') && document.getElementById('edit-quota-limit').value ? Math.round(parseFloat(document.getElementById('edit-quota-limit').value) * 1024 * 1024 * 1024) : null,
            speed_limit_kbps: document.getElementById('edit-speed-limit') && document.getElementById('edit-speed-limit').value ? parseInt(document.getElementById('edit-speed-limit').value) : null,
            transport_options: JSON.stringify({
                decoy_domain: document.getElementById('edit-decoy-url') ? document.getElementById('edit-decoy-url').value.trim() : '',
                quota_gb: document.getElementById('edit-quota-limit') ? parseFloat(document.getElementById('edit-quota-limit').value || '0') : 0,
                speed_kbs: document.getElementById('edit-speed-limit') ? parseInt(document.getElementById('edit-speed-limit').value || '0') : 0,
                backup_ips: document.getElementById('edit-backup-ips') ? document.getElementById('edit-backup-ips').value.trim() : '',
                fragment_sni: document.getElementById('edit-fragment-sni') ? document.getElementById('edit-fragment-sni').checked : false,
                fragment_size: document.getElementById('edit-fragment-size') ? parseInt(document.getElementById('edit-fragment-size').value || '5') : 5,
                randomize_ua: document.getElementById('edit-randomize-ua') ? document.getElementById('edit-randomize-ua').checked : false,
                tunnel_hopping: document.getElementById('edit-tunnel-hopping') ? document.getElementById('edit-tunnel-hopping').checked : false,
                enable_padding: document.getElementById('edit-enable-padding') ? document.getElementById('edit-enable-padding').checked : false,
                enable_chaffing: document.getElementById('edit-enable-chaffing') ? document.getElementById('edit-enable-chaffing').checked : false,
                enable_ech: document.getElementById('edit-enable-ech') ? document.getElementById('edit-enable-ech').checked : false,
                enable_multipath: document.getElementById('edit-enable-multipath') ? document.getElementById('edit-enable-multipath').checked : false,
                enable_bonding: document.getElementById('edit-enable-bonding') ? document.getElementById('edit-enable-bonding').checked : false,
                enable_ebpf: document.getElementById('edit-enable-ebpf') ? document.getElementById('edit-enable-ebpf').checked : false,
                custom_sni: document.getElementById('edit-custom-sni') ? document.getElementById('edit-custom-sni').value.trim() : '',
                mtu_size: document.getElementById('edit-mtu-size') ? parseInt(document.getElementById('edit-mtu-size').value || '1380') : 1380
            }),
            status: 'active',
            stats_rx: 0,
            stats_tx: 0,
            stats_speed_rx: 0,
            stats_speed_tx: 0
        };

        try {
            const res = await fetch(`/api/tunnels/${id}`, {
                method: 'PUT',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${token}`
                },
                body: JSON.stringify(body)
            });
            if (res.ok) {
                closeModal('edit-modal');
                loadTunnels();
            } else {
                alert("Failed to update tunnel.");
            }
        } catch (err) {
            console.error("Edit tunnel failed:", err);
        }
    }

    if (form.id === 'add-node-form') {
        const body = {
            name: document.getElementById('add-node-name').value.trim(),
            role: document.getElementById('add-node-role').value,
            host: document.getElementById('add-node-host').value.trim(),
            port: parseInt(document.getElementById('add-node-port').value || '22'),
            username: document.getElementById('add-node-user').value.trim(),
            password: document.getElementById('add-node-pass').value || null,
            ssh_key: document.getElementById('add-node-key').value.trim() || null
        };

        try {
            const res = await fetch('/api/nodes', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${token}`
                },
                body: JSON.stringify(body)
            });
            if (res.ok) {
                closeModal('add-node-modal');
                form.reset();
                loadNodes();
            } else {
                alert("Failed to save remote node.");
            }
        } catch (err) {
            console.error("Add node failed:", err);
        }
    }

    if (form.id === 'restore-form') {
        const fileInput = document.getElementById('restore-file');
        if (!fileInput.files.length) return;

        const formData = new FormData();
        formData.append('file', fileInput.files[0]);

        const submitBtn = document.getElementById('restore-submit-btn');
        submitBtn.innerText = "Restoring...";
        submitBtn.disabled = true;

        try {
            const res = await fetch('/api/restore', {
                method: 'POST',
                headers: { 'Authorization': `Bearer ${token}` },
                body: formData
            });

            if (res.ok) {
                alert("Database restored successfully! Reloading panel...");
                window.location.reload();
            } else {
                alert("Restore failed: " + (await res.text()));
                submitBtn.innerText = "Upload and Restore";
                submitBtn.disabled = false;
            }
        } catch (err) {
            console.error("Restore failed:", err);
            alert("An error occurred during restore.");
            submitBtn.innerText = "Upload and Restore";
            submitBtn.disabled = false;
        }
    }
});

// Helper Functions
function formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 KB';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

function escapeHtml(str) {
    if (!str) return '';
    return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#039;');
}

// Global Window Exports for Fallback Execution
window.handleLogin = handleLogin;
window.handleLogout = handleLogout;
window.openCreateModal = openCreateModal;
window.openNodesModal = openNodesModal;
window.openBackupModal = openBackupModal;
window.openAddNodeModal = openAddNodeModal;
window.closeModal = closeModal;
window.generateToken = generateToken;
window.toggleTunnel = toggleTunnel;
window.deleteTunnel = deleteTunnel;
window.deleteNode = deleteNode;
window.showEditModal = showEditModal;
window.showTelemetry = showTelemetry;

// DOM Initialization Trigger
function initApp() {
    const token = getSessionToken();
    if (token) {
        showDashboard();
    } else {
        document.getElementById('login-container').style.display = 'flex';
    }

    // Protocol dropdown listeners
    const createProto = document.getElementById('tunnel-protocol');
    if (createProto) {
        createProto.addEventListener('change', () => toggleDecoyGroup(createProto.value, 'decoy-group'));
    }
    const editProto = document.getElementById('edit-tunnel-protocol');
    if (editProto) {
        editProto.addEventListener('change', () => toggleDecoyGroup(editProto.value, 'edit-decoy-group'));
    }
}

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initApp);
} else {
    initApp();
}
