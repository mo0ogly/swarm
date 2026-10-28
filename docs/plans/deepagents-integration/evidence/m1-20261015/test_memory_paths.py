"""Audit probes against the pinned SDK; fake models, no provider calls."""
from dataclasses import dataclass
import json
from pathlib import Path
import subprocess
import sys

from deepagents import create_deep_agent
from deepagents.backends import CompositeBackend, FilesystemBackend, StateBackend, StoreBackend
from deepagents.middleware.filesystem import FilesystemPermission
from langchain_core.messages import AIMessage, HumanMessage, SystemMessage, ToolMessage
from langgraph.checkpoint.memory import InMemorySaver
from langgraph.store.memory import InMemoryStore
from tests.unit_tests.chat_model import GenericFakeChatModel


def system_text(model, index=-1):
    return '\n'.join(str(m.content) for m in model.call_history[index]['messages'] if isinstance(m, SystemMessage))


def fake(*responses):
    return GenericFakeChatModel(messages=iter(responses))


def edit(path, old, new):
    return AIMessage(content='', tool_calls=[{'name':'edit_file','args':{'file_path':path,'old_string':old,'new_string':new},'id':'audit-edit','type':'tool_call'}])


def invoke(graph, thread, context=None):
    return graph.invoke({'messages':[HumanMessage(content='Audit deterministic memory path')]},config={'configurable':{'thread_id':thread}},context=context)


def test_disk_edit_new_thread_and_same_thread_snapshot(tmp_path):
    backend=FilesystemBackend(root_dir=str(tmp_path),virtual_mode=True)
    assert backend.write('/AGENTS.md','audit-disk-before').error is None
    model=fake(edit('/AGENTS.md','audit-disk-before','audit-disk-after'),AIMessage(content='edited'),AIMessage(content='continued'),AIMessage(content='new thread'))
    graph=create_deep_agent(model=model,memory=['/AGENTS.md'],backend=backend,checkpointer=InMemorySaver())
    first=invoke(graph,'disk-original')
    assert any(isinstance(m,ToolMessage) and m.status=='success' for m in first['messages'])
    assert (tmp_path/'AGENTS.md').read_text()=='audit-disk-after'
    assert 'audit-disk-before' in system_text(model,0)
    assert 'audit-disk-before' in system_text(model,1)
    assert 'audit-disk-after' not in system_text(model,1)
    invoke(graph,'disk-original')
    assert 'audit-disk-before' in system_text(model)
    assert 'audit-disk-after' not in system_text(model)
    invoke(graph,'disk-new')
    assert 'audit-disk-after' in system_text(model)
    assert 'audit-disk-before' not in system_text(model)
    print('OBSERVED: disk updated by edit_file; loaded thread kept old system memory; new thread loaded new value')


@dataclass
class Identity:
    user: str


def store_graph(*responses):
    store=InMemoryStore()
    for user,value in [('alice','alice-before'),('bob','bob-only')]:
        be=StoreBackend(store=store,namespace=lambda _rt,u=user:('audit',u))
        assert be.write('/AGENTS.md',value).error is None
    shared=StoreBackend(store=store,namespace=lambda rt:('audit',rt.context.user))
    model=fake(*responses)
    graph=create_deep_agent(model=model,memory=['/memories/AGENTS.md'],backend=CompositeBackend(default=StateBackend(),routes={'/memories/':shared}),context_schema=Identity,checkpointer=InMemorySaver())
    return graph,model,store


def test_store_write_shared_across_threads_and_namespaces_isolated():
    graph,model,store=store_graph(edit('/memories/AGENTS.md','alice-before','alice-after'),AIMessage(content='edited'),AIMessage(content='bob'),AIMessage(content='alice again'))
    invoke(graph,'alice-one',Identity('alice'))
    assert store.get(('audit','alice'),'/AGENTS.md').value['content']=='alice-after'
    invoke(graph,'bob-one',Identity('bob'))
    assert 'bob-only' in system_text(model)
    assert 'alice-' not in system_text(model)
    invoke(graph,'alice-two',Identity('alice'))
    assert 'alice-after' in system_text(model)
    assert 'bob-only' not in system_text(model)
    print('OBSERVED: shared store persisted edit across thread IDs; configured namespaces separated Alice and Bob')


def test_reusing_thread_with_new_identity_keeps_prior_memory_snapshot():
    graph,model,_=store_graph(AIMessage(content='alice'),AIMessage(content='bob reusing thread'))
    invoke(graph,'reused-thread',Identity('alice'))
    assert 'alice-before' in system_text(model)
    invoke(graph,'reused-thread',Identity('bob'))
    assert 'alice-before' in system_text(model)
    assert 'bob-only' not in system_text(model)
    print('OBSERVED: changing runtime namespace on the same thread did not refresh cached memory; caller must bind thread ownership')


def test_denied_memory_edit_does_not_mutate_file(tmp_path):
    backend=FilesystemBackend(root_dir=str(tmp_path),virtual_mode=True)
    assert backend.write('/AGENTS.md','deny-before').error is None
    model=fake(edit('/AGENTS.md','deny-before','deny-after'),AIMessage(content='refused'))
    graph=create_deep_agent(model=model,memory=['/AGENTS.md'],backend=backend,permissions=[FilesystemPermission(operations=['write'],paths=['/AGENTS.md'],mode='deny')])
    result=invoke(graph,'denied')
    assert any(isinstance(m,ToolMessage) and m.status=='error' and 'permission denied' in str(m.content) for m in result['messages'])
    assert (tmp_path/'AGENTS.md').read_text()=='deny-before'
    print('OBSERVED: mediated edit_file denial surfaced a tool error and left bytes unchanged')


def test_filesystem_memory_loaded_in_fresh_process(tmp_path):
    backend=FilesystemBackend(root_dir=str(tmp_path),virtual_mode=True)
    assert backend.write('/AGENTS.md','survives-fresh-python').error is None
    code="""
import json,sys
from deepagents import create_deep_agent
from deepagents.backends import FilesystemBackend
from langchain_core.messages import AIMessage,HumanMessage,SystemMessage
from tests.unit_tests.chat_model import GenericFakeChatModel
model=GenericFakeChatModel(messages=iter([AIMessage(content='done')]))
graph=create_deep_agent(model=model,memory=['/AGENTS.md'],backend=FilesystemBackend(root_dir=sys.argv[1],virtual_mode=True))
graph.invoke({'messages':[HumanMessage(content='Read memory')]})
text='\\n'.join(str(m.content) for m in model.call_history[0]['messages'] if isinstance(m,SystemMessage))
print(json.dumps({'marker_received':'survives-fresh-python' in text}))
"""
    result=subprocess.run([sys.executable,'-c',code,str(tmp_path)],capture_output=True,text=True,check=True)
    assert json.loads(result.stdout)['marker_received'] is True
    print('OBSERVED: new Python process read persisted filesystem memory into actual fake-model request')
