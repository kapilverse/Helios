import React, { useState } from 'react';
import { Login } from './components/Login';
import { Editor } from './components/Editor';
import { useHelios } from './hooks/useHelios';

export default function App() {
  const [userName, setUserName] = useState<string | null>(null);

  const wsUrl =
    window.location.protocol === 'https:'
      ? `wss://${window.location.host}/ws`
      : `ws://${window.location.hostname}:3000/ws`;

  const { connected, content, cursors, applyLocalText } = useHelios(wsUrl);

  if (!userName) {
    return <Login onLogin={setUserName} />;
  }

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div
        className="glass-panel"
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '16px 24px',
          borderBottom: '1px solid var(--border-glass)',
          borderTop: 'none',
          borderLeft: 'none',
          borderRight: 'none',
          borderRadius: 0,
          zIndex: 10,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <div style={{
            background: 'linear-gradient(135deg, var(--primary) 0%, #8b5cf6 100%)',
            WebkitBackgroundClip: 'text',
            WebkitTextFillColor: 'transparent',
            fontWeight: 800,
            fontSize: 20,
            letterSpacing: '-0.02em',
          }}>
            HELIOS
          </div>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '4px 10px',
              borderRadius: 20,
              background: connected ? 'rgba(16, 185, 129, 0.1)' : 'rgba(239, 68, 68, 0.1)',
              border: `1px solid ${connected ? 'rgba(16, 185, 129, 0.2)' : 'rgba(239, 68, 68, 0.2)'}`,
            }}
          >
            <div style={{
              width: 6,
              height: 6,
              borderRadius: '50%',
              background: connected ? '#10b981' : '#ef4444',
              boxShadow: connected ? '0 0 8px #10b981' : '0 0 8px #ef4444',
            }} className="pulse-dot" />
            <span style={{ fontSize: 12, fontWeight: 500, color: connected ? '#34d399' : '#f87171' }}>
              {connected ? 'Live' : 'Connecting'}
            </span>
          </div>
        </div>
        
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, background: 'rgba(15, 23, 42, 0.5)', padding: '6px 14px', borderRadius: 20, border: '1px solid var(--border-glass)' }}>
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              background: '#8b5cf6',
              boxShadow: '0 0 10px #8b5cf6'
            }}
          />
          <span style={{ fontSize: 13, fontWeight: 500, color: '#e2e8f0' }}>{userName}</span>
        </div>
      </div>

      {/* Editor */}
      <div style={{ flex: 1, padding: '24px', display: 'flex', flexDirection: 'column' }}>
        <div className="glass-panel animate-fade-in" style={{ flex: 1, display: 'flex', flexDirection: 'column', borderRadius: 16, overflow: 'hidden' }}>
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
