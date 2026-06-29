export interface OpId {
  peer: string;
  clock: number;
}

export interface InsertOp {
  Insert: {
    id: OpId;
    after: OpId | null;
    content: string;
  };
}

export interface DeleteOp {
  Delete: { target: OpId };
}

export type Op = InsertOp | DeleteOp;

export interface ServerMessage {
  Sync?: {
    response: {
      ops: [number, Op][];
      current_seq: number;
    };
  };
  Op?: { op: Op; seq: number };
  Presence?: {
    peers: {
      op_id: OpId | null;
      name: string;
      color: string;
    }[];
  };
  Error?: { message: string };
}

export interface ClientMessage {
  Join?: { document_id: string };
  Op?: { op: Op };
  Presence?: {
    cursor: OpId | null;
    selection_start: OpId | null;
    selection_end: OpId | null;
    viewport_top: OpId | null;
    viewport_bottom: OpId | null;
  };
}

export type MessageHandler = (msg: ServerMessage) => void;

export class HeliosClient {
  private ws: WebSocket | null = null;
  private handlers: MessageHandler[] = [];
  private url: string;
  private peerId: string;
  private clock: number = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(url: string) {
    this.url = url;
    this.peerId = crypto.randomUUID();
  }

  connect() {
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log('[Helios] Connected');
      this.send({ Join: { document_id: 'default' } });
    };

    this.ws.onmessage = (event) => {
      const msg: ServerMessage = JSON.parse(event.data);
      this.handlers.forEach((h) => h(msg));
    };

    this.ws.onclose = () => {
      console.log('[Helios] Disconnected, reconnecting...');
      this.reconnectTimer = setTimeout(() => this.connect(), 2000);
    };

    this.ws.onerror = (err) => {
      console.error('[Helios] Error:', err);
    };
  }

  disconnect() {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.ws?.close();
  }

  onMessage(handler: MessageHandler) {
    this.handlers.push(handler);
    return () => {
      this.handlers = this.handlers.filter((h) => h !== handler);
    };
  }

  send(msg: ClientMessage) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }

  insertChar(after: OpId | null, content: string): OpId {
    this.clock += 1;
    const id: OpId = { peer: this.peerId, clock: this.clock };
    const op: Op = { Insert: { id, after, content } };
    this.send({ Op: { op } });
    return id;
  }

  deleteChar(target: OpId): void {
    const op: Op = { Delete: { target } };
    this.send({ Op: { op } });
  }

  sendPresence(cursor: OpId | null) {
    this.send({
      Presence: {
        cursor,
        selection_start: null,
        selection_end: null,
        viewport_top: null,
        viewport_bottom: null,
      },
    });
  }

  get peerIdStr() {
    return this.peerId;
  }
}
