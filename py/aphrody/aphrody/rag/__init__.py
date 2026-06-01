# SPDX-License-Identifier: Apache-2.0

"""RAG Unified Pipeline API.

Bridges layout-based document chunking, RAPTOR recursive clustering/indexing,
and GraphRAG entity/relationship extraction.
"""

import logging
from collections.abc import Callable
from typing import Any, Optional

import networkx as nx

from aphrody.rag.graph_extractor import GraphExtractor
from aphrody.rag.layout_chunker import LayoutChunker, LayoutElement
from aphrody.rag.raptor import (
    RecursiveAbstractiveProcessing4TreeOrganizedRetrieval,
)

logger = logging.getLogger(__name__)

__all__ = [
    "GraphExtractor",
    "LayoutChunker",
    "LayoutElement",
    "RAGPipeline",
    "RecursiveAbstractiveProcessing4TreeOrganizedRetrieval",
]


class RAGPipeline:
    """Unified pipeline integrating Layout parsing, RAPTOR clustering, and GraphRAG extraction."""

    def __init__(
        self,
        llm_chat_fn: Callable[[str, list[dict[str, str]], dict[str, Any]], Any],
        embed_fn: Callable[[list[str]], Any],
        chunk_token_num: int = 512,
        overlap_token_num: int = 64,
        max_cluster: int = 5,
        raptor_tree_builder: str = "raptor",
        raptor_clustering_method: str = "gmm",
        graph_entity_types: list[str] | None = None,
        language: str = "English",
    ):
        self.llm_chat_fn = llm_chat_fn
        self.embed_fn = embed_fn
        self.language = language

        self.chunker = LayoutChunker(
            chunk_token_num=chunk_token_num, overlap_token_num=overlap_token_num
        )

        raptor_prompt = (
            "Write a concise summary of the following texts. "
            "Do not include introductory or concluding phrases.\n\n"
            "Texts:\n{cluster_content}\n\nSummary:"
        )
        self.raptor_builder = (
            RecursiveAbstractiveProcessing4TreeOrganizedRetrieval(
                max_cluster=max_cluster,
                llm_chat_fn=llm_chat_fn,
                embed_fn=embed_fn,
                prompt=raptor_prompt,
                max_token=chunk_token_num,
                tree_builder=raptor_tree_builder,
                clustering_method=raptor_clustering_method,
            )
        )

        self.graph_extractor = GraphExtractor(
            llm_chat_fn=llm_chat_fn,
            entity_types=graph_entity_types,
            language=language,
        )

    async def process_document(
        self,
        file_path: str,
        binary_data: bytes | None = None,
        doc_id: str = "doc",
        build_raptor_tree: bool = True,
        build_graph: bool = True,
    ) -> dict[str, Any]:
        """Process document end-to-end.

        1. Layout parsing and chunking.
        2. RAPTOR tree-building (clustering & abstractive summarization of layers).
        3. GraphRAG entity/relationship extraction.
        """
        logger.info("Parsing and chunking document: %s", file_path)

        layout_chunks = self.chunker.chunk_document(file_path, binary_data)
        original_texts = [c["content"] for c in layout_chunks]

        embeddings = []
        if original_texts:
            import asyncio

            if asyncio.iscoroutinefunction(self.embed_fn):
                embeddings = await self.embed_fn(original_texts)
            else:
                embeddings = self.embed_fn(original_texts)

        raptor_chunks = []
        for text, embd in zip(original_texts, embeddings):
            raptor_chunks.append((text, embd))

        final_chunks = list(raptor_chunks)
        layers = []
        if build_raptor_tree and len(raptor_chunks) > 1:
            logger.info("Building RAPTOR tree structure...")
            res_chunks, res_layers = await self.raptor_builder(raptor_chunks)
            if res_chunks:
                final_chunks = res_chunks
                layers = res_layers

        graph = None
        if build_graph and original_texts:
            logger.info("Extracting GraphRAG entity relationship network...")
            graph = await self.graph_extractor.extract_graph(
                original_texts, doc_id=doc_id
            )

        output_chunks = []
        for idx, (text, embd) in enumerate(final_chunks[: len(original_texts)]):
            layout_info = layout_chunks[idx]
            output_chunks.append(
                {
                    "id": f"{doc_id}_chunk_{idx}",
                    "content": text,
                    "embedding": embd,
                    "type": "original",
                    "layout_types": layout_info["layout_types"],
                    "heading_context": layout_info["heading_context"],
                }
            )

        for idx, (text, embd) in enumerate(final_chunks[len(original_texts) :]):
            output_chunks.append(
                {
                    "id": f"{doc_id}_raptor_summary_{idx}",
                    "content": text,
                    "embedding": embd,
                    "type": "summary",
                    "layout_types": ["summary"],
                    "heading_context": "",
                }
            )

        return {
            "chunks": output_chunks,
            "raptor_layers": layers,
            "graph": graph,
        }
