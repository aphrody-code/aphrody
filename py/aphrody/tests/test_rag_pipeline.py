# SPDX-License-Identifier: Apache-2.0
"""Tests for the new integrated RAG features (layout chunking, RAPTOR, and GraphRAG)."""

import os
import tempfile

import networkx as nx
import pytest
from aphrody.rag import (
    GraphExtractor,
    LayoutChunker,
    RAGPipeline,
    RecursiveAbstractiveProcessing4TreeOrganizedRetrieval,
)


def test_layout_chunker():
    chunker = LayoutChunker(chunk_token_num=100, overlap_token_num=10)

    mock_md = """# Section 1
This is a simple paragraph with some text.

- List item 1
- List item 2

```python
print("Hello World")
```

| Header 1 | Header 2 |
|----------|----------|
| Row 1    | Val 1    |
"""

    elements = chunker.extract_elements(mock_md)
    assert len(elements) >= 5
    assert any(e.type == "heading" for e in elements)
    assert any(e.type == "list" for e in elements)
    assert any(e.type == "code_block" for e in elements)
    assert any(e.type == "table" for e in elements)

    chunks = chunker.chunk_text(mock_md)
    assert len(chunks) > 0
    assert chunks[0]["heading_context"] == "Section 1"


@pytest.mark.asyncio
async def test_raptor_builder():
    async def mock_chat(system, history, gen_conf):
        return "Summary of cluster content"

    def mock_embed(texts):
        return [[0.1] * 128 for _ in range(len(texts))]

    builder = RecursiveAbstractiveProcessing4TreeOrganizedRetrieval(
        max_cluster=3,
        llm_chat_fn=mock_chat,
        embed_fn=mock_embed,
        prompt="{cluster_content}",
        tree_builder="raptor",
        clustering_method="gmm",
    )

    chunks = [
        ("Text segment one", [0.1] * 128),
        ("Text segment two", [0.2] * 128),
        ("Text segment three", [0.1] * 128),
        ("Text segment four", [0.3] * 128),
    ]

    final_chunks, layers = await builder(chunks)
    assert len(final_chunks) > 4
    assert len(layers) > 1


@pytest.mark.asyncio
async def test_graph_extractor():
    async def mock_chat(system, history, gen_conf):
        return (
            '("entity"<|>"ALEX"<|>"PERSON"<|>"Alex is a character.")##'
            '("entity"<|>"TAYLOR"<|>"PERSON"<|>"Taylor is an engineer.")##'
            '("relationship"<|>"ALEX"<|>"TAYLOR"<|>"Alex knows Taylor."<|>8)<||COMPLETE|>'
        )

    extractor = GraphExtractor(
        llm_chat_fn=mock_chat, entity_types=["PERSON"], language="English"
    )

    chunks = ["Alex met Taylor in the office."]

    graph = await extractor.extract_graph(chunks, doc_id="test_doc")
    assert isinstance(graph, nx.Graph)
    assert "ALEX" in graph.nodes
    assert "TAYLOR" in graph.nodes
    assert graph.has_edge("ALEX", "TAYLOR")


@pytest.mark.asyncio
async def test_rag_pipeline():
    async def mock_chat(system, history, gen_conf):
        if "summary" in system.lower() or "summarize" in system.lower():
            return "Unified summary text"
        return (
            '("entity"<|>"BOB"<|>"PERSON"<|>"Bob is a developer.")##'
            '("relationship"<|>"BOB"<|>"PROJECT"<|>"Bob built the project."<|>9)<||COMPLETE|>'
        )

    def mock_embed(texts):
        return [[0.05] * 128 for _ in range(len(texts))]

    pipeline = RAGPipeline(
        llm_chat_fn=mock_chat,
        embed_fn=mock_embed,
        graph_entity_types=["PERSON", "PROJECT"],
    )

    with tempfile.NamedTemporaryFile(suffix=".md", mode="w", delete=False) as f:
        f.write("# Project Alpha\nBob is the lead developer of Project Alpha.")
        temp_path = f.name

    try:
        pipeline.chunker.parse_document = lambda path, binary=None: (
            "# Project Alpha\nBob is the lead developer of Project Alpha."
        )

        result = await pipeline.process_document(
            temp_path, doc_id="test_pipeline"
        )
        assert "chunks" in result
        assert "raptor_layers" in result
        assert "graph" in result

        assert len(result["chunks"]) > 0
        assert isinstance(result["graph"], nx.Graph)
    finally:
        if os.path.exists(temp_path):
            os.remove(temp_path)
