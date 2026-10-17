"""Source-bound M2 probes. Fake LLM; template factory is a declared spy."""
from pathlib import Path
from types import ModuleType, SimpleNamespace
import importlib
import sys
import uuid

import pytest
from langchain_core.messages import AIMessage, HumanMessage, SystemMessage
from langgraph.checkpoint.memory import InMemorySaver
from langgraph.store.memory import InMemoryStore
from tests.unit_tests.chat_model import GenericFakeChatModel

sys.path.insert(0,'/tmp/swarm-memory-agent-AFnHs8/src')
sys.path.insert(0,'/tmp/swarm-memory-template-wsrdkl/src')


def system_text(model,index=-1):
    return '\n'.join(str(m.content) for m in model.call_history[index]['messages'] if isinstance(m,SystemMessage))


@pytest.mark.asyncio
async def test_actual_memory_agent_inserts_retrieves_updates_and_scopes(monkeypatch):
    mod=importlib.import_module('memory_agent.graph')
    from memory_agent.context import Context
    key=str(uuid.uuid4())
    def upsert(content,call):
        return AIMessage(content='',tool_calls=[{'name':'upsert_memory','id':call,'type':'tool_call','args':{'content':content,'context':'audit context','memory_id':key}}])
    model=GenericFakeChatModel(messages=iter([upsert('remember-before','insert'),AIMessage(content='saved'),AIMessage(content='new thread'),upsert('remember-after','update'),AIMessage(content='updated'),AIMessage(content='other user')]))
    monkeypatch.setattr(mod.utils,'load_chat_model',lambda _:model)
    store=InMemoryStore()
    graph=mod.builder.compile(store=store,checkpointer=InMemorySaver())
    async def call(thread,user):
        return await graph.ainvoke({'messages':[HumanMessage(content='audit memory')]},{'configurable':{'thread_id':thread}},context=Context(user_id=user))
    await call('thread-a','alice')
    assert 'remember-before' in system_text(model,1)
    await call('thread-b','alice')
    assert 'remember-before' in system_text(model)
    await call('thread-c','alice')
    items=store.search(('memories','alice'))
    assert len(items)==1 and items[0].key==key and items[0].value['content']=='remember-after'
    assert 'remember-after' in system_text(model)
    await call('thread-d','bob')
    assert 'remember-' not in system_text(model)
    print('OBSERVED: actual memory-agent graph stored, recalled next model call and next thread, updated existing ID and separated configured users')


@pytest.mark.asyncio
async def test_memory_agent_duplicate_without_id_is_not_deduplicated():
    from memory_agent.tools import upsert_memory
    store=InMemoryStore()
    for _ in range(2):
        await upsert_memory('identical fact','identical context',user_id='alice',store=store)
    assert len(store.search(('memories','alice')))==2
    print('OBSERVED: identical facts without memory_id produced two records; semantic deduplication is delegated to model/tool use')


@pytest.mark.asyncio
async def test_actual_memory_agent_does_not_validate_uuid_tool_argument(monkeypatch):
    mod=importlib.import_module('memory_agent.graph')
    from memory_agent.context import Context
    model=GenericFakeChatModel(messages=iter([AIMessage(content='',tool_calls=[{'name':'upsert_memory','id':'invalid-id-call','type':'tool_call','args':{'content':'fact','context':'audit','memory_id':'not-a-uuid'}}]),AIMessage(content='done')]))
    monkeypatch.setattr(mod.utils,'load_chat_model',lambda _:model)
    store=InMemoryStore()
    graph=mod.builder.compile(store=store)
    await graph.ainvoke({'messages':[HumanMessage(content='audit invalid model input')]},context=Context(user_id='alice'))
    assert store.get(('memories','alice'),'not-a-uuid') is not None
    print('OBSERVED: manual tool dispatch accepted malformed UUID despite declared tool type; source annotations are not engine validation')


@pytest.fixture
def template(monkeypatch):
    stub=ModuleType('langmem')
    stub.create_memory_store_manager=lambda *args,**kwargs:object()
    monkeypatch.setitem(sys.modules,'langmem',stub)
    mod=importlib.import_module('memory_graph.graph')
    mod.get_store_manager.cache_clear()
    yield mod
    mod.get_store_manager.cache_clear()


def config(model='audit-model',instructions='audit instructions'):
    return SimpleNamespace(model=model,memory_types=[SimpleNamespace(name='User',update_mode='patch',parameters={'type':'object','properties':{'required_fact':{'type':'string'}}},description='required fact schema',system_prompt=instructions)])


def test_template_declared_schema_is_not_forwarded_to_manager_factory(template,monkeypatch):
    calls=[]
    monkeypatch.setattr(template.configuration.Configuration,'from_context',staticmethod(lambda:config()))
    monkeypatch.setattr(template,'create_memory_store_manager',lambda *a,**kw:calls.append((a,kw)) or object())
    template.get_store_manager('User')
    args,kwargs=calls[0]
    assert kwargs['namespace']==('memories','{user_id}','User')
    assert 'schemas' not in kwargs and 'parameters' not in kwargs and 'description' not in kwargs
    print('OBSERVED: actual template adapter forwarded namespace/mode/instructions but did not forward configured schema parameters; factory was a spy')


def test_template_cache_uses_type_name_despite_changed_model_and_instructions(template,monkeypatch):
    current=[config('model-one','instructions-one')]
    calls=[]
    monkeypatch.setattr(template.configuration.Configuration,'from_context',staticmethod(lambda:current[0]))
    monkeypatch.setattr(template,'create_memory_store_manager',lambda *a,**kw:calls.append((a,kw)) or object())
    first=template.get_store_manager('User')
    current[0]=config('model-two','instructions-two')
    second=template.get_store_manager('User')
    assert first is second and len(calls)==1
    assert calls[0][0]==('model-one',) and calls[0][1]['instructions']=='instructions-one'
    print('OBSERVED: changing type configuration reused cached first manager; cache key excludes model/instructions/schema')


@pytest.mark.asyncio
async def test_template_chatbot_reads_another_namespace_prefix(monkeypatch):
    mod=importlib.import_module('chatbot.graph')
    model=GenericFakeChatModel(messages=iter([AIMessage(content='first'),AIMessage(content='second')]))
    store=InMemoryStore()
    store.put(('memories','alice','User'),'one',{'content':'producer-prefix-marker'})
    monkeypatch.setattr(mod,'get_store',lambda:store)
    monkeypatch.setattr(mod,'llm',model)
    monkeypatch.setattr(mod.ChatConfigurable,'from_context',staticmethod(lambda:SimpleNamespace(user_id='alice',model='fake',system_prompt='{user_info} {time}')))
    state=mod.ChatState(messages=[HumanMessage(content='audit')])
    await mod.bot(state)
    assert 'producer-prefix-marker' not in system_text(model)
    store.put(('alice','User'),'two',{'content':'consumer-prefix-marker'})
    await mod.bot(state)
    assert 'consumer-prefix-marker' in system_text(model)
    print('OBSERVED: actual chatbot read user-first prefix; record at manager-declared memories-first prefix was not retrieved; producer manager itself not executed')


def test_template_integration_test_imports_a_builder_absent_from_current_module(template):
    assert not hasattr(template,'builder')
    source=Path('/tmp/swarm-memory-template-wsrdkl/tests/integration_tests/test_graph.py').read_text()
    assert 'from memory_graph.graph import builder' in source
    with pytest.raises(ImportError):
        exec('from memory_graph.graph import builder',{})
    print('OBSERVED: current integration test cannot import its required builder; no paid integration test was launched')
