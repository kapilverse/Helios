import { useState, useEffect, useRef, useCallback } from 'react';
import { HeliosClient, ServerMessage, Op, OpId } from '../lib/ws';

interface Cursor {
  name: string;
  color: string;
  opId: OpId | null;
}

interface CharEntry {
  id: OpId;
  after: OpId | null;
  ch: string;
}

export function useHelios(url: string) {
  const clientRef = useRef<HeliosClient | null>(null);
  const [connected, setConnected] = useState(false);
  const [content, setContent] = useState('');
  const [cursors, setCursors] = useState<Cursor[]>([]);
  const entriesRef = useRef<CharEntry[]>([]);

  useEffect(() => {
    const client = new HeliosClient(url);
    clientRef.current = client;

    const unsubscribe = client.onMessage((msg: ServerMessage) => {
      if (msg.Sync) {
        setConnected(true);
        const entries: CharEntry[] = [];
        for (const [, op] of msg.Sync.response.ops) {
          const entry = applyOpToEntries(entries, op);
          if (entry) entries.push(entry);
        }
        entriesRef.current = entries;
        setContent(entries.map((e) => e.ch).join(''));
      }

      if (msg.Op) {
        const entries = [...entriesRef.current];
        const entry = applyOpToEntries(entries, msg.Op.op);
        if (entry) entries.push(entry);
        entriesRef.current = entries;
        setContent(entries.map((e) => e.ch).join(''));
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
    const entries = entriesRef.current;
    const afterId = pos > 0 ? entries[pos - 1]?.id ?? null : null;

    const newEntry: CharEntry = {
      id: clientRef.current!.nextOpId(),
      after: afterId,
      ch,
    };

    // Insert at correct position
    entries.splice(pos, 0, newEntry);
    entriesRef.current = entries;
    setContent(entries.map((e) => e.ch).join(''));

    // Send to server
    clientRef.current?.sendInsert(newEntry.id, newEntry.after, ch);
  }, []);

  const deleteChar = useCallback((pos: number) => {
    const entries = entriesRef.current;
    if (pos < 0 || pos >= entries.length) return;

    const target = entries[pos];
    entries.splice(pos, 1);
    entriesRef.current = entries;
    setContent(entries.map((e) => e.ch).join(''));

    clientRef.current?.sendDelete(target.id);
  }, []);

  const sendPresence = useCallback((cursor: OpId | null) => {
    clientRef.current?.sendPresence(cursor);
  }, []);

  return { connected, content, cursors, insertChar, deleteChar, sendPresence };
}

function applyOpToEntries(entries: CharEntry[], op: Op): CharEntry | null {
  if ('Insert' in op) {
    const { id, after, content: ch } = op.Insert;
    return { id, after, ch };
  }
  if ('Delete' in op) {
    const idx = entries.findIndex((e) => e.id.peer === op.Delete.target.peer && e.id.clock === op.Delete.target.clock);
    if (idx !== -1) entries.splice(idx, 1);
  }
  return null;
}
