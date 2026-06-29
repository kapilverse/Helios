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

  const { connected, content, cursors, insertChar, deleteChar } = useHelios(wsUrl);

  if (!userName) {
    return <Login onLogin={setUserName} />;
  }

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column', background: '#0f172a' }}>
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '12px 20px',
          borderBottom: '1px solid #1e293b',
          background: '#0f172a',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <h1 style={{ fontSize: 18, fontWeight: 700, color: '#38bdf8' }}>HELIOS</h1>
          <span
            style={{
              fontSize: 12,
              padding: '2px 8px',
              borderRadius: 4,
              background: connected ? '#064e3b' : '#7f1d1d',
              color: connected ? '#6ee7b7' : '#fca5a5',
            }}
          >
            {connected ? 'Connected' : 'Connecting...'}
          </span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              background: '#3b82f6',
            }}
          />
          <span style={{ fontSize: 13, color: '#94a3b8' }}>{userName}</span>
        </div>
      </div>

      {/* Editor */}
      <Editor
        content={content}
        cursors={cursors}
        onInsert={insertChar}
        onDelete={deleteChar}
      />
    </div>
  );
}
