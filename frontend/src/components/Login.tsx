import React, { useState } from 'react';

interface LoginProps {
  onLogin: (name: string) => void;
}

export function Login({ onLogin }: LoginProps) {
  const [name, setName] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (name.trim()) {
      onLogin(name.trim());
    }
  };

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '100vh',
        width: '100%',
        padding: 24,
      }}
    >
      <div
        className="glass-panel animate-fade-in"
        style={{
          borderRadius: 24,
          padding: '48px 40px',
          width: '100%',
          maxWidth: 420,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
        }}
      >
        <div style={{
          width: 64,
          height: 64,
          background: 'linear-gradient(135deg, var(--primary) 0%, #8b5cf6 100%)',
          borderRadius: 16,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          marginBottom: 24,
          boxShadow: '0 8px 32px rgba(56, 189, 248, 0.3)'
        }}>
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#0f172a" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
          </svg>
        </div>
        
        <h1
          style={{
            fontSize: 32,
            fontWeight: 800,
            letterSpacing: '-0.02em',
            color: '#fff',
            marginBottom: 8,
            textAlign: 'center',
          }}
        >
          HELIOS
        </h1>
        <p
          style={{
            color: 'var(--text-muted)',
            textAlign: 'center',
            marginBottom: 32,
            fontSize: 15,
            lineHeight: 1.5,
          }}
        >
          Real-time collaborative editing with absolute convergence.
        </p>
        <form onSubmit={handleSubmit} style={{ width: '100%' }}>
          <div style={{ position: 'relative', marginBottom: 24 }}>
            <input
              type="text"
              placeholder="Enter your display name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
              style={{
                width: '100%',
                padding: '14px 16px',
                background: 'rgba(15, 23, 42, 0.6)',
                border: '1px solid rgba(255, 255, 255, 0.1)',
                borderRadius: 12,
                color: '#fff',
                fontSize: 15,
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
            style={{ width: '100%', padding: '14px' }}
          >
            Enter Workspace
          </button>
        </form>
      </div>
    </div>
  );
}
