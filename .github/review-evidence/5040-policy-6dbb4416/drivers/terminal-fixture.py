import os,pathlib,tempfile,time,socket,http.client,hashlib,shutil,uuid,json,subprocess,re

trace_root=pathlib.Path("/__epic_private_fixture_not_started__")
trace_proc=None
trace_proc_log=None
trace_ids=[]
trace_env={'PATH':str(trace_root/'bin')+':/usr/bin:/bin','LANG':'C.UTF-8','TERM':'xterm-256color','HOME':str(trace_root/'home'),'XDG_CONFIG_HOME':str(trace_root/'config'),'XDG_DATA_HOME':str(trace_root/'data'),'XDG_STATE_HOME':str(trace_root/'state'),'XDG_RUNTIME_DIR':str(trace_root/'runtime'),'TMPDIR':str(trace_root/'tmp'),'SHELL':str(trace_root/'bin/private-shell'),'AOE_TMUX_SOCKET':str(trace_root/'tmux.sock'),'AOE_NO_AUTOSTART':'1'}
trace_fake="#!/usr/bin/python3\nimport os,sys,json,pathlib,subprocess,uuid\nROOT=pathlib.Path(ROOT_LITERAL)\nif any(x in sys.argv[1:] for x in ['--help','--version']):\n print('private fixture');sys.exit(0)\nident=os.environ.get('AOE_INSTANCE_ID','unknown')\nmodepath=ROOT/'modes'/(ident+'.json')\nmode=json.loads(modepath.read_text()) if modepath.exists() else {}\ndef event(action,**kw):\n with (ROOT/'events.jsonl').open('a') as out:out.write(json.dumps(dict(action=action,instance=ident,pid=os.getpid(),argv=sys.argv[1:],**kw))+'\\n')\ndef arg(flag):\n return sys.argv[sys.argv.index(flag)+1] if flag in sys.argv and sys.argv.index(flag)+1<len(sys.argv) else None\nsid=arg('--resume') or arg('--session-id') or mode.get('publish_sid') or str(uuid.uuid4())\nstore=pathlib.Path(os.environ.get('CLAUDE_CONFIG_DIR',str(ROOT/'home/.claude')))\ncwd=os.getcwd();encoded=''.join(c if c.isascii() and (c.isalnum() or c=='-') else '-' for c in cwd)\ntranscript=store/'projects'/encoded/(sid+'.jsonl')\nloaded=transcript.exists()\nif not loaded:\n transcript.parent.mkdir(parents=True,exist_ok=True)\n transcript.write_text(json.dumps({'type':'user','message':{'role':'user','content':'PRIVATE ORIGINAL '+sid}})+'\\n')\ncontents=transcript.read_text()\nif mode.get('publish'):\n helper=os.environ.get('AOE_HOOK_BIN',str(ROOT/'binary-does-not-exist'))\n result=subprocess.run([helper,'__extract-session-id','--field','session-id','--agent','claude'],input=json.dumps({'session_id':sid}),text=True,capture_output=True)\n event('publication',sid=sid,returncode=result.returncode,helper=helper)\nevent('loaded' if loaded else 'created',sid=sid,transcript=str(transcript),content=contents)\nif mode.get('exit_after_load'):\n event('exit_after_load',sid=sid);os._exit(17)\nprint('PRIVATE FIXTURE READY',flush=True)\nfor line in sys.stdin:\n event('input',sid=sid,line=line.rstrip('\\n'))\n"

def trace_config(auto=True,declared=False):
 text='[systemd]\nenabled=false\n[session]\nsmart_rename=false\nauto_resume_on_restart='+str(auto).lower()+'\n[session.custom_agents]\nwork-claude="work-claude"\n[session.agent_detect_as]\nwork-claude="claude"\n'
 if declared:text+='[session.agent_execution_as]\nwork-claude="claude"\n[session.agent_config_dir]\nwork-claude='+json.dumps(str(trace_root/'home/.claude'))+'\n'
 write(str(trace_app/'config.toml'),text)

class TraceUnixHTTP(http.client.HTTPConnection):
 def connect(self):
  self.sock=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);self.sock.settimeout(30);self.sock.connect(str(trace_sock))

def trace_api(method,path,body=None):
 c=TraceUnixHTTP('localhost')
 try:
  c.request(method,path,body=None if body is None else json.dumps(body),headers={'Content-Type':'application/json'} if body is not None else {})
  r=c.getresponse();return r.status,dict(r.getheaders()),r.read().decode()
 finally:c.close()

def trace_cli(*args,check=True):
 p=subprocess.run([str(trace_binary),*args],env=trace_env,cwd=trace_root/'project',capture_output=True,text=True,timeout=45)
 if check:assert p.returncode==0,(args,p.returncode,p.stdout,p.stderr)
 return p

def trace_events(ident=None):
 f=trace_root/'events.jsonl'
 ev=[json.loads(l) for l in f.read_text().splitlines()] if f.exists() else []
 return [e for e in ev if ident is None or e['instance']==ident]

def trace_until(predicate,label,seconds=20):
 end=time.monotonic()+seconds
 while True:
  try:answer=predicate()
  except (OSError,http.client.HTTPException):answer=False
  if answer:return answer
  if time.monotonic()>end:raise TimeoutError(label)
  time.sleep(0.05)

def trace_listed(ident):
 status,headers,body=trace_api('GET','/api/sessions')
 if status!=200:return None
 parsed=json.loads(body);rows=parsed['sessions'] if isinstance(parsed,dict) else parsed
 return next((r for r in rows if r['id']==ident),None)

def trace_row(ident):
 return next(r for r in json.loads((trace_app/'profiles/main/sessions.json').read_text()) if r['id']==ident)

def trace_start_daemon():
 global trace_proc,trace_proc_log
 trace_proc_log=open(trace_root/'daemon-output.log','ab',buffering=0)
 trace_proc=subprocess.Popen([str(trace_binary),'serve','--core-only'],env=trace_env,cwd=trace_root/'project',stdout=trace_proc_log,stderr=subprocess.STDOUT)
 trace_until(lambda:trace_api('GET','/api/sessions')[0]==200,'private daemon ready')

def trace_stop_daemon():
 global trace_proc
 if trace_proc and trace_proc.poll() is None:trace_proc.terminate();trace_proc.wait(timeout=30)
 if trace_proc_log:trace_proc_log.close()
 trace_proc=None

def trace_add(title,tool):
 trace_cli('add',str(trace_root/'project'),'--title',title,'--tool',tool)
 ident=next(r['id'] for r in json.loads((trace_app/'profiles/main/sessions.json').read_text()) if r['title']==title)
 trace_ids.append(ident);trace_until(lambda:trace_listed(ident),'CLI creation observed by daemon');return ident

def prepare_epic_private_fixture(prefix,binary):
 global trace_root,trace_env,trace_app,trace_sock,trace_binary,trace_proc,trace_ids
 assert trace_proc is None and not trace_ids
 previous=str(trace_root);trace_root=pathlib.Path(tempfile.mkdtemp(prefix=prefix))
 trace_env={k:v.replace(previous,str(trace_root)) for k,v in trace_env.items()}
 trace_app=trace_root/'config/agent-of-empires-dev';trace_sock=trace_app/'daemon/api.sock';trace_binary=pathlib.Path(binary);trace_ids=[]
 for part in ('home','config','data','state','runtime','tmp','bin','project','modes'):(trace_root/part).mkdir(parents=True,exist_ok=True)
 os.chmod(trace_root/'runtime',0o700)
 script=re.sub(r'^ROOT=pathlib.Path\(.*\)$','ROOT=pathlib.Path('+repr(str(trace_root))+')',trace_fake,flags=re.MULTILINE)
 script=script.replace("print('PRIVATE FIXTURE READY',flush=True)","(ROOT/'native-ready').write_text(str(os.getpid()))\nprint('PRIVATE FIXTURE READY',flush=True)")
 write(str(trace_root/'bin/claude'),script);os.chmod(trace_root/'bin/claude',0o755)
 write(str(trace_root/'bin/work-claude'),'#!/bin/sh\nexec "'+str(trace_root/'bin/claude')+'" "$@"\n');os.chmod(trace_root/'bin/work-claude',0o755)
 write(str(trace_root/'bin/private-shell'),'#!/bin/sh\nexec /bin/bash --noprofile --norc "$@"\n');os.chmod(trace_root/'bin/private-shell',0o755)
 trace_config(auto=True,declared=True)
 return trace_root
