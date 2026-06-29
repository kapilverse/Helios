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
        </div>
        
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <span style={{ fontSize: 14, fontWeight: 500, color: userColor }}>{userName}</span>
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
