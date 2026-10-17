import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,readFileSync,rmSync,existsSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {resolve,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {run} from '../dist/main.js';
const native=resolve(fileURLToPath(new URL('../../../',import.meta.url)),'target/debug/miilo-local'+(process.platform==='win32'?'.exe':''));
test('archive uses native codecs without consulting remote credentials',async()=>{
 const previous={executable:process.env.MIILO_LOCAL_EXECUTABLE,url:process.env.MIILO_URL,token:process.env.MIILO_TOKEN_FILE};
 const folder=mkdtempSync(join(tmpdir(),'miilo-archive-'));
 try {
  assert(existsSync(native),'Build native CLI before the compiled Node/native integration test');
  process.env.MIILO_LOCAL_EXECUTABLE=native;process.env.MIILO_URL='not-a-url';process.env.MIILO_TOKEN_FILE='/missing';
  const input=join(folder,'input');const data=Buffer.from('Synthetic café evidence\n'.repeat(200));writeFileSync(input,data);
  for(const codec of ['gzip','brotli','zstd']){
   const packed=join(folder,codec),restored=packed+'.restored';
   await run(['archive','export',input,packed,'--codec',codec]);
   await run(['archive','restore',packed,restored]);
   assert.deepEqual(readFileSync(restored),data);
   await assert.rejects(run(['archive','restore',packed,restored]),/Local observation stopped/);
  }
 }finally{
  rmSync(folder,{recursive:true,force:true});
  for(const [key,value]of [['MIILO_LOCAL_EXECUTABLE',previous.executable],['MIILO_URL',previous.url],['MIILO_TOKEN_FILE',previous.token]]){
   if(value===undefined)delete process.env[key];else process.env[key]=value;
  }
 }
});
