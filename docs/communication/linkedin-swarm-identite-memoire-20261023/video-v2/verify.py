"""Verify the produced film through FFprobe, full decode and extracted evidence.

Usage: bundled-python verify.py /absolute/path/film.mp4 /fresh/evidence/path
Requires local FFmpeg/FFprobe and bundled NumPy/Pillow. Checks actual codec,
geometry, timing, frame count, captions, chapter ranges, audio samples and MP4
faststart order. Extracted PNGs still require visual review. No test deadline or
Swarm operation is introduced, and passing media checks proves no runtime claim.
"""
from pathlib import Path
import sys,subprocess,json,hashlib,struct
import numpy as np
HERE=Path(__file__).resolve().parent

def run(argv):
    """Execute a local decoder without a global watchdog; retain the original error."""
    p=subprocess.run(argv,capture_output=True)
    if p.returncode:raise RuntimeError(json.dumps({'argv':argv,'code':p.returncode,'stderr':p.stderr.decode(errors='replace')}))
    return p

def verify(film,out):
    """Write fresh media evidence; refuse nonmatching structure or malformed cues."""
    out.mkdir(parents=True,exist_ok=False)
    spec=json.loads((HERE/'scenes.json').read_text())
    cmds=[]
    argv=['ffprobe','-v','error','-count_frames','-show_streams','-show_format','-show_chapters','-of','json',str(film)];cmds.append(argv)
    probe=json.loads(run(argv).stdout);(out/'probe.json').write_text(json.dumps(probe,ensure_ascii=False,indent=2)+'\n')
    video=next(s for s in probe['streams'] if s['codec_type']=='video');audio=next(s for s in probe['streams'] if s['codec_type']=='audio')
    assert (video['width'],video['height'],video['r_frame_rate'],video['codec_name'],video['pix_fmt'])==(1920,1080,'30/1','h264','yuv420p')
    assert int(video['nb_read_frames'])==3000
    assert abs(float(probe['format']['duration'])-spec['duration'])<1/spec['fps']
    assert audio['codec_name']=='aac' and audio['channels']==2 and int(audio['sample_rate'])==48000
    subs=[s for s in probe['streams'] if s['codec_type']=='subtitle'];assert len(subs)==2
    assert {s['tags']['language'] for s in subs}=={'fra','eng'}
    assert len(probe['chapters'])==len(spec['scenes'])
    for got,want in zip(probe['chapters'],spec['scenes']):
        assert float(got['start_time'])==want['start'];assert float(got['end_time'])==want['start']+want['duration']
    for lang in ['fr','en']:
        srt=(HERE/f'Swarm_Architecture_Film_v2.{lang}.srt').read_text();assert len(srt.strip().split('\n\n'))==27
        for cue in spec['cues']:assert cue[lang] in srt
    argv=['ffmpeg','-hide_banner','-v','error','-i',str(film),'-map','0:v:0','-map','0:a:0','-f','null','-'];cmds.append(argv)
    result=run(argv);(out/'decode.stderr').write_bytes(result.stderr)
    argv=['ffmpeg','-v','error','-i',str(film),'-map','0:a:0','-f','f32le','-'];cmds.append(argv)
    raw=run(argv).stdout;pcm=np.frombuffer(raw,dtype='<f4');assert pcm.size and np.isfinite(pcm).all()
    peak=float(np.abs(pcm).max());rms=float(np.sqrt(np.mean(pcm**2)));assert 0<rms<1 and peak<1
    # Inspect actual encoded images, at three named times per scene.
    samples=[{'scene':s['id'],'phase':i,'frame':round((s['start']+s['duration']*v)*spec['fps'])} for s in spec['scenes'] for i,v in enumerate([.22,.56,.86])]
    select='+'.join(f'eq(n,{x["frame"]})' for x in samples)
    argv=['ffmpeg','-v','error','-i',str(film),'-vf',"select='"+select+"'",'-fps_mode','vfr',str(out/'frame-%02d.png')];cmds.append(argv);run(argv)
    for i,x in enumerate(samples,1):
        x['path']=f'frame-{i:02}.png';assert (out/x['path']).is_file();x['sha256']=hashlib.sha256((out/x['path']).read_bytes()).hexdigest()
    boxes=[]
    with film.open('rb') as f:
        size=film.stat().st_size;pos=0
        while pos+8<=size:
            f.seek(pos);length,kind=struct.unpack('>I4s',f.read(8))
            if length==1:length=struct.unpack('>Q',f.read(8))[0]
            if length==0:length=size-pos
            assert length>=8
            boxes.append({'kind':kind.decode(errors='replace'),'offset':pos,'bytes':length});pos+=length
    names=[x['kind'] for x in boxes];assert names.index('moov')<names.index('mdat')
    report={'film':str(film),'sha256':hashlib.sha256(film.read_bytes()).hexdigest(),'bytes':film.stat().st_size,'seconds':float(probe['format']['duration']),'frames':3000,'full_decode':'PASS','geometry':'1920x1080 30fps H264 yuv420p','audio':{'codec':'AAC','channels':2,'sample_rate':48000,'decoded_peak_dbfs':20*np.log10(peak),'decoded_rms_dbfs':20*np.log10(rms),'clipping':False,'manual_audition':'Not established by these automated checks'},'subtitles':['fra','eng'],'chapters':9,'samples':samples,'boxes':boxes,'visual_review':'Pending explicit inspection of the extracted images','scope':'Media validity only; no Swarm runtime qualification'}
    (out/'commands.json').write_text(json.dumps(cmds,ensure_ascii=False,indent=2)+'\n');(out/'result.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps({k:report[k] for k in ['sha256','bytes','seconds','frames','full_decode','geometry','chapters']}))

if __name__=='__main__':
    try:verify(Path(sys.argv[1]).resolve(),Path(sys.argv[2]).resolve())
    except Exception:
        import traceback
        failure=traceback.format_exc();out=Path(sys.argv[2]).resolve()
        if out.is_dir():(out/'failure.txt').write_text(failure)
        raise
