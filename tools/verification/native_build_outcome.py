"""Retain native subprocess results before post-execution admission checks.

Owned stdout/stderr and immutable process/admission receipts are separate phases.
Any process, close, hashing, postcheck or receipt failure remains unqualified and
keeps all causes; no retry, timeout, provider or installation authority is added.
"""
import json,subprocess,time


def describe(error,seen=None):
    """Render safe tool failures and nested causes, avoiding exception cycles.

    Callers pass build/IO/admission exceptions, never credentials/environment.
    Raw process channels stay in their private files, represented here by hashes.
    """
    seen=set() if seen is None else seen
    if id(error) in seen:return {'type':type(error).__name__,'cycle':True}
    seen.add(id(error));value=dict(type=type(error).__name__,message=str(error))
    if getattr(error,'exceptions',None):value['exceptions']=[describe(e,seen) for e in error.exceptions]
    if error.__cause__ is not None:value['cause']=describe(error.__cause__,seen)
    elif error.__context__ is not None and not error.__suppress_context__:value['context']=describe(error.__context__,seen)
    return value


def failure_document(error):
    """Expose the retained process/postcheck result to the final build consumer.

    The optional outcome is evidence of observed completion, not proof of no
    side effect or successful receipt publication. Original causes remain visible.
    """
    value=dict(type=type(error).__name__,reason=str(error),causes=describe(error))
    if hasattr(error,'native_outcome'):value['native_outcome']=error.native_outcome
    return value


def execute(argv,*,out,name,admission,publish,digest,failure_code,**kwargs):
    """Execute one owned process with complete outcome/cleanup retention.

    Validate admission before spawn, close channels without masking completion,
    then attempt the original exit/signal receipt BEFORE postcheck. Independently
    retain admission outcome in a second receipt. Publication failures and every
    other issue propagate together with structured details for failure.json.
    A failed receipt is not retried; the distinct postcheck receipt records new
    observations. Missing/failed channels are explicit, never successful empties.
    """
    errors=[];streams={};result=None;attempted=False;started=time.time()
    try:
        streams['stdout']=(out/(name+'.stdout')).open('xb',buffering=0)
        streams['stderr']=(out/(name+'.stderr')).open('xb',buffering=0)
        admission.check();attempted=True
        result=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=streams['stdout'],stderr=streams['stderr'],**kwargs)
    except BaseException as error:errors.append(error)
    finally:
        for stream in streams.values():
            try:stream.close()
            except BaseException as error:errors.append(error)
    outcome=dict(returncode=None if result is None else result.returncode,
        signal=-result.returncode if result is not None and result.returncode<0 else None,
        execution_attempted=attempted,started_at_unix=started,finished_at_unix=time.time())
    if result is not None and result.returncode!=0:errors.insert(0,ValueError(failure_code))
    for channel in ('stdout','stderr'):
        try:outcome[channel+'_sha256']=digest(out/(name+'.'+channel))
        except BaseException as error:outcome[channel+'_sha256']=None;errors.append(error)
    outcome['issues']=[describe(e) for e in errors]
    publication={'ok':True}
    try:publish(out/(name+'-result.json'),json.dumps(outcome,indent=2)+'\n')
    except BaseException as error:publication={'ok':False,'error':describe(error)};errors.append(error)
    after={'checked':attempted,'ok':None}
    if attempted:
        try:after['facts']=admission.check();after['ok']=True
        except BaseException as error:after['ok']=False;after['error']=describe(error);errors.append(error)
    else:after['reason']='execution-not-started'
    details=dict(subprocess=outcome,admission_after=after,receipt_publication=publication)
    try:publish(out/(name+'-admission-after.json'),json.dumps(details,indent=2)+'\n')
    except BaseException as error:details['diagnostic_publication']={'ok':False,'error':describe(error)};errors.append(error)
    if errors:
        failure=errors[0] if len(errors)==1 else BaseExceptionGroup(
            'Native process/admission evidence failed: '+'; '.join(str(e) for e in errors),errors)
        failure.native_outcome=details
        raise failure
    return result
