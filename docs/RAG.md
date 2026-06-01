# Retrieval-Augmented Generation (RAG) in Aphrody

Aphrody provides a high-performance, modular RAG pipeline that integrates layout-based document chunking, RAPTOR hierarchical indexing, and GraphRAG entity-relationship extraction.

---

## Architecture & Workflow

The RAG workflow processes documents in three main phases:

1. **Layout-Based Chunking** (`LayoutChunker`): Splits source documents (PDF, DOCX, Markdown, etc.) based on visual and logical structure (headings, lists, code blocks, tables), preserving layout context.
2. **RAPTOR Clustering** (`RecursiveAbstractiveProcessing4TreeOrganizedRetrieval`): Generates a recursive tree of text segments by clustering semantic embeddings and summarizing node clusters using Gemini models.
3. **GraphRAG Extraction** (`GraphExtractor`): Performs parallel extraction of entities (e.g., PERSON, ORGANIZATION, EVENT) and relationships using LLM prompts, building a semantic network mapped via `networkx`.

---

## CLI Usage

Exposed via the `aphrody rag` subcommand group:

```bash
# 1. Parse and chunk a document using layout boundaries
aphrody rag chunk --file document.md

# 2. Build the RAPTOR tree hierarchy and view summarizes
aphrody rag raptor --file document.md --max-cluster 5 --model gemini-3.5-flash

# 3. Extract the GraphRAG semantic network
aphrody rag graph --file document.md --entity-types "PERSON,ORGANIZATION"

# 4. Run the end-to-end RAG pipeline and write results to a directory
aphrody rag process --file document.md --out-dir ./output/
```

### Outputs

When running `aphrody rag process` with `--out-dir`, the following files are produced:
- `chunks.json`: Document chunks with layouts, heading context, and embeddings.
- `graph.json`: JSON representation of the entity-relationship network.
- `graph.graphml`: Standard XML GraphML representation for visualization tools (e.g., Gephi).

---

## Programmatic API

The python modules can be imported directly:

```python
from aphrody.rag import RAGPipeline

pipeline = RAGPipeline(
    llm_chat_fn=llm_chat,
    embed_fn=embed_texts,
    chunk_token_num=512,
    overlap_token_num=64,
)

result = await pipeline.process_document("path/to/doc.md", doc_id="unique_doc_1")

# Access chunks, layers, and networkx Graph
chunks = result["chunks"]
raptor_layers = result["raptor_layers"]
graph = result["graph"]
```
