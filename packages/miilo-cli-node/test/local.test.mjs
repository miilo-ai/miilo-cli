import {test} from 'node:test';
import assert from 'node:assert/strict';
import {run} from '../dist/main.js';
test('local dispatch bypasses remote credentials and fails honestly when engine is absent',async()=>{
 const previous={executable:process.env.MIILO_LOCAL_EXECUTABLE,tokenFile:process.env.MIILO_TOKEN_FILE,url:process.env.MIILO_URL};
 try{
  process.env.MIILO_LOCAL_EXECUTABLE=process.execPath;process.env.MIILO_TOKEN_FILE='/not-present/credential';process.env.MIILO_URL='not-a-url';
  await run(['local','-e','process.exit(0)']);
  await run(['--json','local','-e','process.exit(0)']);
  process.env.MIILO_LOCAL_EXECUTABLE='/not-present/miilo-local';
  await assert.rejects(run(['local','snapshot']),/Install the native Miilo CLI/);
 }finally{for(const [key,value]of [['MIILO_LOCAL_EXECUTABLE',previous.executable],['MIILO_TOKEN_FILE',previous.tokenFile],['MIILO_URL',previous.url]]){if(value===undefined)delete process.env[key];else process.env[key]=value;}}
});
