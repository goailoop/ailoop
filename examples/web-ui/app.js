/**
 * ailoop Web UI Client
 * Provides real-time monitoring of agent messages via WebSocket and HTTP API
 */

class AiloopWebUI {
    constructor() {
        this.ws = null;
        this.currentChannel = null;
        this.channels = new Map();
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.reconnectDelay = 1000;

        this.init();
    }

    init() {
        this.bindElements();
        this.connectWebSocket();
        this.loadChannels();
        this.loadStats();
    }

    bindElements() {
        this.connectionStatus = document.getElementById('connection-status');
        this.channelsList = document.getElementById('channels-list');
        this.currentChannelHeader = document.getElementById('current-channel');
        this.messagesContainer = document.getElementById('messages-container');
        this.statsInfo = document.getElementById('stats-info');

        document.getElementById('refresh-btn').addEventListener('click', () => {
            this.loadChannels();
            this.loadStats();
            if (this.currentChannel) {
                this.loadMessages(this.currentChannel);
            }
        });

        document.getElementById('clear-btn').addEventListener('click', () => {
            this.messagesContainer.innerHTML = '';
        });
    }

    connectWebSocket() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}`;

        this.updateConnectionStatus('connecting', 'Connecting...');

        try {
            this.ws = new WebSocket(wsUrl);

            this.ws.onopen = () => {
                this.updateConnectionStatus('connected', 'Connected');
                this.reconnectAttempts = 0;
                console.log('WebSocket connected');

                // Subscribe to all channels for monitoring
                this.sendWebSocketMessage({
                    type: 'subscribe',
                    channel: '*'
                });
            };

            this.ws.onmessage = (event) => {
                this.handleWebSocketMessage(event.data);
            };

            this.ws.onclose = () => {
                this.updateConnectionStatus('disconnected', 'Disconnected');
                this.handleReconnect();
            };

            this.ws.onerror = (error) => {
                console.error('WebSocket error:', error);
                this.updateConnectionStatus('disconnected', 'Connection Error');
            };

        } catch (error) {
            console.error('Failed to create WebSocket connection:', error);
            this.updateConnectionStatus('disconnected', 'Failed to Connect');
            this.handleReconnect();
        }
    }

    handleReconnect() {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            console.log(`Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts})...`);

            setTimeout(() => {
                this.connectWebSocket();
            }, this.reconnectDelay * this.reconnectAttempts);
        } else {
            console.error('Max reconnection attempts reached');
            this.updateConnectionStatus('disconnected', 'Failed to Reconnect');
        }
    }

    sendWebSocketMessage(message) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(message));
        } else {
            console.warn('WebSocket not connected, cannot send message:', message);
        }
    }

    handleWebSocketMessage(data) {
        try {
            const message = JSON.parse(data);
            this.displayMessage(message);
        } catch (error) {
            console.error('Failed to parse WebSocket message:', error, data);
        }
    }

    updateConnectionStatus(status, text) {
        this.connectionStatus.className = `status ${status}`;
        this.connectionStatus.textContent = text;
    }

    async loadChannels() {
        try {
            const response = await fetch('/api/channels');
            const data = await response.json();
            this.updateChannelsList(data.channels);
        } catch (error) {
            console.error('Failed to load channels:', error);
            this.channelsList.innerHTML = '<div class="error">Failed to load channels</div>';
        }
    }

    async loadStats() {
        try {
            const response = await fetch('/api/stats');
            const stats = await response.json();
            this.updateStats(stats);
        } catch (error) {
            console.error('Failed to load stats:', error);
            this.statsInfo.textContent = 'Failed to load stats';
        }
    }

    async loadMessages(channel) {
        try {
            const response = await fetch(`/api/channels/${encodeURIComponent(channel)}/messages?limit=50`);
            const data = await response.json();
            this.displayChannelMessages(channel, data.messages);
        } catch (error) {
            console.error('Failed to load messages:', error);
        }
    }

    updateChannelsList(channels) {
        this.channelsList.innerHTML = '';

        if (channels.length === 0) {
            this.channelsList.innerHTML = '<div class="channel-item">No active channels</div>';
            return;
        }

        channels.forEach(channel => {
            const channelElement = document.createElement('div');
            channelElement.className = 'channel-item';
            if (this.currentChannel === channel.name) {
                channelElement.classList.add('active');
            }

            channelElement.innerHTML = `
                <div class="channel-name">${this.escapeHtml(channel.name)}</div>
                <div class="message-count">${channel.message_count}</div>
            `;

            channelElement.addEventListener('click', () => {
                this.selectChannel(channel.name);
            });

            this.channelsList.appendChild(channelElement);
            this.channels.set(channel.name, channel);
        });
    }

    updateStats(stats) {
        this.statsInfo.innerHTML = `
            <div>Total Viewers: ${stats.total_viewers}</div>
            <div>Agent Connections: ${stats.agent_connections}</div>
            <div>Viewer Connections: ${stats.viewer_connections}</div>
            <div>Active Channels: ${stats.active_channels}</div>
        `;
    }

    selectChannel(channelName) {
        this.currentChannel = channelName;
        this.currentChannelHeader.textContent = `Channel: ${channelName}`;
        this.updateChannelSelection();
        this.loadMessages(channelName);
    }

    updateChannelSelection() {
        document.querySelectorAll('.channel-item').forEach(item => {
            item.classList.remove('active');
        });

        const activeItem = Array.from(document.querySelectorAll('.channel-item')).find(item => {
            return item.querySelector('.channel-name').textContent === this.currentChannel;
        });

        if (activeItem) {
            activeItem.classList.add('active');
        }
    }

    displayChannelMessages(channel, messages) {
        this.messagesContainer.innerHTML = '';

        if (messages.length === 0) {
            this.messagesContainer.innerHTML = '<div class="message">No messages in this channel yet</div>';
            return;
        }

        messages.reverse().forEach(message => {
            this.displayMessage(message);
        });
    }

    displayMessage(message) {
        const messageElement = document.createElement('div');
        messageElement.className = `message ${this.getMessageClass(message)}`;

        const timestamp = message.timestamp ? new Date(message.timestamp).toLocaleTimeString() : 'Unknown';
        const agentType = message.metadata && message.metadata.agent_type ? message.metadata.agent_type : 'unknown';

        messageElement.innerHTML = `
            <div class="message-header">
                <span class="message-agent">[${this.escapeHtml(agentType)}]</span>
                <span class="message-timestamp">${timestamp}</span>
            </div>
            <div class="message-content">${this.formatMessageContent(message)}</div>
        `;

        this.messagesContainer.appendChild(messageElement);

        // Auto-scroll to bottom
        this.messagesContainer.scrollTop = this.messagesContainer.scrollHeight;
    }

    getMessageClass(message) {
        if (!message.content) return '';

        switch (message.content.type) {
            case 'notification':
                if (message.content.priority === 'urgent') return 'error';
                if (message.content.priority === 'high') return 'warning';
                return 'agent';
            case 'question':
                return 'agent';
            case 'authorization':
                return 'warning';
            case 'response':
                return 'system';
            default:
                return 'agent';
        }
    }

    formatMessageContent(message) {
        if (!message.content) return 'No content';

        switch (message.content.type) {
            case 'notification':
                return this.escapeHtml(message.content.text || 'No text');
            case 'question':
                return `❓ ${this.escapeHtml(message.content.text || 'No question')}`;
            case 'authorization':
                return `🔐 Authorization: ${this.escapeHtml(message.content.action || 'Unknown action')}`;
            case 'response':
                const answer = message.content.answer || '(no answer)';
                return `📤 Response: ${this.escapeHtml(answer)}`;
            default:
                return JSON.stringify(message.content, null, 2);
        }
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
}

// Initialize the web UI when the page loads
document.addEventListener('DOMContentLoaded', () => {
    new AiloopWebUI();
});