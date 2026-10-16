import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, chmodSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { resolveCredential } from '../dist/credentials.js';
import { MiiloClient } from '../dist/client.js';
import { parseArguments } from '../dist/args.js';

test('credential files are explicit, bounded, private and never echoed on failure', () => {
  const dir=mkdtempSync(join(tmpdir(),'miilo-credential-test-'));
  const path=join(dir,'token');
  try {
    writeFileSync(path,'test-only-token\n',{mode:0o600});
    assert.equal(resolveCredential(undefined,path),'test-only-token');
    assert.throws(()=>resolveCredential('other-token',path),/either/);
    writeFileSync(path,'private token content');
    assert.throws(()=>resolveCredential(undefined,path),/one nonempty token/);
    writeFileSync(path,'x'.repeat(4097));
    assert.throws(()=>resolveCredential(undefined,path),/at most 4096/);
    if(process.platform!=='win32') {
      writeFileSync(path,'test-only-token');chmodSync(path,0o644);
      assert.throws(()=>resolveCredential(undefined,path),/private/);
    }
    assert.equal(parseArguments(['--token-file',path,'doctor'],{}).tokenFile,path);
    assert.equal(parseArguments(['doctor'],{MIILO_TOKEN_FILE:path}).tokenFile,path);
  } finally { rmSync(dir,{recursive:true,force:true}); }
});

test('credentials cannot be embedded in endpoint URLs', () => {
  for(const endpoint of ['https://user:secret@example.com','https://example.com?token=secret','https://example.com#secret']) assert.throws(()=>new MiiloClient(endpoint),/must not contain/);
});
