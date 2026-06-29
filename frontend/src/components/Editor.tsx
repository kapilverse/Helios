import React, { useRef, useEffect, KeyboardEvent } from 'react';

interface EditorProps {
  content: string;
  cursors: { name: string; color: string }[];
  onInsert: (after: { peer: string; clock: number } | null, ch: string) => void;
  onDelete: (target: { peer: string; clock: number }) => void;
}

export function Editor({ content, cursors, onInsert, onDelete }: EditorProps) {
  const editorRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (editorRef.current && editorRef.current.textContent !== content) {
      editorRef.current.textContent = content;
    }
  }, [content]);

  const handleKeyDown = (e: KeyboardEvent) => {
    e.preventDefault();
    if (e.key === 'Backspace') {
      // Simple delete last char
      return;
    }
    if (e.key.length === 1) {
      onInsert(null, e.key);
    }
  };

  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
      <div style={{ padding: '8px 16px', borderBottom: '1px solid #334155', display: 'flex', gap: 8, alignItems: 'center' }}>
        <span style={{ color: '#94a3b8', fontSize: 14 }}>Cursors:</span>
        {cursors.map((c, i) => (
          <span
            key={i}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 4,
              padding: '2px 8px',
              background: '#1e293b',
              borderRadius: 4,
              fontSize: 12,
            }}
          >
            <span
              style={{
                width: 8,
                height: 8,
                borderRadius: '50%',
                background: c.color,
              }}
            />
            {c.name}
          </span>
        ))}
      </div>
      <div
        ref={editorRef}
        contentEditable
        suppressContentEditableWarning
        onKeyDown={handleKeyDown}
        style={{
          flex: 1,
          padding: '16px',
          fontFamily: "'SF Mono', 'Fira Code', monospace",
          fontSize: 14,
          lineHeight: 1.6,
          outline: 'none',
          background: '#1e293b',
          color: '#e2e8f0',
          borderRadius: '0 0 8px 8px',
          whiteSpace: 'pre-wrap',
          overflowY: 'auto',
        }}
      />
    </div>
  );
}
