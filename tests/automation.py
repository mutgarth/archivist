import importlib.util
import json
import io
import sqlite3
import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('automation',Path(__file__).resolve().parents[1]/'scripts/automation.py')
a=importlib.util.module_from_spec(spec);spec.loader.exec_module(a)

class AutomationTests(unittest.TestCase):
 def test_decision_distribution_and_uncertainty(self):
  probabilities={f:0.0 for f in a.FOLDERS};probabilities['Animais']=1.0
  answer={'type':'choice','choice':'Animais','confidence':.9,'probabilities':probabilities}
  response={'answers':{'folder':answer}}
  self.assertEqual(a.parse_decision(response)['folder'],'Animais')
  answer['confidence']=.4
  self.assertEqual(a.parse_decision(response)['folder'],'Revisar')
  for invalid in (float('nan'),float('inf'),True,-1,2):
   answer['confidence']=invalid
   with self.assertRaises(ValueError):a.parse_decision(response)
  answer['confidence']=.9;answer['choice']='../../source'
  with self.assertRaises(ValueError):a.parse_decision(response)
  answer['choice']='Comida e cozinha'
  with self.assertRaises(ValueError):a.parse_decision(response)
  answer['choice']='Animais';probabilities['Animais']=.7
  with self.assertRaises(ValueError):a.parse_decision(response)
  probabilities['Animais']=1.;del probabilities['Revisar']
  with self.assertRaises(ValueError):a.parse_decision(response)
 def test_decision_payload_contains_described_choices(self):
  payload=a.decision_payload('local-decision','untrusted evidence')
  self.assertEqual(payload['state'],'untrusted evidence')
  self.assertEqual(set(payload['questions']['folder']['criteria']),set(a.FOLDERS))
  self.assertTrue(all(payload['questions']['folder']['criteria'].values()))
  self.assertNotIn('messages',payload)
 def test_systemone_classification_persists_and_retries_invalid_response(self):
  worker=a.Automation.__new__(a.Automation)
  worker.model='decision';worker.model_version='decision-q8-v1';worker.decision_mode='systemone';worker.decision_minimum=.65
  worker.db=sqlite3.connect(':memory:')
  worker.db.execute('CREATE TABLE retries(asset TEXT PRIMARY KEY,retry_at REAL)')
  worker.db.execute('CREATE TABLE decisions(asset TEXT,analysis_hash TEXT,model TEXT,response TEXT,elapsed_ms REAL,created_at REAL)')
  worker.counts={'errors':0};worker.report=lambda phase:None
  writes=[];worker.internal=lambda path,body:writes.append(body)
  asset={'id':'one','status':'completed','analysis_hash':'a'*64,'mime':'image/png','description':'Gato'}
  probabilities={f:float(f=='Animais') for f in a.FOLDERS}
  response={'answers':{'folder':{'type':'choice','choice':'Animais','confidence':.9,'probabilities':probabilities}}}
  with patch.dict(os.environ,{'LMSTUDIO_URL':'http://localhost:1234/v1'}):
   with patch.object(a.urllib.request,'urlopen',return_value=io.BytesIO(json.dumps(response).encode())) as request:
    worker.classify([asset])
    self.assertTrue(request.call_args.args[0].full_url.endswith('/v1/systemone'))
   self.assertEqual(writes[0]['folder'],'Animais')
   self.assertEqual(worker.db.execute('SELECT COUNT(*) FROM decisions').fetchone()[0],1)
   with patch.object(a.urllib.request,'urlopen',return_value=io.BytesIO(b'{}')):
    worker.classify([asset])
   self.assertEqual(len(writes),1)
   self.assertGreater(worker.db.execute('SELECT retry_at FROM retries').fetchone()[0],time.time())
  worker.db.close()
 def test_stability_restarts_when_upload_changes(self):
  s=a.Stability(30);signature=(5,1,1,1)
  self.assertFalse(s.ready(Path('x'),signature,0))
  self.assertFalse(s.ready(Path('x'),signature,29))
  self.assertTrue(s.ready(Path('x'),signature,31))
  self.assertFalse(s.ready(Path('x'),(6,1,1,1),32))
 def test_scan_ignores_symlinks_temporary_and_internal_files(self):
  with tempfile.TemporaryDirectory() as d:
   p=Path(d);(p/'a.MOV').write_bytes(b'abc');(p/'a.MOV.part').write_bytes(b'abc')
   (p/'.hidden.mp4').write_bytes(b'abc');(p/'link.mp4').symlink_to(p/'a.MOV')
   (p/'internal').mkdir();(p/'internal/x.mp4').write_bytes(b'a')
   self.assertEqual(list(a.candidates(p,{p/'internal'})),[p/'a.MOV'])
 def test_copy_preserves_original_and_detects_change(self):
  with tempfile.TemporaryDirectory() as d:
   p=Path(d);src=p/'test.mov';src.write_bytes(b'original');media=p/'media';media.mkdir()
   sig=a.fingerprint(src);path,sha=a.stable_copy(src,media,sig)
   self.assertEqual(src.read_bytes(),b'original');self.assertEqual(a.digest(media/path),sha)
   src.write_bytes(b'changed')
   with self.assertRaises(ValueError):a.stable_copy(src,media,sig)
 def test_rejects_invalid_or_truncated_model_results(self):
  result={'folder':'Animais','tags':['gato'],'reason':'Gato visível','confidence':.9}
  def response(r,finish='stop'):return {'choices':[{'finish_reason':finish,'message':{'content':json.dumps(r)}}]}
  self.assertEqual(a.parse_classification(response(result))['folder'],'Animais')
  result['confidence']=.3;self.assertEqual(a.parse_classification(response(result))['folder'],'Revisar')
  result['folder']='../../private'
  with self.assertRaises(ValueError):a.parse_classification(response(result))
  with self.assertRaises(ValueError):a.parse_classification(response(result,'length'))
 def test_invalid_output_goes_to_review(self):
  result=a.classification_or_review({"choices":[]})
  self.assertEqual(result["folder"],"Revisar")
  self.assertEqual(result["confidence"],0)
 def test_duplicate_and_restart_do_not_reimport(self):
  with tempfile.TemporaryDirectory() as d:
   p=Path(d);media=p/'media';media.mkdir();source=p/'source';source.mkdir();f=source/'x.mp4';f.write_bytes(b'video');(source/'empty.mp4').touch()
   config={'roots':[str(source)],'state_path':str(p/'state.sqlite'),'settle_seconds':0}
   env={'MEDIA_ROOT':str(media),'ARCHIVIST_URL':'http://localhost','ARCHIVIST_WORKER_TOKEN':'test','VISION_MODEL':'test','VISION_VERSION':'test'}
   with patch.dict(os.environ,env):
    worker=a.Automation(config);calls=[]
    worker.request=lambda *args,**kwargs:calls.append(args) or {'assets':['one']}
    worker.scan([]);worker.scan([]);worker.scan([])
    self.assertEqual(len(calls),1)
    worker.db.close()
    restarted=a.Automation(config);restarted.request=worker.request;restarted.scan([])
    self.assertEqual(len(calls),1)
    duplicate=source/'duplicate.mp4';duplicate.write_bytes(b'video')
    restarted.scan([{'sha256':a.digest(f)}]);restarted.scan([{'sha256':a.digest(f)}])
    self.assertEqual(len(calls),1);self.assertEqual(restarted.counts['duplicates'],1)
    restarted.db.close()
 def test_failed_import_is_not_marked_done(self):
  with tempfile.TemporaryDirectory() as d:
   p=Path(d);(p/'media').mkdir();(p/'input').mkdir();(p/'input/x.mp4').write_bytes(b'video')
   with patch.dict(os.environ,{'MEDIA_ROOT':str(p/'media'),'ARCHIVIST_URL':'http://localhost','ARCHIVIST_WORKER_TOKEN':'test','VISION_MODEL':'test','VISION_VERSION':'test'}):
    worker=a.Automation({'roots':[str(p/'input')],'state_path':str(p/'state.sqlite'),'settle_seconds':0})
    def fail(*args,**kwargs):raise ConnectionError()
    worker.request=fail;worker.scan([]);worker.scan([])
    self.assertEqual(worker.db.execute('SELECT state FROM files').fetchone()[0],'error')
    worker.db.close()

if __name__=='__main__':unittest.main()
