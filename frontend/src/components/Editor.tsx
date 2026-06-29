import React, { useRef, useEffect, useCallback } from 'react';

interface EditorProps {
  content: string;
  cursors: { name: string; color: string }[];
  onInsert: (pos: number, ch: string) => void;
  onDelete: (pos: number) => void;
}

export function Editor({ content, cursors, onInsert, onDelete }: EditorProps) {
  const textRef = useRef<HTMLTextAreaElement>(null);
  const localContentRef = useRef('');
  const isLocalChange = useRef(false);

  // Only sync from server when it's NOT a local change
  useEffect(() => {
    if (isLocalChange.current) {
      isLocalChange.current = false;
      return;
    }
    const el = textRef.current;
    if (el && el.value !== content) {
      const pos = el.selectionStart;
      el.value = content;
      el.setSelectionRange(pos, pos);
    }
    localContentRef.current = content;
  }, [content]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const el = e.currentTarget;
    const pos = el.selectionStart;
    const val = el.value;

    if (e.key === 'Backspace') {
      if (pos > 0) {
        e.preventDefault();
        isLocalChange.current = true;
        const newContent = val.slice(0, pos - 1) + val.slice(pos);
        el.value = newContent;
        el.setSelectionRange(pos - 1, pos - 1);
        localContentRef.current = newContent;
        onDelete(pos - 1);
      }
      return;
    }
    if (e.key === 'Delete') {
      if (pos < val.length) {
        e.preventDefault();
        isLocalChange.current = true;
        const newContent = val.slice(0, pos) + val.slice(pos + 1);
        el.value = newContent;
        el.setSelectionRange(pos, pos);
        localContentRef.current = newContent;
        onDelete(pos);
      }
      return;
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      isLocalChange.current = true;
      const newContent = val.slice(0, pos) + '\n' + val.slice(pos);
      el.value = newContent;
      el.setSelectionRange(pos + 1, pos + 1);
      localContentRef.current = newContent;
      onInsert(pos, '\n');
      return;
    }
    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      isLocalChange.current = true;
      const newContent = val.slice(0, pos) + e.key + val.slice(pos);
      el.value = newContent;
      el.setSelectionRange(pos + 1, pos + 1);
      localContentRef.current = newContent;
      onInsert(pos, e.key);
    }
  }, [onInsert, onDelete]);

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
              {c.name}
            </span>
          ))}
        </div>
      </div>
      <div style={{ flex: 1, position: 'relative' }}>
        <textarea
          ref={textRef}
          onKeyDown={handleKeyDown}
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
