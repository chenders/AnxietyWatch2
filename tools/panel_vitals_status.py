#!/usr/bin/env python3
"""Publish latest durable BP and Polar H10 values for the local panel."""
import json, os, pathlib, subprocess, tempfile, time

OUTPUT = pathlib.Path(os.environ.get("PANEL_VITALS_STATUS_JSON", "/home/chris/anxietywatch-panel/docs/design/pi-panel/direction-a-clinical/vitals_status.json"))
DATABASE = os.environ.get("PANEL_VITALS_DATABASE", "vitals")
INTERVAL = float(os.environ.get("PANEL_VITALS_INTERVAL_SECONDS", "1"))
SQL = r"""
WITH bp_latest AS (
 SELECT session_id,seq,timestamp FROM samples WHERE source=6 AND type=6 ORDER BY timestamp DESC LIMIT 1
), bp AS (
 SELECT json_build_object('sample_at',extract(epoch FROM b.timestamp),'session_id',b.session_id,'seq',b.seq,
  'systolic',max(s.value) FILTER(WHERE s.type=6),'diastolic',max(s.value) FILTER(WHERE s.type=7),
  'map',max(s.value) FILTER(WHERE s.type=8),'pulse_bpm',max(s.value) FILTER(WHERE s.type=1),
  'event_time','arrival_estimate') value
 FROM bp_latest b JOIN samples s ON s.source=6 AND s.session_id=b.session_id AND s.seq=b.seq
 GROUP BY b.session_id,b.seq,b.timestamp
), h10 AS (
 SELECT json_build_object('sample_at',extract(epoch FROM timestamp),'session_id',session_id,'seq',seq,
  'hr',value,'test_origin',CASE WHEN convert_from(extra,'UTF8') LIKE '%test_origin=polar_h10_emulator%' THEN 'polar_h10_emulator' ELSE 'physical_device' END) value
 FROM samples WHERE source=1 AND type=1 ORDER BY timestamp DESC LIMIT 1
)
SELECT json_build_object('schema',1,'bp',(SELECT value FROM bp),'h10',(SELECT value FROM h10))::text;
"""

def query():
    p=subprocess.run(["psql","-X","-qAt","-U","vitals","-d",DATABASE,"-c",SQL],text=True,capture_output=True,check=True)
    return json.loads(p.stdout.strip())

def publish(data):
    data["written_at"]=time.time(); OUTPUT.parent.mkdir(parents=True,exist_ok=True)
    fd,tmp=tempfile.mkstemp(prefix=".vitals_status.",dir=OUTPUT.parent)
    try:
        with os.fdopen(fd,"w") as f: json.dump(data,f,separators=(",",":"),allow_nan=False); f.write("\n"); f.flush(); os.fsync(f.fileno())
        os.chmod(tmp,0o600); os.replace(tmp,OUTPUT)
    finally:
        try: os.unlink(tmp)
        except FileNotFoundError: pass

def main():
    while True:
        try: publish(query())
        except Exception as e: publish({"schema":1,"bp":None,"h10":None,"error":str(e)})
        time.sleep(INTERVAL)
if __name__=="__main__": main()
