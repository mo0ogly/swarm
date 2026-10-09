"""Build caption-free previews from real, locally captured UI screenshots."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parent
for lang in ('fr','en'):
    frames=[root/f'graph-tour-{lang}.png',root.parents[1]/'training/casa-pizza/tutoriels/frames'/f'14-tache-{lang}.png',root/f'dark-{lang}.png']
    args=[]
    for frame in frames: args += ['-loop','1','-t','3','-i',str(frame)]
    chain=';'.join(f'[{i}:v]scale=1264:712:force_original_aspect_ratio=decrease,pad=1264:712:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=12[v{i}]' for i in range(3))+';[v0][v1][v2]concat=n=3:v=1:a=0[out]'
    subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y',*args,'-filter_complex',chain,'-map','[out]','-c:v','libx264','-pix_fmt','yuv420p','-movflags','+faststart',str(root/f'graph-tour-{lang}.mp4')],check=True)
    subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(root/f'graph-tour-{lang}.mp4'),'-filter_complex','fps=2,scale=900:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse','-loop','0',str(root/f'graph-tour-{lang}.gif')],check=True)
