import React, { useRef, useEffect, useCallback } from 'react';

interface EditorProps {
  content: string;
  cursors: { name: string; color: string; selectionStart?: { peer: string; clock: number } | null; selectionEnd?: { peer: string; clock: number } | null }[];
  onTextChange: (nextContent: string, selectionStart: number | null, selectionEnd: number | null) => void;
}

export function Editor({ content, cursors, onTextChange }: EditorProps) {
  const textRef = useRef<HTMLTextAreaElement>(null);
  const pendingSelectionRef = useRef<{ start: number; end: number } | null>(null);

  useEffect(() => {
    const el = textRef.current;
    if (!el) return;

    if (el.value !== content) {
      el.value = content;
    }

    if (pendingSelectionRef.current) {
      const { start, end } = pendingSelectionRef.current;
      el.setSelectionRange(start, end);
      pendingSelectionRef.current = null;
    }
  }, [content]);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    onTextChange(e.target.value, e.target.selectionStart, e.target.selectionEnd);
  }, [onTextChange]);

  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
      <div style={{ padding: '12px 20px', borderBottom: '1px solid var(--border-glass)', display: 'flex', gap: 12, alignItems: 'center', background: 'rgba(15, 23, 42, 0.4)' }}>
        <span style={{ color: 'var(--text-muted)', fontSize: 13, fontWeight: 500, display: 'flex', alignItems: 'center', gap: 6 }}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
            <circle cx="9" cy="7" r="4"></circle>
            <path d="M23 21v-2a4 4 0 0 0-3-3.87"></path>
            <path d="M16 3.13a4 4 0 0 1 0 7.75"></path>
          </svg>
          Collaborators:
        </span>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {cursors.length === 0 && <span style={{ color: 'var(--text-muted)', fontSize: 13 }}>Only you</span>}
          {cursors.map((c, i) => (
            <span key={i} className="cursor-tag" style={{ display: 'inline-flex', alignItems: 'center', gap: 6, padding: '4px 10px', background: 'rgba(255, 255, 255, 0.05)', border: '1px solid var(--border-glass)', borderRadius: 20, fontSize: 12, fontWeight: 500 }}>
              <span style={{ width: 8, height: 8, borderRadius: '50%', background: c.color, boxShadow: `0 0 8px ${c.color}` }} />
              <span>{c.name}</span>
              {(c.selectionStart != null && c.selectionEnd != null) ? (
                <span style={{ color: 'var(--text-muted)' }}>
                  {'cursor'}
                </span>
              ) : null}
            </span>
          ))}
        </div>
      </div>
      <div style={{ flex: 1, position: 'relative' }}>
        <textarea
          ref={textRef}
          value={content}
          onChange={handleChange}
          placeholder="Start typing your document..."
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            padding: '24px 32px',
            fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
            fontSize: 15,
            lineHeight: 1.7,
            outline: 'none',
            background: 'transparent',
            color: 'var(--text-main)',
            border: 'none',
            resize: 'none',
            width: '100%',
            height: '100%',
            boxSizing: 'border-box',
          }}
        />
      </div>
    </div>
  );
}
