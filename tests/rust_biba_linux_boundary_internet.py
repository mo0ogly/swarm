#!/usr/bin/env python3
"""Verify the additive Internet-socket refusal on fresh owned synthetic files.
An unconfined control precedes both confined cases: the mandatory profile alone
reproduces the BF-14 gap (AF_INET/AF_INET6 creation still allowed), then the
additive engine policy denies Internet creation with EPERM while the mandatory
profile's AF_UNIX verdict is preserved. Fixture drivers only, never the product
binary or acceptance proof. No account, live Root, global observation, retry or
whole-test watchdog is used.
"""
import argparse,json,subprocess,tempfile
from fixture_area import fixture_area
from pathlib import Path
import rust_agent_model_route as proof


def run(driver,internet_driver,out):
    """Retain both drivers' argv, channels and per-domain verdicts with errno.
    IO, unavailable-kernel and assertion failures propagate without PASS; the
    owned Root is retained for diagnosis rather than deleted or retried.
    """
    out.mkdir(parents=True,exist_ok=False)
    root=Path(tempfile.mkdtemp(prefix='swarm-biba-internet-', dir=fixture_area()))
    private=root/'.swarm';private.mkdir();workspace=root/'out';workspace.mkdir()
    proof.publish(private/'state.synthetic',b'OWNED_SYNTHETIC_NO_SECRET')
    proof.publish(out/'fixture.json',dict(root=str(root),driver_sha256=proof.digest(driver.read_bytes()),internet_driver_sha256=proof.digest(internet_driver.read_bytes()),source_sha256=proof.digest(Path(__file__).read_bytes()),global_watchdog=None))
    script=root/'internet.py'
    source='''import socket,errno,json
facts={}
def probe(name,make):
\ttry:make().close();facts[name]='created'
\texcept OSError as e:facts[name]='errno'+str(e.errno)
probe('inet',lambda:socket.socket(socket.AF_INET,socket.SOCK_STREAM))
probe('inet6',lambda:socket.socket(socket.AF_INET6,socket.SOCK_STREAM))
probe('unix_socket',lambda:socket.socket(socket.AF_UNIX,socket.SOCK_STREAM))
probe('inet_pair',lambda:socket.socketpair(socket.AF_INET,socket.SOCK_STREAM))
with open('internet-facts.json','x') as f:json.dump(facts,f)
print('OWNED_PROVIDER_OUTPUT',flush=True)
'''
    proof.publish(script,source.encode())
    def direct(label):
        """Unconfined control keeps original channels and verdicts."""
        proof.publish(out/(label+'-argv.json'),['/usr/bin/python3',str(script)])
        r=subprocess.run(['/usr/bin/python3',str(script)],cwd=workspace,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        proof.publish(out/(label+'.stdout'),r.stdout);proof.publish(out/(label+'.stderr'),r.stderr)
        assert r.returncode==0 and r.stdout==b'OWNED_PROVIDER_OUTPUT\n',(label,r.returncode,r.stderr.decode())
        return json.loads((workspace/'internet-facts.json').read_text())
    def invoke(label,driver_path):
        """Keep exact argv and original bytes; an exit alone proves no assertion."""
        proof.publish(out/(label+'-argv.json'),[str(driver_path),str(root),str(workspace),str(script)])
        r=subprocess.run([str(driver_path),str(root),str(workspace),str(script)],cwd=workspace,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        proof.publish(out/(label+'.stdout'),r.stdout);proof.publish(out/(label+'.stderr'),r.stderr)
        assert r.returncode==0,(label,r.returncode,r.stderr.decode())
        assert r.stdout==b'OWNED_PROVIDER_OUTPUT\n',label
        return json.loads((workspace/'internet-facts.json').read_text())
    control=direct('internet-control')
    # This host lacks IPv6 (EAFNOSUPPORT=97) and the kernel only implements
    # socketpair() for AF_UNIX (EOPNOTSUPP=95): kernel non-support is a fact of
    # the environment, not a policy verdict. The control records the actual
    # verdicts and the confined comparisons are per-domain against them.
    assert control['inet']=='created' and control['unix_socket']=='created',control
    assert control['inet6']=='errno97',control
    assert control['inet_pair']=='errno95',control
    (workspace/'internet-facts.json').unlink()
    profile_only=invoke('profile-only',driver)
    # The mandatory profile refuses only AF_UNIX: Internet creation stays
    # reachable — exactly the BF-14 gap this increment closes.
    assert profile_only['inet']=='created' and profile_only['inet_pair']=='errno95' and profile_only['unix_socket']=='errno1',profile_only
    assert profile_only['inet6']==control['inet6'],profile_only
    (workspace/'internet-facts.json').unlink()
    denied=invoke('internet-denied',internet_driver)
    # Seccomp evaluates before the kernel's family validation, so every domain
    # the policy denies returns EPERM (1) uniformly, including AF_INET6 and the
    # AF_INET socketpair the unconfined kernel would have refused differently.
    assert denied['inet']=='errno1' and denied['inet6']=='errno1' and denied['inet_pair']=='errno1',denied
    # The mandatory profile's AF_UNIX verdict is preserved by the second filter.
    assert denied['unix_socket']=='errno1',denied
    result=dict(status='PASS',scope='Native standalone boundary fixture only',control=control,profile_only_allows_inet=True,profile_only=profile_only,internet_denied=denied,product_qualified=False,installed=False)
    proof.publish(out/'result.json',result);print('PASS native-boundary-internet-control-profile-only-denied',flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--driver',type=Path,required=True);p.add_argument('--internet-driver',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();run(a.driver.resolve(),a.internet_driver.resolve(),a.output.resolve())
