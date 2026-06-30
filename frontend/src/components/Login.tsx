import React, { useState } from 'react';

interface LoginProps {
  onLogin: (name: string, documentId: string) => void;
}

export function Login({ onLogin }: LoginProps) {
  const [name, setName] = useState('');
  const [documentId, setDocumentId] = useState('default');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (name.trim()) {
      onLogin(name.trim(), documentId.trim() || 'default');
    }
  };

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '100%',
        width: '100%',
        padding: 20,
        paddingTop: 'calc(20px + var(--safe-top))',
      }}
    >
      <div
        className="glass-panel animate-fade-in"
        style={{
          borderRadius: 20,
          padding: '36px 28px',
          width: '100%',
          maxWidth: 380,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
        }}
      >
        <div style={{
          width: 56,
          height: 56,
          background: 'linear-gradient(135deg, var(--primary) 0%, #8b5cf6 100%)',
          borderRadius: 14,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          marginBottom: 20,
          boxShadow: '0 8px 32px rgba(56, 189, 248, 0.3)'
        }}>
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#0f172a" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
          </svg>
        </div>

        <h1
          style={{
            fontSize: 28,
            fontWeight: 800,
            letterSpacing: '-0.02em',
            color: '#fff',
            marginBottom: 6,
            textAlign: 'center',
          }}
        >
          HELIOS
        </h1>
        <p
          style={{
            color: 'var(--text-muted)',
            textAlign: 'center',
            marginBottom: 28,
            fontSize: 14,
            lineHeight: 1.5,
          }}
        >
          Real-time collaborative editing with absolute convergence.
        </p>
        <form onSubmit={handleSubmit} style={{ width: '100%' }}>
          <div style={{ marginBottom: 16 }}>
            <input
              type="text"
              placeholder="Enter your display name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
              autoComplete="name"
              style={{
                width: '100%',
                padding: '14px 16px',
                background: 'rgba(15, 23, 42, 0.6)',
                border: '1px solid rgba(255, 255, 255, 0.1)',
                borderRadius: 12,
                color: '#fff',
                fontSize: 16,
                outline: 'none',
                transition: 'all 0.3s ease',
              }}
              onFocus={(e) => {
                e.target.style.borderColor = 'var(--primary)';
                e.target.style.boxShadow = '0 0 0 4px rgba(56, 189, 248, 0.1)';
              }}
              onBlur={(e) => {
                e.target.style.borderColor = 'rgba(255, 255, 255, 0.1)';
                e.target.style.boxShadow = 'none';
              }}
            />
          </div>
          <div style={{ marginBottom: 20 }}>
            <input
              type="text"
              placeholder="Document name"
              value={documentId}
              onChange={(e) => setDocumentId(e.target.value)}
              autoComplete="off"
              style={{
                width: '100%',
                padding: '14px 16px',
                background: 'rgba(15, 23, 42, 0.6)',
                border: '1px solid rgba(255, 255, 255, 0.1)',
                borderRadius: 12,
                color: '#fff',
                fontSize: 16,
                outline: 'none',
                transition: 'all 0.3s ease',
              }}
              onFocus={(e) => {
                e.target.style.borderColor = 'var(--primary)';
                e.target.style.boxShadow = '0 0 0 4px rgba(56, 189, 248, 0.1)';
              }}
              onBlur={(e) => {
                e.target.style.borderColor = 'rgba(255, 255, 255, 0.1)';
                e.target.style.boxShadow = 'none';
              }}
            />
          </div>
          <button
            type="submit"
            disabled={!name.trim()}
            className="btn-primary"
            style={{ width: '100%', padding: '14px', fontSize: 16 }}
          >
            Enter Workspace
          </button>
        </form>
      </div>
    </div>
  );
}