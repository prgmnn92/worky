//! HTML templates for the kanban board.

pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>worky - Kanban Board</title>
    <link rel="stylesheet" href="/styles.css">
    <script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
</head>
<body>
    <header>
        <h1>📋 worky Board</h1>
        <button id="refresh-btn" onclick="loadItems()">↻ Refresh</button>
    </header>

    <main id="board">
        <div class="column" data-state="TODO">
            <div class="column-header">
                <span class="column-title">📝 TODO</span>
                <span class="column-count">0</span>
            </div>
            <div class="cards"></div>
        </div>
        <div class="column" data-state="IN_PROGRESS">
            <div class="column-header">
                <span class="column-title">🔄 In Progress</span>
                <span class="column-count">0</span>
            </div>
            <div class="cards"></div>
        </div>
        <div class="column" data-state="IN_REVIEW">
            <div class="column-header">
                <span class="column-title">👀 In Review</span>
                <span class="column-count">0</span>
            </div>
            <div class="cards"></div>
        </div>
        <div class="column" data-state="BLOCKED">
            <div class="column-header">
                <span class="column-title">🚫 Blocked</span>
                <span class="column-count">0</span>
            </div>
            <div class="cards"></div>
        </div>
        <div class="column" data-state="DONE">
            <div class="column-header">
                <span class="column-title">✅ Done</span>
                <span class="column-count">0</span>
            </div>
            <div class="cards"></div>
        </div>
    </main>

    <div id="modal" class="modal hidden">
        <div class="modal-content">
            <span class="close" onclick="closeModal()">&times;</span>
            <div id="modal-body"></div>
        </div>
    </div>

    <script>
        const STATES = ['TODO', 'IN_PROGRESS', 'IN_REVIEW', 'BLOCKED', 'DONE'];

        async function loadItems() {
            try {
                const response = await fetch('/api/items');
                const data = await response.json();

                if (data.error) {
                    alert('Error: ' + data.error);
                    return;
                }

                renderBoard(data.items);
            } catch (e) {
                alert('Failed to load items: ' + e.message);
            }
        }

        function renderBoard(items) {
            // Clear all columns
            STATES.forEach(state => {
                const column = document.querySelector(`[data-state="${state}"] .cards`);
                column.innerHTML = '';
            });

            // Group items by state
            const grouped = {};
            STATES.forEach(s => grouped[s] = []);

            items.forEach(item => {
                const state = STATES.includes(item.state) ? item.state : 'TODO';
                grouped[state].push(item);
            });

            // Render items
            STATES.forEach(state => {
                const column = document.querySelector(`[data-state="${state}"] .cards`);
                const countEl = document.querySelector(`[data-state="${state}"] .column-count`);
                countEl.textContent = grouped[state].length;

                grouped[state].forEach(item => {
                    column.appendChild(createCard(item));
                });
            });
        }

        function createCard(item) {
            const card = document.createElement('div');
            card.className = 'card';
            card.onclick = () => showDetail(item);

            const title = document.createElement('div');
            title.className = 'card-title';
            title.textContent = item.title;
            card.appendChild(title);

            const uid = document.createElement('div');
            uid.className = 'card-uid';
            uid.textContent = item.uid;
            card.appendChild(uid);

            if (item.assignee) {
                const assignee = document.createElement('div');
                assignee.className = 'card-assignee';
                assignee.textContent = '👤 ' + item.assignee;
                card.appendChild(assignee);
            }

            if (item.labels && item.labels.length > 0) {
                const labels = document.createElement('div');
                labels.className = 'card-labels';
                item.labels.forEach(label => {
                    const tag = document.createElement('span');
                    tag.className = 'label';
                    tag.textContent = label;
                    labels.appendChild(tag);
                });
                card.appendChild(labels);
            }

            const meta = document.createElement('div');
            meta.className = 'card-meta';
            meta.textContent = 'Updated: ' + item.updated_at;
            card.appendChild(meta);

            return card;
        }

        function showDetail(item) {
            const modal = document.getElementById('modal');
            const body = document.getElementById('modal-body');

            let html = `
                <h2>${escapeHtml(item.title)}</h2>
                <div class="detail-row">
                    <strong>UID:</strong> <code>${escapeHtml(item.uid)}</code>
                </div>
                <div class="detail-row">
                    <strong>State:</strong> <span class="state-badge state-${item.state.toLowerCase()}">${item.state}</span>
                </div>
            `;

            if (item.assignee) {
                html += `<div class="detail-row"><strong>Assignee:</strong> ${escapeHtml(item.assignee)}</div>`;
            }

            if (item.labels && item.labels.length > 0) {
                html += `<div class="detail-row"><strong>Labels:</strong> ${item.labels.map(l => `<span class="label">${escapeHtml(l)}</span>`).join(' ')}</div>`;
            }

            html += `
                <div class="detail-row"><strong>Created:</strong> ${item.created_at}</div>
                <div class="detail-row"><strong>Updated:</strong> ${item.updated_at}</div>
            `;

            // Check for description field (render as markdown)
            if (item.fields && item.fields.description) {
                html += `<h3>Description</h3><div class="markdown-content">${renderMarkdown(item.fields.description)}</div>`;
            }

            if (item.fields && Object.keys(item.fields).length > 0) {
                const otherFields = Object.entries(item.fields).filter(([k]) => k !== 'description');
                if (otherFields.length > 0) {
                    html += `<h3>Custom Fields</h3>`;
                    for (const [key, value] of otherFields) {
                        // Render string values as markdown if they contain markdown-like syntax
                        const valueStr = String(value);
                        if (typeof value === 'string' && (valueStr.includes('```') || valueStr.includes('- ') || valueStr.includes('# ') || valueStr.includes('**'))) {
                            html += `<div class="detail-row"><strong>${escapeHtml(key)}:</strong></div><div class="markdown-content">${renderMarkdown(valueStr)}</div>`;
                        } else {
                            html += `<div class="detail-row"><strong>${escapeHtml(key)}:</strong> ${escapeHtml(valueStr)}</div>`;
                        }
                    }
                }
            }

            if (item.comments && item.comments.length > 0) {
                html += `<h3>Comments (${item.comments.length})</h3><div class="comments">`;
                item.comments.forEach(c => {
                    html += `
                        <div class="comment">
                            <div class="comment-header">
                                <span class="comment-author">${escapeHtml(c.actor || 'user')}</span>
                                <span class="comment-time">${c.timestamp}</span>
                            </div>
                            <div class="comment-body markdown-content">${renderMarkdown(c.message)}</div>
                        </div>
                    `;
                });
                html += `</div>`;
            }

            body.innerHTML = html;
            modal.classList.remove('hidden');
        }

        function closeModal() {
            document.getElementById('modal').classList.add('hidden');
        }

        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        function renderMarkdown(text) {
            if (!text) return '';
            // Configure marked for safe rendering
            marked.setOptions({
                breaks: true,      // Convert \n to <br>
                gfm: true,         // GitHub Flavored Markdown
                headerIds: false,  // Don't add IDs to headers
                mangle: false      // Don't mangle email addresses
            });
            return marked.parse(text);
        }

        // Close modal on escape key
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') closeModal();
        });

        // Close modal on outside click
        document.getElementById('modal').addEventListener('click', (e) => {
            if (e.target.id === 'modal') closeModal();
        });

        // Load items on page load
        loadItems();

        // Auto-refresh every 30 seconds
        setInterval(loadItems, 30000);
    </script>
</body>
</html>
"#;

pub const STYLES_CSS: &str = r#"
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    background: #1a1a2e;
    color: #eee;
    min-height: 100vh;
}

header {
    background: #16213e;
    padding: 1rem 2rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid #0f3460;
}

header h1 {
    font-size: 1.5rem;
    font-weight: 600;
}

#refresh-btn {
    background: #0f3460;
    color: #eee;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.2s;
}

#refresh-btn:hover {
    background: #1a4a7a;
}

main {
    display: flex;
    gap: 1rem;
    padding: 1rem;
    overflow-x: auto;
    min-height: calc(100vh - 60px);
}

.column {
    background: #16213e;
    border-radius: 8px;
    min-width: 280px;
    max-width: 320px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
}

.column-header {
    padding: 1rem;
    border-bottom: 1px solid #0f3460;
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.column-title {
    font-weight: 600;
    font-size: 0.9rem;
}

.column-count {
    background: #0f3460;
    color: #94a3b8;
    padding: 0.2rem 0.6rem;
    border-radius: 12px;
    font-size: 0.8rem;
}

.cards {
    padding: 0.5rem;
    flex: 1;
    overflow-y: auto;
}

.card {
    background: #1a1a2e;
    border: 1px solid #0f3460;
    border-radius: 6px;
    padding: 0.75rem;
    margin-bottom: 0.5rem;
    cursor: pointer;
    transition: transform 0.1s, box-shadow 0.1s;
}

.card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    border-color: #e94560;
}

.card-title {
    font-weight: 500;
    margin-bottom: 0.5rem;
    line-height: 1.3;
}

.card-uid {
    font-size: 0.75rem;
    color: #64748b;
    font-family: monospace;
    margin-bottom: 0.5rem;
}

.card-assignee {
    font-size: 0.8rem;
    color: #94a3b8;
    margin-bottom: 0.5rem;
}

.card-labels {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-bottom: 0.5rem;
}

.label {
    background: #0f3460;
    color: #60a5fa;
    padding: 0.15rem 0.5rem;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 500;
}

.card-meta {
    font-size: 0.7rem;
    color: #64748b;
}

/* Modal */
.modal {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
}

.modal.hidden {
    display: none;
}

.modal-content {
    background: #16213e;
    border-radius: 8px;
    padding: 1.5rem;
    max-width: 600px;
    width: 90%;
    max-height: 80vh;
    overflow-y: auto;
    position: relative;
}

.close {
    position: absolute;
    top: 1rem;
    right: 1rem;
    font-size: 1.5rem;
    cursor: pointer;
    color: #94a3b8;
}

.close:hover {
    color: #e94560;
}

.modal-content h2 {
    margin-bottom: 1rem;
    padding-right: 2rem;
}

.modal-content h3 {
    margin-top: 1.5rem;
    margin-bottom: 0.75rem;
    font-size: 1rem;
    color: #94a3b8;
}

.detail-row {
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
}

.detail-row strong {
    color: #94a3b8;
}

.detail-row code {
    background: #0f3460;
    padding: 0.2rem 0.4rem;
    border-radius: 3px;
    font-size: 0.85rem;
}

.state-badge {
    padding: 0.2rem 0.6rem;
    border-radius: 4px;
    font-size: 0.8rem;
    font-weight: 500;
}

.state-todo { background: #3b82f6; color: white; }
.state-in_progress { background: #f59e0b; color: black; }
.state-in_review { background: #8b5cf6; color: white; }
.state-blocked { background: #ef4444; color: white; }
.state-done { background: #22c55e; color: white; }

.comments {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}

.comment {
    background: #1a1a2e;
    border-radius: 6px;
    padding: 0.75rem;
}

.comment-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.5rem;
    font-size: 0.8rem;
}

.comment-author {
    color: #60a5fa;
    font-weight: 500;
}

.comment-time {
    color: #64748b;
}

.comment-body {
    font-size: 0.9rem;
    line-height: 1.4;
}

/* Markdown content styling */
.markdown-content {
    font-size: 0.9rem;
    line-height: 1.6;
}

.markdown-content h1,
.markdown-content h2,
.markdown-content h3,
.markdown-content h4 {
    margin-top: 1rem;
    margin-bottom: 0.5rem;
    color: #eee;
}

.markdown-content h1 { font-size: 1.4rem; }
.markdown-content h2 { font-size: 1.2rem; }
.markdown-content h3 { font-size: 1.1rem; }
.markdown-content h4 { font-size: 1rem; }

.markdown-content p {
    margin-bottom: 0.75rem;
}

.markdown-content ul,
.markdown-content ol {
    margin-left: 1.5rem;
    margin-bottom: 0.75rem;
}

.markdown-content li {
    margin-bottom: 0.25rem;
}

.markdown-content code {
    background: #0f3460;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    font-family: 'Fira Code', 'Monaco', 'Consolas', monospace;
    font-size: 0.85em;
}

.markdown-content pre {
    background: #0f3460;
    padding: 1rem;
    border-radius: 6px;
    overflow-x: auto;
    margin-bottom: 0.75rem;
}

.markdown-content pre code {
    background: none;
    padding: 0;
    font-size: 0.85rem;
    line-height: 1.5;
}

.markdown-content blockquote {
    border-left: 3px solid #e94560;
    padding-left: 1rem;
    margin-left: 0;
    margin-bottom: 0.75rem;
    color: #94a3b8;
    font-style: italic;
}

.markdown-content a {
    color: #60a5fa;
    text-decoration: none;
}

.markdown-content a:hover {
    text-decoration: underline;
}

.markdown-content table {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 0.75rem;
}

.markdown-content th,
.markdown-content td {
    border: 1px solid #0f3460;
    padding: 0.5rem;
    text-align: left;
}

.markdown-content th {
    background: #0f3460;
}

.markdown-content hr {
    border: none;
    border-top: 1px solid #0f3460;
    margin: 1rem 0;
}

.markdown-content img {
    max-width: 100%;
    border-radius: 4px;
}

/* Scrollbar styling */
::-webkit-scrollbar {
    width: 8px;
    height: 8px;
}

::-webkit-scrollbar-track {
    background: #1a1a2e;
}

::-webkit-scrollbar-thumb {
    background: #0f3460;
    border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
    background: #1a4a7a;
}
"#;
