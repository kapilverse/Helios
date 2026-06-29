import React, { useRef, useEffect, useCallback } from 'react';

interface EditorProps {
  content: string;
  cursors: { name: string; color: string }[];
  onInsert: (pos: number, ch: string) => void;
  onDelete: (pos: number) => void;
}

export function Editor({ content, cursors, onInsert, onDelete }: EditorProps) {
  const editorRef = useRef<HTMLDivElement>(null);
  const cursorPosRef = useRef(0);
  const composingRef = useRef(false);

  useEffect(() => {
    if (!editorRef.current) return;
    const el = editorRef.current;

    if (el.textContent !== content) {
      const savedPos = cursorPosRef.current;
      el.textContent = content;
      // Restore cursor position
      if (el.firstChild) {
        const range = document.createRange();
        const sel = window.getSelection();
        const pos = Math.min(savedPos, el.textContent.length);
        range.setStart(el.firstChild, pos);
        range.collapse(true);
        sel?.removeAllRanges();
        sel?.addRange(range);
        cursorPosRef.current = pos;
      }
    }
  }, [content]);

  const updateCursorPos = useCallback(() => {
    const sel = window.getSelection();
    if (sel && sel.rangeCount > 0) {
      const range = sel.getRangeAt(0);
      const preRange = document.createRange();
      preRange.selectNodeContents(editorRef.current!);
      preRange.setEnd(range.startContainer, range.startOffset);
      cursorPosRef.current = preRange.toString().length;
    }
  }, []);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (composingRef.current) return;

    updateCursorPos();
    const pos = cursorPosRef.current;

    if (e.key === 'Backspace') {
      e.preventDefault();
      if (pos > 0) {
        onDelete(pos - 1);
      }
      return;
    }

    if (e.key === 'Delete') {
      e.preventDefault();
      if (pos < content.length) {
        onDelete(pos);
      }
      return;
    }

    if (e.key === 'Enter') {
      e.preventDefault();
      onInsert(pos, '\n');
      return;
    }

    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      onInsert(pos, e.key);
    }
  }, [content, onInsert, onDelete, updateCursorPos]);

  const handleInput = useCallback(() => {
    updateCursorPos();
  }, [updateCursorPos]);

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
            <span style={{ width: 8, height: 8, borderRadius: '50%', background: c.color }} />
            {c.name}
          </span>
        ))}
      </div>
      <div
        ref={editorRef}
        contentEditable
        suppressContentEditableWarning
        onKeyDown={handleKeyDown}
        onInput={handleInput}
        onClick={updateCursorPos}
        onKeyUp={updateCursorPos}
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
          wordBreak: 'break-all',
          overflowY: 'auto',
          minHeight: 200,
        }}
      />
    </div>
  );
}
