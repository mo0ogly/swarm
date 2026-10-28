"""Synthesize an original, restrained electronic instrumental for the Swarm film.

Inputs are presentation timing and oscillator parameters only; there are no
samples, remote calls, artist imitation or product policies. The output is a
100-second stereo PCM WAV plus peak/RMS provenance. The caller verifies the
encoded AAC stream separately. A nonfinite or clipped mix is rejected.
"""
from pathlib import Path
import numpy as np
import wave,json,hashlib
ROOT=Path(__file__).resolve().parents[4]
OUT=ROOT/'.cache/editorial-swarm-20261023/media-refresh'
RATE=48000
DURATION=100
BPM=90
BEAT=60/BPM
N=RATE*DURATION
mix=np.zeros((N,2),dtype=np.float64)
def note(midi):
    """Map a musical MIDI note number to its standard equal-tempered frequency."""
    return 440*2**((midi-69)/12)
def add(signal,start,pan=0):
    """Mix a finite mono voice into the bounded stereo timeline with equal-power pan."""
    a=round(start*RATE)
    if a>=N:return
    z=min(N,a+len(signal));sig=signal[:z-a]
    mix[a:z,0]+=sig*np.sqrt((1-pan)/2)
    mix[a:z,1]+=sig*np.sqrt((1+pan)/2)
# D minor, B-flat, F, C. Soft detuned pads with an original sparse top line.
chords=[[50,57,60,65],[46,53,58,62],[48,53,57,60],[48,55,60,64]]
for k,start in enumerate(np.arange(0,DURATION,8*BEAT)):
    chord=chords[k%4];length=8*BEAT+1.4;t=np.arange(round(length*RATE))/RATE
    env=np.minimum(1,t/1.3)*np.minimum(1,np.maximum(0,(length-t)/1.8))
    for j,midi in enumerate(chord):
        f=note(midi)
        sig=(np.sin(2*np.pi*f*.999*t)+np.sin(2*np.pi*f*1.001*t)+.16*np.sin(2*np.pi*2*f*t))/2.16
        add(.075*sig*env,start,(-.55+j*.36))
# Quiet bass anchors and a rounded pulse, deliberately below the explanatory text.
for k,start in enumerate(np.arange(0,DURATION,BEAT)):
    t=np.arange(round(.54*RATE))/RATE
    if k%2==0:
        f=note(chords[(k//8)%4][0]-12)
        add(.12*np.sin(2*np.pi*f*t)*np.exp(-t*5)*np.minimum(1,t/.02),start,0)
    if k%2==0:
        phase=2*np.pi*(44*t+23*(1-np.exp(-t*28))/28)
        add(.08*np.sin(phase)*np.exp(-t*15)*np.minimum(1,t/.004),start,0)
    if k%2==1:
        dur=.9;u=np.arange(round(dur*RATE))/RATE
        midi=chords[(k//8)%4][2+(k%4==1)] +12
        f=note(midi)
        pluck=(np.sin(2*np.pi*f*u)+.2*np.sin(2*np.pi*2*f*u))*np.exp(-u*7)*np.minimum(1,u/.008)
        add(.048*pluck,start,-.45 if k%4==1 else .45)
# Subtle stereo delay, then a smooth four-second head/tail fade.
for seconds,gain in [(.25,.13),(.5,.08)]:
    d=round(seconds*RATE);mix[d:]+=gain*mix[:-d,::-1]
f=np.arange(N)/RATE
fade=np.minimum(1,f/3)*np.minimum(1,np.maximum(0,(DURATION-f)/4))
mix*=fade[:,None]
assert np.isfinite(mix).all()
peak=float(np.max(np.abs(mix)));mix*=.53/max(peak,1e-12)
assert float(np.max(np.abs(mix)))<1
pcm=(mix*32767).astype('<i2')
OUT.mkdir(parents=True,exist_ok=True)
p=OUT/'soundtrack.wav'
with wave.open(str(p),'wb') as w:w.setnchannels(2);w.setsampwidth(2);w.setframerate(RATE);w.writeframes(pcm.tobytes())
info={'duration_seconds':DURATION,'sample_rate':RATE,'channels':2,'bpm':BPM,'peak_dbfs':20*np.log10(np.max(np.abs(mix))),'rms_dbfs':20*np.log10(np.sqrt(np.mean(mix**2))),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'origin':'Original procedural composition; no external recordings or samples','manual_listening':'Not established by numeric audio checks'}
(OUT/'soundtrack.json').write_text(json.dumps(info,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(info))
