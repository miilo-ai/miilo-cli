import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {MiiloClient} from '../dist/client.js';
test('subscribe before bootstrap; events and reconnects read fresh snapshots', async()=>{
 const calls=[];let stream;let revision=1;
 const server=createServer((req,res)=>{
  if(req.url==='/v1/stream'){calls.push('subscribe');stream=res;res.writeHead(200,{'content-type':'text/event-stream'});res.write(':connected\n\n');}
  else if(req.url==='/v1/snapshot'){calls.push('snapshot');res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify({protocolVersion:'0.1',snapshotId:String(revision),cursor:`boot:${revision}`,observedAt:new Date().toISOString(),providers:[],workItems:[],actors:[],attempts:[],artifacts:[],evidence:[],attention:[]}));}
 });
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 const client=new MiiloClient(`http://127.0.0.1:${server.address().port}`);
 let observation=client.observe();
 try {
  const initial=await observation.next();assert.equal(initial.value.reason,'bootstrap');assert.equal(initial.value.snapshot.snapshotId,'1');assert.deepEqual(calls,['subscribe','snapshot']);
  revision=2;stream.write('id: boot:2\nevent: miilo.event\ndata: {"protocolVersion":"0.1","eventId":"event","cursor":"boot:2","providerId":"one","occurredAt":"2026-10-16T00:00:00Z","kind":"updated","subject":{"kind":"provider","id":"one"},"summary":"Changed","payload":{}}\n\n');
  const updated=await observation.next();assert.equal(updated.value.snapshot.snapshotId,'2');assert.equal(updated.value.reason,'event');
  await observation.return();revision=3;observation=client.observe();
  const reconnected=await observation.next();assert.equal(reconnected.value.reason,'bootstrap');assert.equal(reconnected.value.snapshot.snapshotId,'3');assert.deepEqual(calls,['subscribe','snapshot','snapshot','subscribe','snapshot']);
  stream.write('event: miilo.error\ndata: {"reason":"snapshot_required","message":"History lost"}\n\n');
  await assert.rejects(observation.next(),/fresh snapshot/);
 }finally{await observation.return();server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}
});

test('ordinary JSON and error responses are bounded',async()=>{
 let status=200;const server=createServer((req,res)=>{res.writeHead(status,{'content-type':'application/json'});res.end('x'.repeat(2*1024*1024+1));});
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const client=new MiiloClient(`http://127.0.0.1:${server.address().port}`,'do-not-echo');
 try{await assert.rejects(client.snapshot(),/size limit/);status=401;await assert.rejects(client.snapshot(),error=>error.status===401&&!error.message.includes('do-not-echo'));}
 finally{server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}
});
