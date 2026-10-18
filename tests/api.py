"""Contract tests against an isolated running Archivist and its dedicated test database.
Run before starting a worker. Uses only the standard library and synthetic fixtures.
"""
import json, os, urllib.request, urllib.error, uuid, concurrent.futures
BASE=os.environ.get("ARCHIVIST_URL","http://127.0.0.1:8098")
WORKER=os.environ["ARCHIVIST_WORKER_TOKEN"]
def call(path,body=None,token=None,headers=None):
    h=dict(headers or {})
    if token is not None:h["Authorization"]="Bearer "+token
    if body is not None:h["Content-Type"]="application/json"
    req=urllib.request.Request(BASE+path,json.dumps(body).encode() if body is not None else None,headers=h)
    try:
        with urllib.request.urlopen(req,timeout=30) as r:return r.status,r.read(),r.headers
    except urllib.error.HTTPError as e:return e.code,e.read(),e.headers
def parsed(path,body=None,**kw):
    status,data,_=call(path,body,**kw)
    return status,json.loads(data) if data else None
assert call("/health/live")[0]==200
assert call("/api/v1/assets")[0]==200
assert call("/internal/v1/jobs/reserve",{},token="invalid")[0]==401
assert call("/internal/v1/jobs/reserve",{},token=None)[0]==401
key=str(uuid.uuid4())
body={"files":["fixtures/red.png"],"copy_confirmed":True}
status,imp=parsed("/api/v1/imports",body,headers={"Idempotency-Key":key})
assert status==202,(status,imp)
assert len(imp["assets"])==1
asset=imp["assets"][0]
assert parsed("/api/v1/imports",body,headers={"Idempotency-Key":key})[1]==imp
assert call("/api/v1/imports",{"files":["fixtures/blue.png"],"copy_confirmed":True},headers={"Idempotency-Key":key})[0]==409
assert call("/api/v1/imports",{"files":["../secret"],"copy_confirmed":True},headers={"Idempotency-Key":str(uuid.uuid4())})[0]==400
assert call("/api/v1/imports",{"files":["fixtures/link.png"],"copy_confirmed":True},headers={"Idempotency-Key":str(uuid.uuid4())})[0]==400
assert call("/api/v1/imports",{"files":["fixtures/red.png"],"copy_confirmed":False},headers={"Idempotency-Key":str(uuid.uuid4())})[0]==400
assert call("/api/v1/assets/"+asset+"/preview")[0]==200
assert call("/api/v1/assets/"+asset+"/original",headers={"Range":"bytes=0-9"})[0]==206
assert call("/api/v1/assets/"+asset+"/original",headers={"Range":"bytes=999999999-"})[0]==416
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
    claims=list(pool.map(lambda _:parsed("/internal/v1/jobs/reserve",{},token=WORKER),range(2)))
assert sorted(c[0] for c in claims)==[200,204],claims
job=next(c[1] for c in claims if c[0]==200)
lease={"lease_token":job["lease_token"],"attempt":job["attempt"]}
assert call("/internal/v1/jobs/"+job["id"]+"/renew",lease,token=WORKER)[0]==200
wrong={**lease,"lease_token":str(uuid.uuid4())}
assert call("/internal/v1/jobs/"+job["id"]+"/renew",wrong,token=WORKER)[0]==409
result={**lease,"generation":job["generation"],"description":"Um quadrado vermelho sobre fundo branco.","embedding":[0.1]*int(os.environ["EMBEDDING_DIMENSION"]),"embedding_model":os.environ["EMBEDDING_VERSION"],"vision_model":os.environ["VISION_VERSION"],"pipeline_version":"image-v1"}
assert call("/internal/v1/jobs/"+job["id"]+"/complete",{**result,"embedding":[0]},token=WORKER)[0]==400
assert call("/internal/v1/jobs/"+job["id"]+"/complete",result,token=WORKER)[0]==200
assert call("/internal/v1/jobs/"+job["id"]+"/complete",result,token=WORKER)[0]==200
assert call("/internal/v1/jobs/"+job["id"]+"/complete",{**result,"description":"outro"},token=WORKER)[0]==409
assert parsed("/api/v1/assets/"+asset)[1]["description"]==result["description"]
assert call("/api/v1/jobs/"+job["id"]+"/retry",{})[0]==409
print("PASS: auth, import, idempotency, traversal, symlink, preview, Range, reservation, lease, vector validation, atomic completion.")

# Recovery is exercised deterministically only in the disposable test database.
if os.environ.get("TEST_DATABASE","").startswith("archivist_test_"):
    import subprocess,pathlib
    def sql(statement):
        subprocess.run(["docker","compose","-p","archivist","exec","-T","db","psql","-v","ON_ERROR_STOP=1","-U","archivist","-d",os.environ["TEST_DATABASE"],"-c",statement],check=True,stdout=subprocess.DEVNULL)
    status,second=parsed("/api/v1/imports",{"files":["fixtures/blue.png"],"copy_confirmed":True},headers={"Idempotency-Key":str(uuid.uuid4())})
    assert status==202
    status,old=parsed("/internal/v1/jobs/reserve",{},token=WORKER);assert status==200
    sql("UPDATE jobs SET lease_expires_at=now()-interval '1 second' WHERE id='"+str(uuid.UUID(old["id"]))+"'")
    status,new=parsed("/internal/v1/jobs/reserve",{},token=WORKER);assert status==200
    assert new["id"]==old["id"] and new["attempt"]==old["attempt"]+1 and new["lease_token"]!=old["lease_token"]
    old_result={**result,"lease_token":old["lease_token"],"attempt":old["attempt"],"generation":old["generation"]}
    assert call("/internal/v1/jobs/"+old["id"]+"/complete",old_result,token=WORKER)[0]==409
    sql("UPDATE jobs SET attempt=3,lease_expires_at=now()-interval '1 second' WHERE id='"+str(uuid.UUID(old["id"]))+"'")
    assert call("/internal/v1/jobs/reserve",{},token=WORKER)[0]==204
    assert parsed("/api/v1/jobs/"+old["id"])[1]["status"]=="failed"
    assert call("/api/v1/jobs/"+old["id"]+"/retry",{})[0]==202
    assert parsed("/api/v1/jobs/"+old["id"])[1]["status"]=="queued"
    assert parsed("/api/v1/search",{"query":"vermelho","mode":"text"})[1]["items"]
    assert call("/api/v1/search",{"query":"azul","mode":"hybrid"})[0]==503
    assert call("/api/v1/assets/"+asset,token=None)[0]==200
    source=pathlib.Path(os.environ["MEDIA_ROOT"])/"fixtures/red.png"
    source.write_bytes((source.parent/"blue.png").read_bytes())
    assert call("/api/v1/imports",body,headers={"Idempotency-Key":str(uuid.uuid4())})[0]==409
    assert call("/api/v1/assets/"+asset+"/original")[0]==200
    print("PASS: expired lease recovery, stale result rejection, retry budget, shared catalog, original stability and text search with AI offline.")

import pathlib
# A renamed non-video must never become an imported original.
bad_video=pathlib.Path(os.environ["MEDIA_ROOT"])/"fixtures/invalid.mp4"
bad_video.write_bytes(b"not a video")
assert call("/api/v1/imports",{"files":["fixtures/invalid.mp4"],"copy_confirmed":True},headers={"Idempotency-Key":str(uuid.uuid4())})[0]==400

# Videos retain byte-identical originals and expose technical metadata.
import hashlib,pathlib
status,video_import=parsed("/api/v1/imports",{"files":["fixtures/clip.mp4"],"copy_confirmed":True},headers={"Idempotency-Key":str(uuid.uuid4())})
assert status==202,(status,video_import)
video=video_import["assets"][0]
metadata=parsed("/api/v1/assets/"+video)[1]
assert metadata["mime"]=="video/mp4"
assert metadata["width"]==320 and metadata["height"]==240
assert float(metadata["media_metadata"]["format"]["duration"])>=1
original=pathlib.Path(os.environ["MEDIA_ROOT"])/"fixtures/clip.mp4"
assert hashlib.sha256(call("/api/v1/assets/"+video+"/original")[1]).hexdigest()==hashlib.sha256(original.read_bytes()).hexdigest()
assert call("/api/v1/assets/"+video+"/preview")[0]==200
assert call("/api/v1/assets/"+video+"/original",headers={"Range":"bytes=-16"})[1]==original.read_bytes()[-16:]
assert call("/api/v1/assets/"+video+"/original",headers={"Range":"bytes=8-3"})[0]==416
# A missing volume marker must stop writes rather than use a fallback directory.
marker=pathlib.Path(os.environ["MEDIA_ROOT"])/".archivist-volume"
marker.unlink()
assert call("/health/ready")[0]==503
assert call("/api/v1/imports",{"files":["fixtures/clip.mp4"],"copy_confirmed":True},headers={"Idempotency-Key":str(uuid.uuid4())})[0]==503
marker.write_text("wrong-volume")
assert call("/health/ready")[0]==503
marker.write_text("test-volume")
assert call("/health/ready")[0]==200
print("PASS: video import, metadata, original hash, preview, ranges and missing/wrong volume protection.")

# Real worker generates a playable proxy even when the AI endpoint is offline.
import subprocess,time
worker_env={k:v for k,v in os.environ.items() if k in {
    "PATH","MEDIA_ROOT","MEDIA_VOLUME_ID","ARCHIVIST_URL","ARCHIVIST_WORKER_TOKEN",
    "LMSTUDIO_URL","VISION_MODEL","VISION_VERSION","EMBEDDING_MODEL",
    "EMBEDDING_VERSION","EMBEDDING_DIMENSION","FFMPEG_BIN","FFPROBE_BIN","RUST_LOG"}}
with (pathlib.Path(os.environ["MEDIA_ROOT"]).parent/"worker.log").open("w") as log:
    process=subprocess.Popen(["target/debug/worker"],env=worker_env,stdout=log,stderr=log)
    try:
        for _ in range(60):
            status,proxy,headers=call("/api/v1/assets/"+video+"/playback")
            if status==200:break
            time.sleep(.5)
        else:raise AssertionError("Worker did not generate playback")
        assert headers["Content-Type"]=="video/mp4" and len(proxy)>0
        assert call("/api/v1/assets/"+video+"/playback",headers={"Range":"bytes=0-15"})[1]==proxy[:16]
        assert hashlib.sha256(call("/api/v1/assets/"+video+"/original")[1]).hexdigest()==metadata["sha256"]
    finally:
        process.terminate()
        process.wait(timeout=10)
print("PASS: worker playback generation, streaming Range and original unchanged with AI offline.")
