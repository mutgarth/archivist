#!/usr/bin/env python3
"""Run integration tests in a disposable DB/media directory. Docker Compose db must be running."""
import json,os,pathlib,shlex,socket,struct,subprocess,sys,time,urllib.request,uuid,zlib
root=pathlib.Path(__file__).resolve().parents[1]
os.chdir(root)
env=os.environ.copy()
for line in (root/".env").read_text().splitlines():
 if line.strip() and not line.startswith("#"):
  k,v=shlex.split(line)[0].split("=",1);env[k]=v
name="archivist_test_"+uuid.uuid4().hex[:12]
base=root/".local"/name
media=base/"media/fixtures";media.mkdir(parents=True)
def png(rgb):
 def chunk(t,d):return struct.pack(">I",len(d))+t+d+struct.pack(">I",zlib.crc32(t+d)&0xffffffff)
 return b"\x89PNG\r\n\x1a\n"+chunk(b"IHDR",struct.pack(">IIBBBBB",32,32,8,2,0,0,0))+chunk(b"IDAT",zlib.compress(b"".join(b"\0"+bytes(rgb)*32 for _ in range(32))))+chunk(b"IEND",b"")
for n,c in [("red.png",(230,30,30)),("blue.png",(20,60,230))]:(media/n).write_bytes(png(c))
(media/"link.png").symlink_to(media/"red.png")
subprocess.run([env.get("FFMPEG_BIN","ffmpeg"),"-v","error","-f","lavfi","-i","color=c=blue:s=320x240:r=24","-t","1","-c:v","libx264","-pix_fmt","yuv420p",str(media/"clip.mp4")],check=True)
(media.parent/".archivist-volume").write_text("test-volume")
env["MEDIA_VOLUME_ID"]="test-volume"
with socket.socket() as s:s.bind(("127.0.0.1",0));port=s.getsockname()[1]
env.update(DATABASE_URL=env["DATABASE_URL"].rsplit("/",1)[0]+"/"+name,MEDIA_ROOT=str(media.parent),ARCHIVIST_BIND=f"127.0.0.1:{port}",ARCHIVIST_URL=f"http://127.0.0.1:{port}",TEST_DATABASE=name,LMSTUDIO_URL="http://127.0.0.1:1/v1")
def pg(sql):
 subprocess.run(["docker","compose","-p","archivist","exec","-T","db","psql","-v","ON_ERROR_STOP=1","-U","archivist","-d","postgres","-c",sql],check=True,stdout=subprocess.DEVNULL,env=env)
pg('CREATE DATABASE '+name)
process=None
try:
 with (base/"server.log").open("w") as log:
  process=subprocess.Popen(["target/debug/server"],env=env,stdout=log,stderr=log)
  for _ in range(100):
   try:
    with urllib.request.urlopen(env["ARCHIVIST_URL"]+"/health/ready",timeout=1) as r:
     if r.status==200:break
   except Exception:time.sleep(.1)
  else:raise RuntimeError("Server did not become ready; inspect "+str(base/"server.log"))
  subprocess.run([sys.executable,"tests/api.py"],env=env,check=True)
finally:
 if process:
  process.terminate()
  try:process.wait(timeout=5)
  except subprocess.TimeoutExpired:process.kill();process.wait()
 pg('DROP DATABASE '+name)
print("PASS: disposable integration database removed; diagnostic log retained in .local.")
