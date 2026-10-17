"""Offline probes of LangGraph primitives, not Swarm acceptance tests."""

from langchain_core.messages import AIMessage, HumanMessage, RemoveMessage
from langchain_core.messages.utils import trim_messages
from langgraph.checkpoint.memory import InMemorySaver
from langgraph.graph import END, START, MessagesState, StateGraph
from langgraph.store.memory import InMemoryStore


def compile_graph(saver, node, store=None):
    builder = StateGraph(MessagesState)
    builder.add_node("observe", node)
    builder.add_edge(START, "observe")
    builder.add_edge("observe", END)
    return builder.compile(checkpointer=saver, store=store)


def test_trim_only_changes_the_view_given_to_the_node_consumer():
    received = []

    def observe(state):
        selected = trim_messages(
            state["messages"],
            max_tokens=2,
            token_counter=len,
            strategy="last",
            start_on="human",
        )
        received.append([message.id for message in selected])
        return {"messages": []}

    saver = InMemorySaver()
    graph = compile_graph(saver, observe)
    config = {"configurable": {"thread_id": "trim-fixture"}}
    messages = [
        HumanMessage(content="earlier input", id="h1"),
        AIMessage(content="earlier output", id="a1"),
        HumanMessage(content="current input", id="h2"),
        AIMessage(content="current output", id="a2"),
    ]
    graph.invoke({"messages": messages}, config)
    assert received == [["h2", "a2"]]
    assert [message.id for message in graph.get_state(config).values["messages"]] == [
        "h1", "a1", "h2", "a2"
    ]


def test_removal_keeps_old_snapshot_and_thread_deletion_leaves_store():
    saver = InMemorySaver()
    store = InMemoryStore()
    store.put(("project-a", "experiences"), "lesson", {"text": "retained lesson"})
    graph = compile_graph(saver, lambda state: {"messages": []}, store)
    config = {"configurable": {"thread_id": "deletion-fixture"}}
    graph.invoke(
        {"messages": [HumanMessage(content="original context", id="source")]},
        config,
    )
    historical = graph.get_state(config).config
    graph.invoke({"messages": [RemoveMessage(id="source")]}, config)
    assert graph.get_state(config).values["messages"] == []
    assert graph.get_state(historical).values["messages"][0].content == "original context"
    assert any(
        state.values.get("messages")
        for state in graph.get_state_history(config)
    )
    saver.delete_thread("deletion-fixture")
    assert list(graph.get_state_history(config)) == []
    assert saver.get_tuple(historical) is None
    assert store.get(("project-a", "experiences"), "lesson").value == {
        "text": "retained lesson"
    }
