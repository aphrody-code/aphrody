# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""RAG CLI commands group for the aphrody CLI.

Provides layouts parsing, RAPTOR clustering, and GraphRAG extraction.
"""

from __future__ import annotations

import asyncio
import json
import logging
from collections.abc import Callable
from pathlib import Path
from typing import Any

from aphrody.cli.utils import _emit
from aphrody.vertex import resolve_location, resolve_project

logger = logging.getLogger(__name__)


def _make_llm_chat(
    model: str,
    project: str,
    location: str,
) -> Callable[[str, list[dict[str, str]], dict[str, Any]], Any]:
    """Create an asynchronous LLM chat function for the RAG pipeline."""
    from aphrody.vertex import GeminiVertex

    vertex_client = GeminiVertex(
        project=project, location=location, model=model
    )

    async def async_chat_fn(
        system: str,
        history: list[dict[str, str]],
        gen_conf: dict[str, Any],
    ) -> str:
        loop = asyncio.get_running_loop()

        def call_gemini() -> str:
            from google.genai import types as gx

            contents = []
            for msg in history:
                role = msg.get("role", "user")
                if role == "assistant":
                    role = "model"
                contents.append(
                    gx.Content(
                        role=role,
                        parts=[gx.Part.from_text(text=msg.get("content", ""))],
                    )
                )

            config = gx.GenerateContentConfig(
                system_instruction=system if system else None,
                temperature=gen_conf.get("temperature"),
                max_output_tokens=gen_conf.get("max_tokens")
                or gen_conf.get("max_output_tokens"),
            )

            res = vertex_client.client.models.generate_content(
                model=model,
                contents=contents,
                config=config,
            )
            return res.text or ""

        return await loop.run_in_executor(None, call_gemini)

    return async_chat_fn


def _make_embed_fn(
    embed_model: str,
    project: str,
    location: str,
) -> Callable[[list[str]], list[list[float]]]:
    """Create an embedding function using fastembed or Vertex AI."""
    if embed_model.startswith("text-embedding"):
        # Use Vertex AI Embeddings
        import google.auth
        from google import genai

        from aphrody.auth import credentials as _credentials

        try:
            credentials, _ = google.auth.default()
            if hasattr(credentials, "with_scopes"):
                credentials = credentials.with_scopes(
                    ["https://www.googleapis.com/auth/cloud-platform"]
                )
        except Exception:
            credentials = _credentials.load_google_credentials()

        client = genai.Client(
            vertexai=True,
            project=project,
            location=location,
            credentials=credentials,
        )

        def vertex_embed_fn(texts: list[str]) -> list[list[float]]:
            results = []
            batch_size = 100
            for i in range(0, len(texts), batch_size):
                batch = texts[i : i + batch_size]
                response = client.models.embed_content(
                    model=embed_model,
                    contents=batch,
                )
                for emb in response.embeddings:
                    results.append([float(val) for val in emb.values])
            return results

        return vertex_embed_fn

    else:
        # Use local fastembed
        try:
            from fastembed import TextEmbedding

            model = TextEmbedding(model_name=embed_model)

            def fastembed_fn(texts: list[str]) -> list[list[float]]:
                # fastembed returns generator
                return [list(map(float, vec)) for vec in model.embed(texts)]

            return fastembed_fn
        except ImportError:
            logger.warning(
                "fastembed is not installed. Falling back to a dummy embedding model."
            )

            def dummy_embed_fn(texts: list[str]) -> list[list[float]]:
                return [[0.0] * 384 for _ in range(len(texts))]

            return dummy_embed_fn


class RAGCommands:
    """``aphrody rag <action>`` — Layout, RAPTOR and GraphRAG operations."""

    def chunk(
        self,
        file: str,
        chunk_size: int = 512,
        overlap: int = 64,
    ) -> None:
        """Parse and chunk a document using layout-based structure.

        Args:
            file: The document file path to process.
            chunk_size: Maximum tokens per chunk.
            overlap: Overlap tokens between chunks.
        """
        from aphrody.rag.layout_chunker import LayoutChunker

        file_path = Path(file)
        if not file_path.exists():
            raise FileNotFoundError(f"File not found: {file}")

        chunker = LayoutChunker(
            chunk_token_num=chunk_size, overlap_token_num=overlap
        )
        chunks = chunker.chunk_document(str(file_path))

        _emit(
            {
                "file": str(file_path),
                "chunks_count": len(chunks),
                "chunks": chunks,
            }
        )

    def raptor(
        self,
        file: str,
        chunk_size: int = 512,
        overlap: int = 64,
        max_cluster: int = 5,
        tree_builder: str = "raptor",
        clustering: str = "gmm",
        model: str = "gemini-3.5-flash",
        embed_model: str = "BAAI/bge-small-en-v1.5",
        project: str | None = None,
        location: str | None = None,
    ) -> None:
        """Build a RAPTOR tree structure for the document.

        Args:
            file: Path to the document.
            chunk_size: Maximum tokens per chunk.
            overlap: Overlap tokens between chunks.
            max_cluster: Maximum number of clusters per layer.
            tree_builder: Tree builder to use ("raptor" or "psi").
            clustering: Clustering method ("gmm" or "ahc").
            model: Gemini model to use for summarization.
            embed_model: Embedding model name.
            project: Google Cloud project ID.
            location: Google Cloud region.
        """
        from aphrody.rag import (
            LayoutChunker,
            RecursiveAbstractiveProcessing4TreeOrganizedRetrieval,
        )

        file_path = Path(file)
        if not file_path.exists():
            raise FileNotFoundError(f"File not found: {file}")

        proj_id = resolve_project(project)
        loc_id = resolve_location(location)

        llm_chat_fn = _make_llm_chat(model, proj_id, loc_id)
        embed_fn = _make_embed_fn(embed_model, proj_id, loc_id)

        # 1. Chunk document
        chunker = LayoutChunker(
            chunk_token_num=chunk_size, overlap_token_num=overlap
        )
        chunks = chunker.chunk_document(str(file_path))
        texts = [c["content"] for c in chunks]

        # 2. Get embeddings
        embeddings = embed_fn(texts)
        input_chunks = list(zip(texts, embeddings))

        # 3. Build RAPTOR layers
        raptor_prompt = (
            "Write a concise summary of the following texts. "
            "Do not include introductory or concluding phrases.\n\n"
            "Texts:\n{cluster_content}\n\nSummary:"
        )

        builder = RecursiveAbstractiveProcessing4TreeOrganizedRetrieval(
            max_cluster=max_cluster,
            llm_chat_fn=llm_chat_fn,
            embed_fn=embed_fn,
            prompt=raptor_prompt,
            max_token=chunk_size,
            tree_builder=tree_builder,
            clustering_method=clustering,
        )

        async def run_pipeline():
            return await builder(input_chunks)

        final_chunks, layers = asyncio.run(run_pipeline())

        output_chunks = []
        for idx, (text, embd) in enumerate(final_chunks[len(texts) :]):
            output_chunks.append(
                {
                    "id": f"summary_{idx}",
                    "content": text,
                    "embedding": embd,
                }
            )

        _emit(
            {
                "file": str(file_path),
                "original_chunks_count": len(texts),
                "summary_chunks_count": len(output_chunks),
                "layers_count": len(layers),
                "summaries": output_chunks,
            }
        )

    def graph(
        self,
        file: str,
        chunk_size: int = 512,
        overlap: int = 64,
        entity_types: str | None = None,
        language: str = "English",
        model: str = "gemini-3.5-flash",
        project: str | None = None,
        location: str | None = None,
    ) -> None:
        """Extract a GraphRAG entity-relationship network from the document.

        Args:
            file: Path to the document.
            chunk_size: Maximum tokens per chunk.
            overlap: Overlap tokens between chunks.
            entity_types: Comma-separated list of entity types to extract.
            language: Target language for extraction.
            model: Gemini model to use for extraction.
            project: Google Cloud project ID.
            location: Google Cloud region.
        """
        from aphrody.rag import GraphExtractor, LayoutChunker

        file_path = Path(file)
        if not file_path.exists():
            raise FileNotFoundError(f"File not found: {file}")

        proj_id = resolve_project(project)
        loc_id = resolve_location(location)

        llm_chat_fn = _make_llm_chat(model, proj_id, loc_id)

        # 1. Chunk document
        chunker = LayoutChunker(
            chunk_token_num=chunk_size, overlap_token_num=overlap
        )
        chunks = chunker.chunk_document(str(file_path))
        texts = [c["content"] for c in chunks]

        # 2. Extract GraphRAG network
        types_list = (
            [t.strip() for t in entity_types.split(",")]
            if entity_types
            else None
        )
        extractor = GraphExtractor(
            llm_chat_fn=llm_chat_fn, entity_types=types_list, language=language
        )

        async def run_pipeline():
            return await extractor.extract_graph(texts, doc_id=file_path.stem)

        graph_obj = asyncio.run(run_pipeline())

        nodes = []
        for node, data in graph_obj.nodes(data=True):
            nodes.append({"id": node, **data})

        edges = []
        for u, v, data in graph_obj.edges(data=True):
            edges.append({"source": u, "target": v, **data})

        _emit(
            {
                "file": str(file_path),
                "nodes_count": len(nodes),
                "edges_count": len(edges),
                "graph": {
                    "nodes": nodes,
                    "edges": edges,
                },
            }
        )

    def process(
        self,
        file: str,
        out_dir: str | None = None,
        chunk_size: int = 512,
        overlap: int = 64,
        max_cluster: int = 5,
        tree_builder: str = "raptor",
        clustering: str = "gmm",
        entity_types: str | None = None,
        language: str = "English",
        model: str = "gemini-3.5-flash",
        embed_model: str = "BAAI/bge-small-en-v1.5",
        no_raptor: bool = False,
        no_graph: bool = False,
        project: str | None = None,
        location: str | None = None,
    ) -> None:
        """Run the end-to-end RAG pipeline and optionally write the outputs.

        Args:
            file: Path to the document.
            out_dir: Optional directory to write output files.
            chunk_size: Maximum tokens per chunk.
            overlap: Overlap tokens between chunks.
            max_cluster: Maximum number of clusters per layer.
            tree_builder: Tree builder to use ("raptor" or "psi").
            clustering: Clustering method ("gmm" or "ahc").
            entity_types: Comma-separated list of entity types to extract.
            language: Target language for extraction.
            model: Gemini model to use.
            embed_model: Embedding model name.
            no_raptor: If True, skip building the RAPTOR hierarchy.
            no_graph: If True, skip extracting the entity-relationship graph.
            project: Google Cloud project ID.
            location: Google Cloud region.
        """
        from aphrody.rag import RAGPipeline

        file_path = Path(file)
        if not file_path.exists():
            raise FileNotFoundError(f"File not found: {file}")

        proj_id = resolve_project(project)
        loc_id = resolve_location(location)

        llm_chat_fn = _make_llm_chat(model, proj_id, loc_id)
        embed_fn = _make_embed_fn(embed_model, proj_id, loc_id)

        types_list = (
            [t.strip() for t in entity_types.split(",")]
            if entity_types
            else None
        )

        pipeline = RAGPipeline(
            llm_chat_fn=llm_chat_fn,
            embed_fn=embed_fn,
            chunk_token_num=chunk_size,
            overlap_token_num=overlap,
            max_cluster=max_cluster,
            raptor_tree_builder=tree_builder,
            raptor_clustering_method=clustering,
            graph_entity_types=types_list,
            language=language,
        )

        async def run_pipeline():
            return await pipeline.process_document(
                str(file_path),
                doc_id=file_path.stem,
                build_raptor_tree=not no_raptor,
                build_graph=not no_graph,
            )

        result = asyncio.run(run_pipeline())

        output_data = {
            "file": str(file_path),
            "chunks_count": len(result["chunks"]),
            "raptor_layers_count": len(result["raptor_layers"]),
        }

        graph_obj = result.get("graph")
        if graph_obj:
            nodes = []
            for node, data in graph_obj.nodes(data=True):
                nodes.append({"id": node, **data})
            edges = []
            for u, v, data in graph_obj.edges(data=True):
                edges.append({"source": u, "target": v, **data})
            output_data["graph_nodes_count"] = len(nodes)
            output_data["graph_edges_count"] = len(edges)
            output_data["graph"] = {
                "nodes": nodes,
                "edges": edges,
            }
        else:
            output_data["graph_nodes_count"] = 0
            output_data["graph_edges_count"] = 0
            output_data["graph"] = None

        output_data["chunks"] = result["chunks"]

        if out_dir:
            out_path = Path(out_dir)
            out_path.mkdir(parents=True, exist_ok=True)

            # Write chunks
            with open(out_path / "chunks.json", "w", encoding="utf-8") as f:
                json.dump(result["chunks"], f, indent=2, ensure_ascii=False)

            # Write GraphML if graph was built
            if graph_obj:
                import networkx as nx

                # GraphML does not support list attributes. Convert list attributes to strings.
                g_write = graph_obj.copy()
                for _, data in g_write.nodes(data=True):
                    for key, val in list(data.items()):
                        if isinstance(val, list):
                            data[key] = ",".join(str(v) for v in val)
                for _, _, data in g_write.edges(data=True):
                    for key, val in list(data.items()):
                        if isinstance(val, list):
                            data[key] = ",".join(str(v) for v in val)

                nx.write_graphml(g_write, str(out_path / "graph.graphml"))
                # Also write as json
                with open(out_path / "graph.json", "w", encoding="utf-8") as f:
                    json.dump(
                        output_data["graph"], f, indent=2, ensure_ascii=False
                    )

            _emit(
                {
                    "status": "success",
                    "output_directory": str(out_path),
                    "chunks_count": len(result["chunks"]),
                    "graph_nodes_count": output_data["graph_nodes_count"],
                    "graph_edges_count": output_data["graph_edges_count"],
                }
            )
        else:
            _emit(output_data)
