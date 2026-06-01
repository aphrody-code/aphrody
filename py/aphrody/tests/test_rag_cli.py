# SPDX-License-Identifier: Apache-2.0
"""Tests for the new integrated RAG CLI commands."""

from unittest import mock

from aphrody.cli.rag import RAGCommands


class MockCandidate:
    content = mock.Mock()
    text = "Mock LLM output"


class MockResponse:
    candidates = (MockCandidate(),)
    text = (
        '("entity"<|>"ALEX"<|>"PERSON"<|>"Alex is a character.")##'
        '("entity"<|>"TAYLOR"<|>"PERSON"<|>"Taylor is an engineer.")##'
        '("relationship"<|>"ALEX"<|>"TAYLOR"<|>"Alex knows Taylor."<|>8)<||COMPLETE|>'
    )


def test_rag_chunk(tmp_path):
    doc_path = tmp_path / "test.md"
    doc_path.write_text("# Test Title\nThis is a layout block.")

    cli = RAGCommands()
    with mock.patch("aphrody.cli.rag._emit") as mock_emit:
        cli.chunk(str(doc_path))
        mock_emit.assert_called_once()
        args, _ = mock_emit.call_args
        res = args[0]
        assert "chunks" in res
        assert len(res["chunks"]) > 0


@mock.patch("aphrody.cli.rag._make_llm_chat")
@mock.patch("aphrody.cli.rag._make_embed_fn")
def test_rag_raptor(mock_make_embed, mock_make_chat, tmp_path):
    doc_path = tmp_path / "test.md"
    doc_path.write_text("# Test Title\nThis is a layout block.")

    mock_chat = mock.AsyncMock(return_value="Summary of cluster")
    mock_make_chat.return_value = mock_chat

    def mock_embed(texts):
        return [[0.1] * 128 for _ in range(len(texts))]

    mock_make_embed.return_value = mock_embed

    cli = RAGCommands()
    with mock.patch("aphrody.cli.rag._emit") as mock_emit:
        cli.raptor(str(doc_path), max_cluster=2)
        mock_emit.assert_called_once()
        args, _ = mock_emit.call_args
        res = args[0]
        assert "summaries" in res
        assert "layers_count" in res


@mock.patch("aphrody.cli.rag._make_llm_chat")
def test_rag_graph(mock_make_chat, tmp_path):
    doc_path = tmp_path / "test.md"
    doc_path.write_text("# Test Title\nThis is a layout block.")

    mock_chat = mock.AsyncMock(
        return_value=(
            '("entity"<|>"ALEX"<|>"PERSON"<|>"Alex is a character.")##'
            '("relationship"<|>"ALEX"<|>"TAYLOR"<|>"Alex knows Taylor."<|>8)<||COMPLETE|>'
        )
    )
    mock_make_chat.return_value = mock_chat

    cli = RAGCommands()
    with mock.patch("aphrody.cli.rag._emit") as mock_emit:
        cli.graph(str(doc_path), entity_types="PERSON")
        mock_emit.assert_called_once()
        args, _ = mock_emit.call_args
        res = args[0]
        assert "graph" in res
        assert "nodes" in res["graph"]


@mock.patch("aphrody.cli.rag._make_llm_chat")
@mock.patch("aphrody.cli.rag._make_embed_fn")
def test_rag_process(mock_make_embed, mock_make_chat, tmp_path):
    doc_path = tmp_path / "test.md"
    doc_path.write_text("# Test Title\nThis is a layout block.")

    mock_chat = mock.AsyncMock(
        return_value=(
            '("entity"<|>"ALEX"<|>"PERSON"<|>"Alex is a character.")##'
            '("relationship"<|>"ALEX"<|>"TAYLOR"<|>"Alex knows Taylor."<|>8)<||COMPLETE|>'
        )
    )
    mock_make_chat.return_value = mock_chat

    def mock_embed(texts):
        return [[0.1] * 128 for _ in range(len(texts))]

    mock_make_embed.return_value = mock_embed

    cli = RAGCommands()
    out_dir = tmp_path / "out"
    with mock.patch("aphrody.cli.rag._emit") as mock_emit:
        cli.process(str(doc_path), out_dir=str(out_dir))
        mock_emit.assert_called_once()
        args, _ = mock_emit.call_args
        res = args[0]
        assert res["status"] == "success"

        assert (out_dir / "chunks.json").exists()
        assert (out_dir / "graph.json").exists()
        assert (out_dir / "graph.graphml").exists()
