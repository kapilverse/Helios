import { useState, useEffect, useRef, useCallback } from 'react';
import { HeliosClient, ServerMessage, Op } from '../lib/ws';

interface Cursor {
  name: string;
  color: string;
  opId: { peer: string; clock: number } | null;
}

export function useHelios(url: string) {
  const clientRef = useRef<HeliosClient | null>(null);
  const [connected, setConnected] = useState(false);
  const [content, setContent] = useState('');
  const [cursors, setCursors] = useState<Cursor[]>([]);

  useEffect(() => {
    const client = new HeliosClient(url);
    clientRef.current = client;

    const unsubscribe = client.onMessage((msg: ServerMessage) => {
      if (msg.Sync) {
        setConnected(true);
        // Apply initial ops
        let text = '';
        for (const [, op] of msg.Sync.response.ops) {
          text = applyOp(text, op);
        }
        if (text) setContent(text);
      }

      if (msg.Op) {
        setContent((prev) => applyOp(prev, msg.Op!.op));
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

  const insertChar = useCallback((after: { peer: string; clock: number } | null, ch: string) => {
    clientRef.current?.insertChar(after, ch);
  }, []);

  const deleteChar = useCallback((target: { peer: string; clock: number }) => {
    clientRef.current?.deleteChar(target);
  }, []);

  const sendPresence = useCallback((cursor: { peer: string; clock: number } | null) => {
    clientRef.current?.sendPresence(cursor);
  }, []);

  return { connected, content, cursors, insertChar, deleteChar, sendPresence };
}

function applyOp(text: string, op: Op): string {
  if ('Insert' in op) {
    const { content } = op.Insert;
    return text + content;
  }
  if ('Delete' in op) {
    return text;
  }
  return text;
}
