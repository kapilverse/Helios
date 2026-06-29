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

  useEffect(() => {
    const client = new HeliosClient(url);
    clientRef.current = client;

    const unsubscribe = client.onMessage((msg: ServerMessage) => {
      if (msg.Sync) {
        setConnected(true);
        let text = '';
        for (const [, op] of msg.Sync.response.ops) {
          text = applyOp(text, op);
        }
        contentRef.current = text;
        setContent(text);
      }

      if (msg.Op) {
        setContent((prev) => {
          const next = applyOp(prev, msg.Op!.op);
          contentRef.current = next;
          return next;
        });
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
    // Optimistic local update
    setContent((prev) => {
      const next = prev.slice(0, pos) + ch + prev.slice(pos);
      contentRef.current = next;
      return next;
    });
    // Send to server (simplified — real CRDT would use OpId-based positioning)
    clientRef.current?.insertChar(null, ch);
  }, []);

  const deleteChar = useCallback((pos: number) => {
    setContent((prev) => {
      const next = prev.slice(0, pos) + prev.slice(pos + 1);
      contentRef.current = next;
      return next;
    });
    // simplified delete
    if (contentRef.current.length > 0) {
      clientRef.current?.deleteChar({ peer: '0', clock: 0 });
    }
  }, []);

  const sendPresence = useCallback((cursor: OpId | null) => {
    clientRef.current?.sendPresence(cursor);
  }, []);

  return { connected, content, cursors, insertChar, deleteChar, sendPresence };
}

function applyOp(text: string, op: Op): string {
  if ('Insert' in op) {
    const { content: ch } = op.Insert;
    // Simple append — real CRDT would position by OpId
    return text + ch;
  }
  if ('Delete' in op) {
    // Simple last-char delete
    return text.length > 0 ? text.slice(0, -1) : text;
  }
  return text;
}
