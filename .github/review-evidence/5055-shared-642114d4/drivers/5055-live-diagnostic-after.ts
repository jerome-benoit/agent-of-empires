import assert from 'node:assert/strict';
import { existsSync, readFileSync, writeFileSync, mkdirSync, chmodSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { createInterface } from 'node:readline';
import { spawnAoeServe, appDirFor, fakeAcpScriptPath } from '/home/fraggle/src/agent-of-empires-git-worktrees/5055-shared-recovery-diagnostics/web/tests/helpers/aoeServe.ts';
import { initWorkingRepo } from '/home/fraggle/src/agent-of-empires-git-worktrees/5055-shared-recovery-diagnostics/web/tests/helpers/gitFixture.ts';
import { replayFrames, waitForAcpReady } from '/home/fraggle/src/agent-of-empires-git-worktrees/5055-shared-recovery-diagnostics/web/tests/helpers/acp.ts';

const [binary, evidence] = process.argv.slice(2);
assert(binary?.startsWith('/') && existsSync(binary));
assert(evidence?.startsWith('/') && existsSync(evidence));
process.env.AOE_E2E_BINARY = binary;
process.env.AOE_E2E_DEBUG = '1';
delete process.env.AOE_TEST_ACP_RUNNER_EXE;
const controls = createInterface({input:process.stdin,crlfDelay:Infinity});
const lines: string[] = [];
let waiter: ((line:string)=>void)|undefined;
controls.on('line', line => { if (waiter) { const consume=waiter; waiter=undefined; consume(line.trim()); } else lines.push(line.trim()); });
const control = () => lines.length ? Promise.resolve(lines.shift()!) : new Promise<string>(resolve => { waiter=resolve; });
const until = async <T>(observe:()=>Promise<T>|T,label:string,milliseconds=30_000):Promise<T> => {
  const deadline=Date.now()+milliseconds;
  for (;;) { const value=await observe(); if(value)return value; if(Date.now()>=deadline)throw new Error('deadline: '+label); await new Promise(resolve=>setTimeout(resolve,25)); }
};
const birth=(pid:number)=>{
  const fields=readFileSync('/proc/'+pid+'/stat','utf8').split(') ').at(-1)!.trim().split(/\s+/);
  assert(fields[0]!=='Z' && fields[19]); return fields[19];
};
let serve:Awaited<ReturnType<typeof spawnAoeServe>>|undefined;
let project=''; let requests=''; let id=''; let sid=''; let registry='';
let originalOwner:any;
let originalRegistryMode:number|undefined;
let originalRegistryBytes:Buffer|undefined;
const proof:any={scope:'Actual production AoE CLI/API/TUI/browser with a controlled real managed ACP Native, not vendor compatibility',cases:[],binary};
const observed:{pid:number,birth:string}[]=[];
let successful=false;
try {
  serve=await spawnAoeServe({workerIndex:0,parallelIndex:59,acp:true,
    extraEnv:home=>({FAKE_ACP_DEBUG:join(home,'5055-native-requests.log')}),
    seedFn:({home,xdg,env})=>{
      project=join(home,'project'); requests=join(home,'5055-native-requests.log'); initWorkingRepo(project,env);
      const app=appDirFor(home,xdg,binary); mkdirSync(app,{recursive:true});
      writeFileSync(join(app,'state.toml'),'has_seen_welcome=true\nhas_responded_to_telemetry=true\nhas_acknowledged_agent_hooks=true\nlast_seen_version="1.21.0"\n');
      writeFileSync(join(app,'config.toml'),readFileSync(join(app,'config.toml'),'utf8')+'[updates]\nupdate_check_mode="off"\n[session]\nsmart_rename=false\n');
    },
    fakeAcpScript:{turns:[
      {updates:[{sessionUpdate:'agent_message_chunk',content:{type:'text',text:'5055_NATIVE_PROVIDER_ATTEMPT'}}],error:{code:-32603,message:'SYNTHETIC_PROVIDER_FAILURE: provider temporarily unavailable',data:{api_key:'5055_NEVER_PUBLIC_SECRET'}}},
      {updates:[{sessionUpdate:'agent_message_chunk',content:{type:'text',text:'5055_NATIVE_PROVIDER_REPAIRED_SAME_ACTOR'}}],stopReason:'end_turn'},
      {updates:[{sessionUpdate:'agent_message_chunk',content:{type:'text',text:'5055_NATIVE_AUTH_INTERVENTION'}}],error:{code:-32603,message:'Sign in required sk-ant-abcdefghijklmnop1234567890',data:{details:'Repair authentication in the Native Agent',api_key:'5055_NEVER_PUBLIC_SECRET'}}},
      {updates:[{sessionUpdate:'agent_message_chunk',content:{type:'text',text:'5055_NATIVE_AUTH_REPAIRED_SAME_ACTOR'}}],stopReason:'end_turn'},
      {updates:[{sessionUpdate:'agent_message_chunk',content:{type:'text',text:'5055_NATIVE_AGENT_OWNED_RETRY: Retrying (2/3) in 30s…'}},{sessionUpdate:'wait_for_release'},{sessionUpdate:'agent_message_chunk',content:{type:'text',text:'5055_NATIVE_RETRY_COMPLETED_WITH_FAILURE'}}],error:{code:-32603,message:'SYNTHETIC_PROVIDER_FAILURE_AFTER_AGENT_RETRY',data:{details:'The Native Agent completed its own retry; operator review is required'}}},
    ]},
  });
  const base=serve.baseUrl;
  const env={...serve.env,AOE_DAEMON_URL:base,AOE_NO_AUTOSTART:'1'};
  const cli=(...args:string[])=>{ const r=spawnSync(binary,args,{cwd:project,env,encoding:'utf8',timeout:45_000}); return {argv:[binary,...args],exit:r.status,stdout:r.stdout,stderr:r.stderr,error:r.error?.message}; };
  const rows=async()=>{ const r=await fetch(base+'/api/sessions'); assert.equal(r.status,200); return (await r.json()).sessions as any[]; };
  const row=async()=> (await rows()).find(r=>r.id===id);
  const frames=async()=>await replayFrames(base,id) as any[];
  const count=()=>{const text=readFileSync(requests,'utf8');return {initialize:[...text.matchAll(/req method=initialize /g)].length,new:[...text.matchAll(/req method=session\/new /g)].length,load:[...text.matchAll(/req method=session\/load /g)].length,prompt:[...text.matchAll(/req method=session\/prompt /g)].length};};
  const owner=()=>{const r=JSON.parse(readFileSync(registry,'utf8'));return {pid:r.pid,birth:birth(r.pid),generation:r.generation,identity:r.runner_identity,nativePid:originalOwner.nativePid,nativeBirth:birth(originalOwner.nativePid)};};
  const preserved=(prompts:number,recordReadable=true)=>{if(recordReadable)assert.deepEqual(owner(),originalOwner);else{assert.equal(birth(originalOwner.pid),originalOwner.birth);assert.equal(birth(originalOwner.nativePid),originalOwner.nativeBirth);assert.equal(statSync(registry).mode&0o777,0);}const c=count();assert.equal(c.initialize,1);assert.equal(c.new,1);assert.equal(c.load,0);assert.equal(c.prompt,prompts);return c;};
  const postPrompt=async(text:string)=>{const r=await fetch(base+'/api/sessions/'+id+'/acp/prompt',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({text,no_revive:true})});const body=await r.text();assert(r.ok,body);const dispatch=JSON.parse(body);assert.equal(dispatch.disposition,'sent','new explicit input must reach the existing Native, not a queue');return {status:r.status,text:body,dispatch};};
  const created=await fetch(base+'/api/sessions',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({path:project,tool:'claude',title:'5055 shared recovery actual',view:'structured'})});
  assert.equal(created.status,201,await created.text());
  id=(await until(async()=>(await rows()).find(r=>r.title==='5055 shared recovery actual'),'real row')).id;
  const assigned=await until(async()=>(await frames()).find(f=>f.event?.AcpSessionAssigned?.connection_ready===true),'real Native New result');
  sid=assigned.event.AcpSessionAssigned.acp_session_id;
  await waitForAcpReady(base,id,30_000,serve.home);
  const app=appDirFor(serve.home,serve.env.XDG_CONFIG_HOME!,binary); registry=join(app,'acp-workers',id+'.json');
  const nr=[...readFileSync(join(serve.home,'fake-acp.log'),'utf8').matchAll(/fake-acp starting pid=(\d+)/g)]; assert.equal(nr.length,1);
  const record=JSON.parse(readFileSync(registry,'utf8')); const nativePid=Number(nr[0][1]);
  originalOwner={pid:record.pid,birth:birth(record.pid),generation:record.generation,identity:record.runner_identity,nativePid,nativeBirth:birth(nativePid)};
  observed.push({pid:record.pid,birth:originalOwner.birth},{pid:nativePid,birth:originalOwner.nativeBirth});
  proof.id=id;proof.sid=sid;proof.originalOwner=originalOwner;proof.baseUrl=base;
  const safeKeys=['PATH','HOME','XDG_CONFIG_HOME','XDG_DATA_HOME','XDG_STATE_HOME','XDG_RUNTIME_DIR','TMPDIR','TMUX_TMPDIR','AOE_TMUX_SOCKET','SHELL','LANG','LC_ALL','TERM'];
  const observerEnv=Object.fromEntries(safeKeys.filter(k=>env[k]!==undefined).map(k=>[k,env[k]]));
  Object.assign(observerEnv,{AOE_DAEMON_URL:base,AOE_NO_AUTOSTART:'1',TERM:'xterm-256color',LANG:'C.UTF-8'});
  writeFileSync(join(evidence,'observer-env-private.json'),JSON.stringify(observerEnv,null,2));
  const snapshot=async(name:string,prompts:number,validate:(r:any)=>void,recordReadable=true)=>{
    const r=await row(); assert.equal(r.acp_session_id,sid); validate(r);
    const show=cli('session','show',id);const showJson=cli('session','show',id,'--json');assert.equal(show.exit,0,JSON.stringify(show));assert.equal(showJson.exit,0,JSON.stringify(showJson));
    writeFileSync(join(evidence,name+'-cli-observation.json'),JSON.stringify({show,showJson},null,2));
    const parsed=JSON.parse(showJson.stdout); const publicFrames=await frames();
    assert(!JSON.stringify({r,show,parsed,publicFrames}).includes('5055_NEVER_PUBLIC_SECRET'));
    assert(!JSON.stringify({r,show,parsed,publicFrames}).includes('sk-ant-abcdefghijklmnop1234567890'));
    if(r.recovery){assert.equal(parsed.recovery.diagnostic,r.recovery.diagnostic);assert.equal(parsed.recovery.reason,r.recovery.reason);assert.deepEqual(parsed.recovery.actions,r.recovery.actions);}
    const result={name,api:r,show,showJson,counts:preserved(prompts,recordReadable),owner_record_currently_readable:recordReadable,public_frames:publicFrames};proof.cases.push(result);
    writeFileSync(join(evidence,name+'-ready.json'),JSON.stringify({...result,baseUrl:base,id,sid,home:serve!.home,project,binary},null,2));
    console.log('DIAGNOSTICS_5055_AFTER_STAGE_READY',JSON.stringify({name,baseUrl:base,id,sid,evidence}));
    assert.equal(await control(),'verify-'+name);
    const again=await row();assert.equal(again.acp_session_id,sid);validate(again);preserved(prompts,recordReadable);
    if(r.recovery){assert.equal(again.recovery.diagnostic,r.recovery.diagnostic);assert.equal(again.recovery.evidence.observed_at_ms,r.recovery.evidence.observed_at_ms);}
    result.opening_preserves_identity_failure_and_prompt_count=true;
    writeFileSync(join(evidence,name+'-proof.json'),JSON.stringify(result,null,2));
  };
  const failed=(r:any)=>{assert.equal(r.recovery.reason,'operation_failed');assert.equal(r.recovery.evidence.blocking_scope,'operation');assert.equal(r.recovery.runtime,'acp_transport_ready');assert.equal(r.recovery.actions.attach,true);assert.equal(r.recovery.actions.retry_operation,false);assert.equal(r.recovery.work_in_progress,false);};
  proof.providerInput=await postPrompt('5055_EXPLICIT_PROVIDER_FAILURE_INPUT');
  await until(async()=>{const r=await row();return r.recovery?.diagnostic.includes('SYNTHETIC_PROVIDER_FAILURE')&&r;},'shared provider failure');
  await snapshot('provider-failure',1,failed);
  proof.providerRepair=await postPrompt('5055_USER_NEW_COMPOSITION_AFTER_MANUAL_PROVIDER_REPAIR');
  await until(async()=>{const f=await frames();const r=await row();return f.some(x=>JSON.stringify(x.event).includes('5055_NATIVE_PROVIDER_REPAIRED_SAME_ACTOR'))&&!r.recovery&&r;},'current Native success supersedes failure');preserved(2);
  proof.provider_recovery={same_actor:true,new_user_composition:true,automatic_retry:false,prior_prompt_not_replayed:true};
  proof.authInput=await postPrompt('5055_EXPLICIT_AUTH_FAILURE_INPUT');
  await until(async()=>{const r=await row();return r.recovery?.diagnostic.includes('Sign in required')&&r;},'Native authentication diagnostic');
  await snapshot('auth-intervention',3,r=>{failed(r);assert(r.recovery.guidance.includes('authentication'));});
  proof.authRepair=await postPrompt('5055_USER_NEW_COMPOSITION_AFTER_MANUAL_NATIVE_AUTH_INTERVENTION');
  await until(async()=>{const f=await frames();const r=await row();return f.some(x=>JSON.stringify(x.event).includes('5055_NATIVE_AUTH_REPAIRED_SAME_ACTOR'))&&!r.recovery&&r.status!=='Running'&&r;},'current Native auth success');preserved(4);
  proof.auth_recovery={scope:'Controlled Native reports success after explicit user intervention, not a real vendor credential test',same_actor:true,no_AoE_auth_or_provider_policy:true};
  const runningPrompt=postPrompt('5055_EXPLICIT_AGENT_OWNED_RETRY_INPUT');
  await until(async()=>{const r=await row();const f=await frames();return r.status==='Running'&&f.some(x=>JSON.stringify(x.event).includes('5055_NATIVE_AGENT_OWNED_RETRY'))&&r;},'Native-owned retry is ongoing work');
  const workRefusal=cli('session','restart',id,'--same');assert.notEqual(workRefusal.exit,0);preserved(5);proof.active_work_refusal=workRefusal;
  await snapshot('agent-owned-retry',5,r=>{assert.equal(r.status,'Running');if(r.recovery){assert.equal(r.recovery.work_in_progress,true);assert.equal(r.recovery.actions.retry_operation,false);assert.equal(r.recovery.actions.restart_same,false);assert.equal(r.recovery.actions.fresh,false);}});
  writeFileSync(fakeAcpScriptPath(serve.home)+'.release','operator-observed-agent-work\n');
  proof.agentRetryResult=await runningPrompt;
  await until(async()=>{const r=await row();return r.recovery?.diagnostic.includes('SYNTHETIC_PROVIDER_FAILURE_AFTER_AGENT_RETRY')&&r;},'Native retry ends with scoped failure');preserved(5);
  originalRegistryMode=statSync(registry).mode&0o777;originalRegistryBytes=readFileSync(registry);
  chmodSync(registry,0);
  assert.throws(()=>readFileSync(registry),(error:any)=>error.code==='EACCES');
  proof.unknown_liveness_intervention={genuine_owned_record_temporarily_unreadable:true,record_bytes_never_rewritten_by_fixture:true,Native_and_runner_births_checked_independently:true,no_saved_record_is_used_as_current_authority:true};
  await until(async()=>{const r=await row();return r.recovery?.runtime==='unknown'&&!r.recovery.actions.stop&&!r.recovery.actions.restart_same&&!r.recovery.actions.fresh&&r;},'loss of current owner observation grants no destructive shortcut');
  await snapshot('unknown-liveness',5,r=>{assert.equal(r.recovery.runtime,'unknown');assert.equal(r.recovery.actions.attach,true);for(const key of ['retry_operation','restart_same','stop','fresh'])assert.equal(r.recovery.actions[key],false);},false);
  chmodSync(registry,originalRegistryMode);originalRegistryMode=undefined;
  assert.deepEqual(readFileSync(registry),originalRegistryBytes);preserved(5);
  proof.unknown_liveness_intervention.original_bytes_and_permissions_restored=true;
  successful=true;console.log('DIAGNOSTICS_5055_AFTER_ACP_CASES_GREEN');
} catch(error){
  proof.failure=String(error);if(serve&&id){proof.failure_frames=await replayFrames(serve.baseUrl,id).catch(String);proof.failure_sessions=await fetch(serve.baseUrl+'/api/sessions').then(r=>r.json()).catch(String);}
  writeFileSync(join(evidence,'after-failure.json'),JSON.stringify(proof,null,2));throw error;
} finally {
  controls.close();let cleanupError:unknown;
  try{if(originalRegistryMode!==undefined){chmodSync(registry,originalRegistryMode);originalRegistryMode=undefined;}}catch(error){cleanupError=error;}
  try{await serve?.stop();}catch(error){cleanupError??=error;}
  for(const actor of observed){if(existsSync('/proc/'+actor.pid)){try{assert.equal(birth(actor.pid),actor.birth);process.kill(actor.pid,'SIGTERM');}catch(error){cleanupError??=error;}}}
  await until(()=>observed.every(actor=>!existsSync('/proc/'+actor.pid)),'owned Native and runner reaped',15_000);
  proof.cleanup={owned_processes_reaped:true,private_server_stopped:cleanupError===undefined,helper_error:cleanupError===undefined?undefined:String(cleanupError)};proof.success=successful&&cleanupError===undefined;
  writeFileSync(join(evidence,'after-proof.json'),JSON.stringify(proof,null,2));
  if(cleanupError)throw cleanupError;
  console.log('DIAGNOSTICS_5055_AFTER_OWNED_CLEANUP_GREEN');
}
