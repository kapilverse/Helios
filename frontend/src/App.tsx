import React, { useState } from 'react';
import { Login } from './components/Login';
import { Editor } from './components/Editor';
import { useHelios } from './hooks/useHelios';

export default function App() {
  const [userName, setUserName] = useState<string | null>(null);
  const [userColor, setUserColor] = useState<string>('#3b82f6');
  const [documentId, setDocumentId] = useState<string>('default');

  const wsUrl = `${window.location.origin.replace(/^http/, 'ws')}/ws`;

  const { connected, content, cursors, applyLocalText } = useHelios(wsUrl, documentId, userName || 'Anonymous', userColor);

  if (!userName) {
    return <Login onLogin={(name, docId) => {
      setUserName(name);
      setDocumentId(docId);
      const colors = ['#f87171', '#fb923c', '#fbbf24', '#a3e635', '#4ade80', '#2dd4bf', '#38bdf8', '#818cf8', '#a78bfa', '#e879f9', '#fb7185'];
      setUserColor(colors[Math.floor(Math.random() * colors.length)]);
    }} />;
  }

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div
        className="glass-panel"
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '12px 16px',
          paddingTop: 'calc(12px + var(--safe-top))',
          borderBottom: '1px solid var(--border-glass)',
          borderTop: 'none',
          borderLeft: 'none',
          borderRight: 'none',
          borderRadius: 0,
          zIndex: 10,
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <div style={{
            background: 'linear-gradient(135deg, var(--primary) 0%, #8b5cf6 100%)',
            WebkitBackgroundClip: 'text',
            WebkitTextFillColor: 'transparent',
            fontWeight: 800,
            fontSize: 18,
            letterSpacing: '-0.02em',
          }}>
            HELIOS
          </div>
          {connected && (
            <span style={{
              fontSize: 11,
              fontWeight: 600,
              color: '#4ade80',
              background: 'rgba(74, 222, 128, 0.1)',
              padding: '2px 8px',
              borderRadius: 999,
              display: 'none',
            }} className="status-badge">
              Connected
            </span>
          )}
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <button
            onClick={() => {
              const element = document.createElement("a");
              const file = new Blob([content], {type: 'text/plain'});
              element.href = URL.createObjectURL(file);
              element.download = `${documentId}.txt`;
              document.body.appendChild(element);
              element.click();
              document.body.removeChild(element);
            }}
            style={{
              padding: '6px 12px',
              fontSize: 12,
              fontWeight: 600,
              color: '#fff',
              background: 'rgba(255, 255, 255, 0.1)',
              border: '1px solid rgba(255, 255, 255, 0.2)',
              borderRadius: 6,
              cursor: 'pointer',
              transition: 'background 0.2s',
            }}
            onMouseOver={(e) => e.currentTarget.style.background = 'rgba(255, 255, 255, 0.2)'}
            onMouseOut={(e) => e.currentTarget.style.background = 'rgba(255, 255, 255, 0.1)'}
          >
            Download
          </button>
          <button
            onClick={() => {
              setUserName(null);
            }}
            style={{
              padding: '6px 12px',
              fontSize: 12,
              fontWeight: 600,
              color: '#0f172a',
              background: 'var(--primary, #38bdf8)',
              border: 'none',
              borderRadius: 6,
              cursor: 'pointer',
              transition: 'opacity 0.2s',
            }}
            onMouseOver={(e) => e.currentTarget.style.opacity = '0.9'}
            onMouseOut={(e) => e.currentTarget.style.opacity = '1'}
          >
            Create New Document
          </button>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginLeft: 4 }}>
            <span style={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              background: connected ? '#4ade80' : '#f87171',
              flexShrink: 0,
            }} />
            <span style={{ fontSize: 13, fontWeight: 500, color: userColor }}>{userName}</span>
          </div>
        </div>
      </div>

      <div style={{ flex: 1, padding: '12px', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
        <div className="glass-panel animate-fade-in" style={{ flex: 1, display: 'flex', flexDirection: 'column', borderRadius: 12, overflow: 'hidden', minHeight: 0 }}>
          <Editor
            content={content}
            cursors={cursors}
            onTextChange={applyLocalText}
          />
        </div>
      </div>
    </div>
  );
}