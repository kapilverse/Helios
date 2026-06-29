import { useState, useEffect, useRef, useCallback } from 'react';
import { HeliosClient, ServerMessage, Op, OpId } from '../lib/ws';

interface Cursor {
  name: string;
  color: string;
  opId: OpId | null;
}

export function useHelios(url: string) {
  const clientRef = useRef<HeliosClient | null>(null);
  const [connected, setConnected] = useState(false);
  const [content, setContent] = useState('');
  const [cursors, setCursors] = useState<Cursor[]>([]);
  const contentRef = useRef('');
  const charIdsRef = useRef<OpId[]>([]);

  useEffect(() => {
    const client = new HeliosClient(url);
    clientRef.current = client;

    const unsubscribe = client.onMessage((msg: ServerMessage) => {
      if (msg.Sync) {
        setConnected(true);
        let text = '';
        const ids: OpId[] = [];
        for (const [, op] of msg.Sync.response.ops) {
          if ('Insert' in op) {
            const afterId = op.Insert.after;
            let pos = 0;
            if (afterId) {
              const idx = ids.findIndex(
                (id) => id.peer === afterId.peer && id.clock === afterId.clock
              );
              if (idx !== -1) pos = idx + 1;
              else pos = ids.length;
            }
            text = text.slice(0, pos) + op.Insert.content + text.slice(pos);
            ids.splice(pos, 0, op.Insert.id);
          } else if ('Delete' in op) {
            const idx = ids.findIndex(
              (id) => id.peer === op.Delete.target.peer && id.clock === op.Delete.target.clock
            );
            if (idx !== -1) {
              text = text.slice(0, idx) + text.slice(idx + 1);
              ids.splice(idx, 1);
            }
          }
        }
        contentRef.current = text;
        charIdsRef.current = ids;
        setContent(text);
      }

      if (msg.Op) {
        const op = msg.Op.op;
        const myPeerId = clientRef.current?.peerIdStr;

        if ('Insert' in op) {
          if (op.Insert.id.peer !== myPeerId) {
            // Find position: after the character with matching OpId
            const afterId = op.Insert.after;
            let pos = 0;
            if (afterId) {
              const idx = charIdsRef.current.findIndex(
                (id) => id.peer === afterId.peer && id.clock === afterId.clock
              );
              if (idx !== -1) pos = idx + 1;
              else pos = contentRef.current.length; // fallback
            }
            contentRef.current =
              contentRef.current.slice(0, pos) +
              op.Insert.content +
              contentRef.current.slice(pos);
            charIdsRef.current.splice(pos, 0, op.Insert.id);
            setContent(contentRef.current);
          }
        } else if ('Delete' in op) {
          const idx = charIdsRef.current.findIndex(
            (id) => id.peer === op.Delete.target.peer && id.clock === op.Delete.target.clock
          );
          if (idx !== -1) {
            contentRef.current =
              contentRef.current.slice(0, idx) + contentRef.current.slice(idx + 1);
            charIdsRef.current.splice(idx, 1);
            setContent(contentRef.current);
          }
        }
      }

      if (msg.Presence) {
        setCursors(
          msg.Presence.peers.map((p) => ({
            name: p.name,
            color: p.color,
            opId: p.op_id,
          }))
        );
      }
    });

    client.connect();

    return () => {
      unsubscribe();
      client.disconnect();
    };
  }, [url]);

  const insertChar = useCallback((pos: number, ch: string) => {
    const afterId = pos > 0 ? charIdsRef.current[pos - 1] ?? null : null;
    const id = clientRef.current!.nextOpId();

    // Optimistic local update
    contentRef.current =
      contentRef.current.slice(0, pos) + ch + contentRef.current.slice(pos);
    charIdsRef.current.splice(pos, 0, id);
    setContent(contentRef.current);

    // Send to server
    clientRef.current?.sendInsert(id, afterId, ch);
  }, []);

  const deleteChar = useCallback((pos: number) => {
    if (pos < 0 || pos >= charIdsRef.current.length) return;
    const target = charIdsRef.current[pos];

    // Optimistic local update
    contentRef.current =
      contentRef.current.slice(0, pos) + contentRef.current.slice(pos + 1);
    charIdsRef.current.splice(pos, 1);
    setContent(contentRef.current);

    clientRef.current?.sendDelete(target);
  }, []);

  const sendPresence = useCallback((cursor: OpId | null) => {
    clientRef.current?.sendPresence(cursor);
  }, []);

  const applyLocalText = useCallback((nextContent: string, selectionStart: number | null) => {
    const current = contentRef.current;
    if (nextContent === current) return;

    let prefix = 0;
    while (prefix < current.length && prefix < nextContent.length && current[prefix] === nextContent[prefix]) {
      prefix += 1;
    }

    let suffix = 0;
    while (
      suffix < current.length - prefix &&
      suffix < nextContent.length - prefix &&
      current[current.length - 1 - suffix] === nextContent[nextContent.length - 1 - suffix]
    ) {
      suffix += 1;
    }

    const removed = current.slice(prefix, current.length - suffix);
    const added = nextContent.slice(prefix, nextContent.length - suffix);

    if (removed.length > 0) {
      for (let i = 0; i < removed.length; i += 1) {
        deleteChar(prefix);
      }
    }

    if (added.length > 0) {
      for (let i = 0; i < added.length; i += 1) {
        insertChar(prefix + i, added[i]);
      }
    }

    contentRef.current = nextContent;
    setContent(nextContent);

    if (selectionStart != null) {
      // Keep the browser caret where the user expects after React catches up.
      queueMicrotask(() => {
        const el = document.querySelector('textarea');
        if (el instanceof HTMLTextAreaElement) {
          el.setSelectionRange(selectionStart, selectionStart);
        }
      });
    }
  }, [deleteChar, insertChar]);

  return { connected, content, cursors, applyLocalText, sendPresence };
}
