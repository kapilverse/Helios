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
      <div style={{ padding: '8px 16px', borderBottom: '1px solid #334155', display: 'flex', gap: 8, alignItems: 'center' }}>
        <span style={{ color: '#94a3b8', fontSize: 14 }}>Cursors:</span>
        {cursors.map((c, i) => (
          <span key={i} style={{ display: 'inline-flex', alignItems: 'center', gap: 4, padding: '2px 8px', background: '#1e293b', borderRadius: 4, fontSize: 12 }}>
            <span style={{ width: 8, height: 8, borderRadius: '50%', background: c.color }} />
            {c.name}
          </span>
        ))}
      </div>
      <textarea
        ref={textRef}
        onKeyDown={handleKeyDown}
        placeholder="Start typing..."
        style={{
          flex: 1,
          padding: '16px',
          fontFamily: "'SF Mono', 'Fira Code', monospace",
          fontSize: 14,
          lineHeight: 1.6,
          outline: 'none',
          background: '#1e293b',
          color: '#e2e8f0',
          border: 'none',
          borderRadius: '0 0 8px 8px',
          resize: 'none',
          width: '100%',
          boxSizing: 'border-box',
        }}
      />
    </div>
  );
}
