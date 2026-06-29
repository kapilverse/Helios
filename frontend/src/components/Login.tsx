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
        height: '100vh',
        background: '#0f172a',
      }}
    >
      <div
        style={{
          background: '#1e293b',
          borderRadius: 12,
          padding: 32,
          width: 360,
          boxShadow: '0 4px 24px rgba(0,0,0,0.3)',
        }}
      >
        <h1
          style={{
            fontSize: 28,
            fontWeight: 700,
            color: '#38bdf8',
            marginBottom: 8,
            textAlign: 'center',
          }}
        >
          HELIOS
        </h1>
        <p
          style={{
            color: '#94a3b8',
            textAlign: 'center',
            marginBottom: 24,
            fontSize: 14,
          }}
        >
          Collaborative real-time editing
        </p>
        <form onSubmit={handleSubmit}>
          <input
            type="text"
            placeholder="Enter your name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            autoFocus
            style={{
              width: '100%',
              padding: '12px 16px',
              background: '#0f172a',
              border: '1px solid #334155',
              borderRadius: 8,
              color: '#e2e8f0',
              fontSize: 14,
              outline: 'none',
              marginBottom: 16,
            }}
          />
          <button
            type="submit"
            disabled={!name.trim()}
            style={{
              width: '100%',
              padding: '12px',
              background: name.trim() ? '#38bdf8' : '#334155',
              border: 'none',
              borderRadius: 8,
              color: name.trim() ? '#0f172a' : '#64748b',
              fontSize: 14,
              fontWeight: 600,
              cursor: name.trim() ? 'pointer' : 'not-allowed',
            }}
          >
            Join Document
          </button>
        </form>
      </div>
    </div>
  );
}
