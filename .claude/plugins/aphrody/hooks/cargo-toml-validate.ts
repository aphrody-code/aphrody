#!/usr/bin/env bun
// PostToolUse hook : after Edit/Write on any Cargo.toml, run
// `cargo metadata --no-deps --offline` to catch malformed manifests
// immediately (broken schema, unresolved workspace deps, missing crates).
//
// BLOCKING by default — broken Cargo.toml means the whole workspace
// fails; Claude needs to know now, not in 5 minutes during cargo build.

import { spawn } from 'node:child_process';

interface HookPayload {
    tool_input?: {
        file_path?: string;
        edits?: Array<{ file_path?: string }>;
    };
}

async function readStdin(): Promise<string> {
    const chunks: Buffer[] = [];
    for await (const chunk of process.stdin) {
        chunks.push(chunk as Buffer);
    }
    return Buffer.concat(chunks).toString('utf8');
}

function extractCargoToml(payload: HookPayload): string[] {
    const paths: string[] = [];
    const input = payload.tool_input ?? {};
    if (input.file_path) paths.push(input.file_path);
    if (Array.isArray(input.edits)) {
        for (const e of input.edits) {
            if (e?.file_path) paths.push(e.file_path);
        }
    }
    return paths.filter((p) => /(^|[\\/])Cargo\.toml$/.test(p));
}

async function runCargoMetadata(): Promise<{ code: number; stderr: string }> {
    return new Promise((resolveResult) => {
        const child = spawn('cargo', ['metadata', '--no-deps', '--offline', '--format-version', '1'], {
            stdio: ['ignore', 'ignore', 'pipe'],
            shell: false,
        });
        let stderr = '';
        child.stderr?.on('data', (b: Buffer) => {
            stderr += b.toString('utf8');
        });
        child.on('close', (code) => resolveResult({ code: code ?? 0, stderr }));
        child.on('error', () => resolveResult({ code: 0, stderr: '' }));
    });
}

async function main(): Promise<void> {
    let raw: string;
    try {
        raw = await readStdin();
    } catch {
        process.exit(0);
    }
    if (!raw.trim()) process.exit(0);

    let payload: HookPayload;
    try {
        payload = JSON.parse(raw);
    } catch {
        process.exit(0);
    }

    const cargoFiles = extractCargoToml(payload);
    if (cargoFiles.length === 0) process.exit(0);

    const result = await runCargoMetadata();
    if (result.code !== 0) {
        process.stderr.write(
            `[cargo-toml-validate] cargo metadata FAILED after editing ${cargoFiles.join(', ')}\n`,
        );
        process.stderr.write(result.stderr);
        // Blocking exit — broken manifest is critical, Claude must fix before next edit.
        process.exit(2);
    }
    process.exit(0);
}

main().catch((err) => {
    process.stderr.write(`[cargo-toml-validate] hook crashed: ${err instanceof Error ? err.message : String(err)}\n`);
    process.exit(0);
});
