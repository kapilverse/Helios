const WebSocket = require('ws');
const crypto = require('crypto');

const ws = new WebSocket('ws://localhost:3000/ws');
const peerId = crypto.randomUUID();
let clock = 0;

ws.on('open', () => {
    console.log('Connected');
    ws.send(JSON.stringify({ Join: { document_id: 'default' } }));
});

ws.on('message', (data) => {
    const msg = JSON.parse(data.toString());
    console.log('Received:', JSON.stringify(msg, null, 2));

    if (msg.Sync) {
        clock++;
        const opId = { peer: peerId, clock: clock };
        const op = { Insert: { id: opId, after: null, content: 'X' } };
        console.log('Sending Op:', JSON.stringify({ Op: { op } }));
        ws.send(JSON.stringify({ Op: { op } }));
    }
});

ws.on('error', (err) => console.error(err));
