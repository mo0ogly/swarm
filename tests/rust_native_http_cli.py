#!/usr/bin/env python3
"""Verify the actual native CLI HTTP process on private ephemeral Roots.

Normal binary required; no provider, live mission, Go restart, global watchdog or
silent retry. Success requires protected HTTP content, actual saved consumer/state,
owner records, graceful SIGINT and BrokenPipe recovery. stdout backpressure that
never returns is not covered. Keys/CSRF/preview tokens stay out of proof streams.
Failed Roots survive; successful ones are removed only after owned processes exit.
"""
import argparse,hashlib,http.client,json,os,shutil,signal,subprocess,tempfile,time,traceback,threading
from pathlib import Path
import rust_agent_model_route as retained


def clean(value):
    """Redact private access/proposal tokens from fixture command/HTTP artifacts."""
    if isinstance(value,dict):return {k:('<redacted>' if k in ['csrf_token','preview_token'] else clean(v)) for k,v in value.items()}
    if isinstance(value,list):return [clean(v) for v in value]
    return value


class Server:
    """Own exactly one CLI process and its pipes; no arbitrary PID/root selected."""
    def __init__(self,binary,root,output,label,*,environment=None):
        """Start the owned CLI with an explicit optional inherited environment.

        Existing callers retain subprocess inheritance. A fixture supplying its
        own catalogue must pass its actual environment, as CLI and HTTP model
        admission consume the same CODEX_HOME. Do not serialize environment
        contents: they may contain credentials. Startup/cleanup errors propagate.
        """
        self.binary=binary;self.root=root;self.output=output;self.label=label;self.records=[];self.requests=[];self.stop_sent=False
        self.argv=[str(binary),'web','--root',str(root),'--json','--lang','en']
        self.start=time.time();self.process=subprocess.Popen(self.argv,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=environment)
        self.stdout_open=True;self.stderr=b'';self.capture_thread=None;self.capture_errors=[];self.capture_bytes=0;self.capture_capacity=None
        self.capture_changed=threading.Event();self.capture_done=False
        retained.publish(output/(label+'-process.json'),dict(argv=self.argv,pid=self.process.pid,start=self.start,global_watchdog=None,owned=True))
        print(json.dumps(dict(state='RUN',case=label,phase='waiting-bound')),flush=True)
        try:
            bound=self.record();assert bound['event']=='web-bound' and bound['runtime_ready'] is False
            address=bound['address'];host,port=address.rsplit(':',1);assert host=='127.0.0.1' and int(port)>0
            self.host=host;self.port=int(port);self.address=address
            digest=hashlib.sha256(address.encode()).hexdigest()
            self.key=(root/'.swarm'/('web-session-'+digest[:16])).read_text()
            assert self.key.startswith('session-') and len(self.key)==32
            self.cookie='swarm_session_'+digest[:12]+'='+self.key
        except BaseException as primary:
            try:self.cleanup()
            except BaseException as secondary:raise BaseExceptionGroup('Bootstrap and owned cleanup failed',[primary,secondary])
            raise

    def capture_async(self,path):
        """Continuously retain stdout when a browser owns the HTTP conversation.

        Opt-in relinquishes synchronous record()/observed request()/stop(); use
        cleanup() to signal, join the reader and close owned pipes. A fresh private
        file inside this fixture's output retains exact bytes. Invalid frames and
        IO/close errors survive and fail cleanup. No retry, time/volume cap or Root
        mutation; pipe capacity is diagnostic data, not an execution policy.
        """
        if self.capture_thread is not None:raise ValueError('capture already active')
        path=Path(path).resolve()
        if not path.is_relative_to(self.output.resolve()):raise ValueError('capture outside fixture output')
        stream=path.open('xb');os.chmod(path,0o600)
        try:
            import fcntl
            if hasattr(fcntl,'F_GETPIPE_SZ'):self.capture_capacity=fcntl.fcntl(self.process.stdout.fileno(),fcntl.F_GETPIPE_SZ)
        except ImportError:pass  # Capacity remains explicitly unknown off POSIX.
        except BaseException as primary:
            try:stream.close()
            except BaseException as secondary:raise BaseExceptionGroup('Capture setup and close failed',[primary,secondary])
            raise
        def drain():
            """Consume to actual EOF; retain malformed originals while reporting failure."""
            try:
                for raw in self.process.stdout:
                    stream.write(raw);stream.flush();self.capture_bytes+=len(raw);self.capture_changed.set()
                    try:
                        value=json.loads(raw)
                        if value.get('schema_version')!=1:raise ValueError('capture protocol schema')
                        self.records.append(clean(value))
                    except BaseException as error:self.capture_errors.append(error)
            except BaseException as error:self.capture_errors.append(error)
            finally:
                try:stream.close()
                except BaseException as error:self.capture_errors.append(error)
                self.capture_done=True;self.capture_changed.set()
        self.capture_thread=threading.Thread(target=drain,name='owned-web-output',daemon=True)
        try:self.capture_thread.start()
        except BaseException as primary:
            self.capture_thread=None
            try:stream.close()
            except BaseException as secondary:raise BaseExceptionGroup('Capture start and close failed',[primary,secondary])
            raise

    def wait_capture_progress(self,previous):
        """Wait for actual additional bytes or a retained failure/EOF, no deadline.

        Only async capture owns the stream. Recheck after clearing the event to
        avoid a lost wakeup; no iteration/byte ceiling or automatic retry applies.
        """
        if self.capture_thread is None:raise ValueError('capture required')
        while self.capture_bytes<=previous and not self.capture_done:
            self.capture_changed.clear()
            if self.capture_bytes>previous or self.capture_done:break
            self.capture_changed.wait()
        if self.capture_errors:raise BaseExceptionGroup('Capture failed',self.capture_errors)
        if self.capture_bytes<=previous:raise ValueError('capture ended before observation')

    def record(self):
        """Read one actual compact protocol frame, with no terminal watchdog."""
        if self.capture_thread is not None:raise ValueError('synchronous observer unavailable during capture')
        raw=self.process.stdout.readline();assert raw,'CLI stdout ended before required frame'
        value=json.loads(raw);assert value['schema_version']==1
        self.records.append(clean(value));return value

    def request(self,path,body=None,csrf=None,authorized=True,status=200,observe=True):
        """Use actual HTTP and exact Host-scoped cookie/CSRF; retain redacted content.

        Observer frames follow completed jobs only; pre-Root access refusals appear
        in final summary. Unknown client response after output failure is not success.
        """
        if observe and self.capture_thread is not None:raise ValueError('observed request unavailable during capture')
        headers={'Origin':'http://'+self.address}
        if authorized:headers['Cookie']=self.cookie
        if csrf is not None:headers['X-Swarm-CSRF']=csrf
        data=None if body is None else json.dumps(body,ensure_ascii=False,separators=(',',':')).encode()
        if data is not None:headers['Content-Type']='application/json'
        connection=http.client.HTTPConnection(self.host,self.port,timeout=None)
        try:
            connection.request('GET' if body is None else 'POST',path,body=data,headers=headers)
            response=connection.getresponse();raw=response.read();actual=response.status;content_type=response.getheader('Content-Type','')
        finally:connection.close()
        if content_type.split(';',1)[0]=='application/json':parsed=json.loads(raw)
        else:
            # Native pre-Root refusal is a public fixed text code, never JSON.
            assert status==403 and actual==403 and raw==b'web-access-refused',(actual,content_type)
            parsed=raw.decode('ascii')
        self.requests.append(dict(path=path,method='GET' if body is None else 'POST',status=actual,content_type=content_type,response=clean(parsed)))
        assert actual==status,(path,actual,clean(parsed))
        if observe:
            frame=self.record();assert frame['event']=='web-outcome' and frame['outcome']['status']==status
            assert self.key not in json.dumps(frame)
        return parsed

    def prepare(self,event):
        """Positive protected witnesses precede output failure; preview remains read-only."""
        session=self.request('/api/v1/native-session');csrf=session['csrf_token'];assert csrf!=self.key
        doc=self.request('/api/v1/native-settings');before=(self.root/'.swarm/engine-settings.json').read_bytes()
        values=doc['values'];values['review_delivery_max_bytes']='3'
        change=dict(schema_version=doc['schema_version'],event_id=event,expected_revision=doc['revision'],values=values,reason='Owned native CLI HTTP proof',preview_token='')
        preview=self.request('/api/v1/native-settings/preview',change,csrf);assert (self.root/'.swarm/engine-settings.json').read_bytes()==before
        change['preview_token']=preview['preview_token'];return csrf,change

    def stop(self):
        """SIGINT only this owned child, then require the actual drained-stop summary."""
        if self.capture_thread is not None:raise ValueError('use cleanup during asynchronous capture')
        self.process.send_signal(signal.SIGINT);self.stop_sent=True
        frame=self.record();assert frame['event']=='web-stopped'
        self.stderr=self.process.stderr.read();code=self.process.wait();assert code==0,(code,clean(json.loads(self.stderr)) if self.stderr else None)
        return frame

    def cleanup(self):
        """Attempt owned signal/drain/wait/pipe closes and retain failures independently.

        No kill timeout or retry is invented. A stall remains observable in driver
        stage/process records and must be diagnosed by the active test controller.
        Opt-in capture is joined after signaling so stdout cannot backpressure
        browser completion; protocol/IO failures remain cleanup failures.
        """
        errors=[]
        if self.process.poll() is None and not self.stop_sent:
            try:self.process.send_signal(signal.SIGINT);self.stop_sent=True
            except BaseException as error:errors.append(error)
        if self.capture_thread is not None:
            try:self.capture_thread.join()
            except BaseException as error:errors.append(error)
            errors.extend(self.capture_errors)
        if self.stdout_open:
            try:
                remainder=self.process.stdout.read()
                if remainder:retained.publish(self.output/(self.label+'-remaining-stdout.bin'),remainder)
            except BaseException as error:errors.append(error)
        try:
            tail=self.process.stderr.read()
            if tail:self.stderr+=tail
        except BaseException as error:errors.append(error)
        try:self.process.wait()
        except BaseException as error:errors.append(error)
        for stream in [self.process.stdout,self.process.stderr]:
            if stream is not None and not stream.closed:
                try:stream.close()
                except BaseException as error:errors.append(error)
        try:
            # CLI output contracts exclude credentials; assert before publication.
            if hasattr(self,'key'):assert self.key.encode() not in self.stderr
            retained.publish(self.output/(self.label+'-records.json'),dict(records=self.records,requests=self.requests,exit=self.process.returncode,start=self.start,end=time.time()))
            retained.publish(self.output/(self.label+'-stderr.bin'),self.stderr)
        except BaseException as error:errors.append(error)
        if errors:raise BaseExceptionGroup('Owned CLI cleanup/evidence failed',errors)


class NativeCliFailure(RuntimeError):
    """Safe helper failure; result/argv hold originals without rendering their data.

    evidence_directory is a new owned0700directory outside the tested Root. It
    preserves stdout/stderr/argv privately, including uncertain saved-state data.
    Nothing here retries, repairs Root, stops a process or grants provider access.
    """
    def __init__(self,argv,result):
        super().__init__('Native CLI contract failed; exit code '+str(result.returncode))
        self.argv=argv;self.result=result;self.evidence_directory=None


def cli_failure(argv,result):
    """Preserve exact command diagnostics privately, never in public traceback text.

    Failure of diagnostic publication retains both original CLI failure and the
    publication cause. The caller separately chains a parse error when relevant.
    """
    error=NativeCliFailure(argv,result)
    try:
        directory=Path(tempfile.mkdtemp(prefix='swarm-native-cli-failure-'))
        assert directory.stat().st_mode&0o777==0o700 and directory.stat().st_uid==os.getuid()
        error.evidence_directory=directory
        error.add_note('Original private CLI diagnostics retained at '+str(directory))
        retained.publish(directory/'stdout.bin',result.stdout);retained.publish(directory/'stderr.bin',result.stderr)
        retained.publish(directory/'command.json',dict(argv=argv,exit=result.returncode,scope='Private original evidence; may contain sensitive fixture data'))
    except BaseException as secondary:raise BaseExceptionGroup('CLI contract and private diagnostic publication failed',[error,secondary])
    return error


def cli(binary,root,arguments):
    """Actual supported CLI; no shell/provider or fixture write into the tested Root.

    Exit/JSON failures retain exact originals in a separate private owned directory;
    malformed successful output never becomes an empty success. Launch failures
    propagate as their original OS error. No repeated operation or watchdog.
    """
    argv=[str(binary),*arguments,'--root',str(root),'--json','--lang','en']
    result=subprocess.run(argv,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    if result.returncode:raise cli_failure(argv,result)
    try:return json.loads(result.stdout)
    except (json.JSONDecodeError,UnicodeDecodeError) as cause:raise cli_failure(argv,result) from cause


def owned_root(binary):
    """Create private disposable fixture and explicitly initialize it through CLI."""
    root=Path(tempfile.mkdtemp(prefix='swarm-cli-http97-'));assert root.stat().st_mode&0o777==0o700
    try:cli(binary,root,['init'])
    except BaseException as error:
        error.add_note('Owned initialization fixture retained at '+str(root));raise
    return root


def finish(server,primary):
    """Keep the primary and every owned cleanup failure without masking either."""
    try:server.cleanup()
    except BaseException as error:
        if primary is not None:raise BaseExceptionGroup('Scenario and cleanup failed',[primary,error])
        raise
    if primary is not None:raise primary


def normal(binary,output):
    """Protected API, saved state, exact replay, expected403 and actual SIGINT close."""
    root=owned_root(binary);server=Server(binary,root,output,'normal');primary=None
    try:
        csrf,change=server.prepare('cli-http97')
        capability=server.request('/api/v1/native-capabilities');assert capability['profile']=='rust-native-http1-observed'
        server.request('/api/v1/works',authorized=False,status=403,observe=False)
        applied=server.request('/api/v1/native-settings/apply',change,csrf);assert applied['replayed'] is False and applied['document']['revision']=='2'
        receipt=server.records[-1]['outcome']['recorded_configuration'];assert receipt['event_id']=='cli-http97' and receipt['event_revision']=='2' and receipt['replayed'] is False
        saved=(root/'.swarm/engine-settings.json').read_bytes()
        replay=server.request('/api/v1/native-settings/apply',change,csrf);assert replay['replayed'] is True and (root/'.swarm/engine-settings.json').read_bytes()==saved
        assert server.request('/api/v1/works')==[]
        shown=cli(binary,root,['engine-settings','show']);assert shown['revision']=='2' and shown['values']['review_delivery_max_bytes']=='3'
        summary=server.stop();assert summary['completed_jobs']==7 and summary['observed_outcomes']==7 and summary['configuration_outcomes']==2 and summary['rejected_requests']==1
        assert summary['first_request_failure']=='web-session-required';assert summary['last_recorded_configuration']['event_id']=='cli-http97'
        assert (root/'.swarm/engine-settings.json').read_bytes()==saved
    except BaseException as error:primary=error
    finish(server,primary);shutil.rmtree(root)
    return dict(case='normal',status='PASS',scope='actual normal CLI/protected HTTP/owner stream/SIGINT; no UI or durable journal')


def broken_stdout(binary,output):
    """Close real stdout only after valid witnesses; verify post-commit owner error,
    safe stderr and explicit same-event replay after public readback/restart.
    """
    root=owned_root(binary);server=Server(binary,root,output,'broken-stdout');primary=None
    try:
        csrf,change=server.prepare('cli-broken97')
        server.process.stdout.close();server.stdout_open=False
        connection=http.client.HTTPConnection(server.host,server.port,timeout=None)
        try:
            connection.request('POST','/api/v1/native-settings/apply',body=json.dumps(change).encode(),headers={'Cookie':server.cookie,'Origin':'http://'+server.address,'X-Swarm-CSRF':csrf,'Content-Type':'application/json'})
            response=connection.getresponse();raw=response.read();assert response.status!=200
        except (http.client.RemoteDisconnected,ConnectionResetError,BrokenPipeError):pass
        finally:connection.close()
        server.stderr=server.process.stderr.read();assert server.process.wait()==2
        diagnostic=json.loads(server.stderr);assert diagnostic['failure']['code']=='web-outcome-observation-failed'
        assert diagnostic['configuration_may_be_recorded'] is True and diagnostic['web_session_may_be_recorded'] is True
        assert diagnostic['recorded_configuration']['event_id']=='cli-broken97' and diagnostic['recorded_configuration']['event_revision']=='2'
        assert any('BrokenPipe' in item for item in diagnostic['failure']['causes'])
        assert diagnostic['pending_outcomes'][0]['configuration_may_be_recorded'] is True
        shown=cli(binary,root,['engine-settings','show']);assert shown['revision']=='2' and shown['values']['review_delivery_max_bytes']=='3'
        assert sum(entry['change']['event_id']=='cli-broken97' for entry in shown['history'])==1
        saved=(root/'.swarm/engine-settings.json').read_bytes()
    except BaseException as error:primary=error
    finish(server,primary)
    recovered=Server(binary,root,output,'recovered');primary=None
    try:
        session=recovered.request('/api/v1/native-session');csrf=session['csrf_token']
        replay=recovered.request('/api/v1/native-settings/apply',change,csrf);assert replay['replayed'] is True
        assert (root/'.swarm/engine-settings.json').read_bytes()==saved
        summary=recovered.stop();assert summary['configuration_outcomes']==1 and summary['completed_jobs']==2
    except BaseException as error:primary=error
    finish(recovered,primary);shutil.rmtree(root)
    return dict(case='broken-stdout',status='PASS',scope='real EPIPE after actual TCP commit, native stderr preserves state; explicit replay without duplication')


def main():
    """Preserve exact driver/helper/binary hashes and original failures; no source mutation."""
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();binary=args.binary.resolve();output=args.output.resolve();output.mkdir(parents=True,exist_ok=False)
    before=retained.digest(binary.read_bytes());source=Path(__file__).read_bytes();helper=Path(retained.__file__).read_bytes()
    retained.publish(output/'driver.py',source);retained.publish(output/'rust_agent_model_route.py',helper);retained.publish(output/'inputs.json',dict(binary=str(binary),binary_sha256=before,source_sha256=retained.digest(source),helper_sha256=retained.digest(helper),global_watchdog=None,scope='owned normal CLI HTTP, no real provider/UI/Go restart'))
    checks=[]
    try:
        for scenario in [normal,broken_stdout]:
            checks.append(scenario(binary,output));print('PASS native-http-cli-'+checks[-1]['case'],flush=True)
        assert retained.digest(binary.read_bytes())==before
        retained.publish(output/'result.json',dict(status='PASS',checks=checks,binary_unchanged=True,global_watchdog=None,private_tokens_redacted=True))
    except BaseException as error:
        retained.publish(output/'failure.json',dict(status='FAIL',type=type(error).__name__,message=str(error),traceback=traceback.format_exc(),checks=checks));raise


if __name__=='__main__':main()


def assert_head_wire(host, port, path, cookie, *, content_type='application/json', observe=None):
    """Observe raw HEAD framing after a valid owned GET, without client suppression.

    Connection-close EOF proves absence of body. Reject contradictory keep-alive,
    invalid MIME/status and any body immediately; never buffer an unbounded body.
    Header/EOF progress may be recorded safely through observe; a peer promising
    close but never closing still requires diagnosis, not an invented watchdog.
    TCP65536 is a buffer, not response policy. Socket/protocol/observer errors
    propagate. Never emit request/cookie/body text or infer application acceptance.
    """
    import socket
    if any(c in str(value) for value in [host,port,path,cookie] for c in ['\r','\n']):raise ValueError('head-wire-header-injection')
    request=('HEAD '+path+' HTTP/1.1\r\nHost: '+host+':'+str(port)+'\r\nCookie: '+cookie+'\r\nConnection: close\r\n\r\n').encode('ascii')
    with socket.create_connection((host,port),timeout=None) as connection:
        connection.sendall(request);prefix=bytearray()
        while b'\r\n\r\n' not in prefix:
            chunk=connection.recv(65536)
            if not chunk:raise AssertionError('HEAD response header incomplete')
            prefix.extend(chunk)
        header,separator,body=bytes(prefix).partition(b'\r\n\r\n');lines=header.split(b'\r\n');status=int(lines[0].split(b' ')[1]);headers={}
        for line in lines[1:]:
            name,value=line.split(b':',1);key=name.lower()
            if key in headers and key in [b'content-type',b'connection']:raise AssertionError('HEAD ambiguous response header')
            headers[key]=value.strip()
        if status!=200:raise AssertionError('HEAD positive control status '+str(status))
        mime=headers.get(b'content-type',b'').decode('ascii')
        if mime.split(';',1)[0].strip().lower()!=content_type.split(';',1)[0].strip().lower():raise AssertionError('HEAD MIME mismatch')
        if b'keep-alive' in [token.strip().lower() for token in headers.get(b'connection',b'').split(b',')]:raise AssertionError('HEAD server ignored Connection: close')
        if body:raise AssertionError('HEAD unexpectedly published body ('+str(len(body))+' bytes)')
        if observe:observe(dict(phase='headers-received-awaiting-eof',header_bytes=len(header),status=status))
        if connection.recv(1):raise AssertionError('HEAD unexpectedly published body after header')
    if observe:observe(dict(phase='eof-no-body',header_bytes=len(header),status=status))
    return dict(status=status,body_bytes=0,wire_sha256=hashlib.sha256(bytes(prefix)).hexdigest(),content_type=mime,contract='actual wire EOF, no HTTP client HEAD suppression')
