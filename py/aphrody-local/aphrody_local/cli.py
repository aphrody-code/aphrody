"""aphrody-local command-line interface."""

from __future__ import annotations

import sys

import typer
from rich.console import Console

from .client import LocalAI
from .config import Settings
from .engines import discover
from .rag import RagUnavailable, get_backend

app = typer.Typer(
    no_args_is_help=True,
    add_completion=False,
    help="aphrody-local — local open-weight AI orchestrator.",
)
rag_app = typer.Typer(no_args_is_help=True, help="RAG: ingest documents and ask grounded questions.")
app.add_typer(rag_app, name="rag")
console = Console()


@app.command()
def chat(
    prompt: str = typer.Argument(..., help="The user prompt."),
    model: str | None = typer.Option(None, "--model", "-m", help="Model id."),
    system: str | None = typer.Option(None, "--system", "-s", help="System prompt."),
    base_url: str | None = typer.Option(None, "--base-url", help="OpenAI /v1 base."),
) -> None:
    """Stream a chat reply from the local engine."""
    settings = Settings.from_env()
    if base_url:
        settings.base_url = base_url
    if model:
        settings.model = model
    ai = LocalAI(settings)
    try:
        chosen = ai.pick_model(model)
    except RuntimeError as exc:
        console.print(f"[red]{exc}[/red]")
        raise typer.Exit(1) from exc
    console.print(f"[dim]→ {ai.base_url} · {chosen}[/dim]")
    for token in ai.chat_stream(prompt, model=model, system=system):
        console.print(token, end="")
        sys.stdout.flush()
    console.print()


@app.command()
def models(base_url: str | None = typer.Option(None, "--base-url")) -> None:
    """List models advertised by the resolved engine."""
    settings = Settings.from_env()
    if base_url:
        settings.base_url = base_url
    ai = LocalAI(settings)
    found = ai.list_models()
    if not found:
        console.print(f"[yellow]no models at {ai.base_url}[/yellow]")
        raise typer.Exit(1)
    console.print(f"[bold]{ai.base_url}[/bold]")
    for name in found:
        console.print(f"  • {name}")


@app.command()
def engines() -> None:
    """Discover all reachable local OpenAI-compatible engines."""
    found = discover()
    if not found:
        console.print("[yellow]no local engines reachable[/yellow]")
        console.print("start one: `ollama serve`, `aphrody-serve`, or `vllm serve`")
        raise typer.Exit(1)
    for engine, model_list in found:
        console.print(f"[green]●[/green] [bold]{engine.name}[/bold] {engine.base_url}")
        for name in model_list:
            console.print(f"    • {name}")


@app.command()
def doctor() -> None:
    """Show config, live engines, and RAG backend health."""
    settings = Settings.from_env()
    console.print("[bold]aphrody-local doctor[/bold]")
    console.print(f"  rag_backend : {settings.rag_backend}")
    console.print(f"  embed_model : {settings.embed_model}")
    found = discover()
    console.print(f"  engines     : {len(found)} reachable")
    for engine, model_list in found:
        console.print(f"    - {engine.name} ({len(model_list)} models)")
    backend = get_backend(settings)
    ok = backend.health()
    color = "green" if ok else "red"
    console.print(f"  rag '{backend.name}': [{color}]{'ready' if ok else 'unavailable'}[/{color}]")


@rag_app.command("ingest")
def rag_ingest(
    paths: list[str] = typer.Argument(..., help="Files or directories to ingest."),
    dataset: str = typer.Option("default", "--dataset", "-d"),
) -> None:
    """Ingest documents into a RAG dataset."""
    backend = get_backend()
    try:
        count = backend.ingest(paths, dataset=dataset)
    except RagUnavailable as exc:
        console.print(f"[red]RAG unavailable:[/red] {exc}")
        raise typer.Exit(1) from exc
    console.print(f"[green]ingested {count} chunk(s) into '{dataset}' via {backend.name}[/green]")


@rag_app.command("query")
def rag_query(
    query: str = typer.Argument(..., help="The question."),
    dataset: str = typer.Option("default", "--dataset", "-d"),
    top_k: int = typer.Option(5, "--top-k", "-k"),
    show_sources: bool = typer.Option(False, "--sources"),
) -> None:
    """Ask a grounded question against a RAG dataset."""
    backend = get_backend()
    try:
        result = backend.answer(query, dataset=dataset, top_k=top_k)
    except RagUnavailable as exc:
        console.print(f"[red]RAG unavailable:[/red] {exc}")
        raise typer.Exit(1) from exc
    console.print(result.answer)
    if show_sources:
        console.print("\n[dim]── sources ──[/dim]")
        for chunk in result.chunks:
            console.print(f"[dim]· {chunk.source} ({chunk.score:.3f})[/dim]")


if __name__ == "__main__":
    app()
