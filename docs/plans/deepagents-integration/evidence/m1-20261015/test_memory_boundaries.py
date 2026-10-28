"""Additional audit checks justified by backend and permission boundaries."""
from concurrent.futures import ThreadPoolExecutor
import threading

from deepagents import create_deep_agent
from deepagents.backends import FilesystemBackend, StoreBackend
from deepagents.backends.utils import create_file_data
from deepagents.middleware.filesystem import FilesystemPermission
from langchain_core.messages import AIMessage, HumanMessage, ToolMessage
from langgraph.checkpoint.memory import InMemorySaver
from langgraph.store.memory import InMemoryStore
from test_memory_paths import fake, invoke, system_text


def test_state_backend_memory_is_not_automatically_shared_between_threads():
    model=fake(AIMessage(content='one'),AIMessage(content='same'),AIMessage(content='new'))
    graph=create_deep_agent(model=model,memory=['/AGENTS.md'],checkpointer=InMemorySaver())
    graph.invoke({'messages':[HumanMessage(content='first')],'files':{'/AGENTS.md':create_file_data('thread-private-marker')}},config={'configurable':{'thread_id':'state-one'}})
    assert 'thread-private-marker' in system_text(model)
    invoke(graph,'state-one')
    assert 'thread-private-marker' in system_text(model)
    invoke(graph,'state-two')
    assert 'thread-private-marker' not in system_text(model)
    print('OBSERVED: default StateBackend memory stayed in its thread; shared memory required another backend/configuration')


def test_file_permission_denial_does_not_filter_configured_memory_injection(tmp_path):
    backend=FilesystemBackend(root_dir=str(tmp_path),virtual_mode=True)
    assert backend.write('/AGENTS.md','configured-source-marker').error is None
    model=fake(AIMessage(content='',tool_calls=[{'name':'read_file','args':{'file_path':'/AGENTS.md'},'id':'read-denied','type':'tool_call'}]),AIMessage(content='done'))
    graph=create_deep_agent(model=model,memory=['/AGENTS.md'],backend=backend,permissions=[FilesystemPermission(operations=['read'],paths=['/AGENTS.md'],mode='deny')])
    result=invoke(graph,'read-denied')
    assert 'configured-source-marker' in system_text(model,0)
    assert any(isinstance(m,ToolMessage) and m.status=='error' and 'permission denied' in str(m.content) for m in result['messages'])
    print('OBSERVED: explicit memory source reached system prompt although mediated read_file was denied; source admission needs its own guard')


class InterleavedStore(InMemoryStore):
    def __init__(self):
        super().__init__()
        self.audit_interleave=False
        self.audit_barrier=threading.Barrier(2)

    def get(self,namespace,key,**kwargs):
        item=super().get(namespace,key,**kwargs)
        if self.audit_interleave and key=='/AGENTS.md':
            self.audit_barrier.wait(timeout=5)
        return item


def test_shared_store_read_modify_write_can_lose_concurrent_edits():
    store=InterleavedStore()
    first=StoreBackend(store=store,namespace=lambda _rt:('shared-audit',))
    second=StoreBackend(store=store,namespace=lambda _rt:('shared-audit',))
    assert first.write('/AGENTS.md','first=0\nsecond=0').error is None
    store.audit_interleave=True
    with ThreadPoolExecutor(max_workers=2) as pool:
        futures=[pool.submit(first.edit,'/AGENTS.md','first=0','first=1'),pool.submit(second.edit,'/AGENTS.md','second=0','second=1')]
        results=[f.result(timeout=7) for f in futures]
    store.audit_interleave=False
    assert all(r.error is None for r in results)
    text=first.download_files(['/AGENTS.md'])[0].content.decode()
    assert ('first=1' in text)+('second=1' in text)==1
    print('OBSERVED: independent shared-backend edits both reported success but only one change survived; caller serialization is required')
