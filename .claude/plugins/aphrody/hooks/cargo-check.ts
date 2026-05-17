#!/usr/bin/env bun
// PostToolUse hook : after Edit/Write/MultiEdit on *.rs files, run
// `cargo check -p <detected-crate> --offline` in the background to
// refresh rust-analyzer's cache and surface real compile errors.
//
// Non-blocking by design: warnings and "no-diagnostics" pass through as
// exit 0. Real compile errors return exit 1 and surface stderr to
// Claude (advisory, not blocking — Claude can read and react).
//
// Bun-native, cross-platform, zero deps.

import { spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve, sep } from 'node:path';

interface HookPayload {
    session_id?: string;
    tool_name?: string;
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

function extractRustFiles(payload: HookPayload): string[] {
    const paths: string[] = [];
    const input = payload.tool_input ?? {};
    if (input.file_path) paths.push(input.file_path);
    if (Array.isArray(input.edits)) {
        for (const edit of input.edits) {
            if (edit?.file_path) paths.push(edit.file_path);
        }
    }
    return paths.filter((p) => p.endsWith('.rs'));
}

function findCrateRoot(file: string): { manifestDir: string; crateName: string } | null {
    let dir = dirname(resolve(file));
    while (dir && dir !== dirname(dir)) {
        const manifest = resolve(dir, 'Cargo.toml');
        if (existsSync(manifest)) {
            const content = readFileSync(manifest, 'utf8');
            const isWorkspaceVirtual = /^\s*\[workspace\]/m.test(content) && !/^\s*\[package\]/m.test(content);
            if (!isWorkspaceVirtual) {
                const nameMatch = content.match(/^\s*name\s*=\s*"([^"]+)"/m);
                if (nameMatch) {
                    return { manifestDir: dir, crateName: nameMatch[1] };
                }
            }
        }
        dir = dirname(dir);
    }
    return null;
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

    const files = extractRustFiles(payload);
    if (files.length === 0) process.exit(0);

    const crates = new Set<string>();
    for (const f of files) {
        const root = findCrateRoot(f);
        if (root) crates.add(root.crateName);
    }
    if (crates.size === 0) process.exit(0);

    // Run cargo check per crate, sequentially (rustup target lock).
    let hadError = false;
    for (const crate of crates) {
        const result = await new Promise<{ code: number; stderr: string }>((resolveResult) => {
            const child = spawn('cargo', ['check', '-p', crate, '--offline', '--message-format=short'], {
                stdio: ['ignore', 'pipe', 'pipe'],
                shell: false,
            });
            let stderr = '';
            child.stderr?.on('data', (b: Buffer) => {
                stderr += b.toString('utf8');
            });
            child.on('close', (code) => resolveResult({ code: code ?? 0, stderr }));
            child.on('error', () => resolveResult({ code: 0, stderr: '' }));
        });
        if (result.code !== 0) {
            hadError = true;
            // Surface to Claude via stderr (hook stderr is shown to the assistant).
            process.stderr.write(`[cargo-check] crate ${crate} FAILED (exit ${result.code})\n`);
            process.stderr.write(result.stderr);
        }
    }

    // Non-blocking: even on error we exit 0 unless the user has set
    // APHRODY_CARGO_HOOK_BLOCK=1, so editing flow stays smooth.
    if (hadError && process.env.APHRODY_CARGO_HOOK_BLOCK === '1') {
        process.exit(2);
    }
    process.exit(0);
}

main().catch((err) => {
    process.stderr.write(`[cargo-check] hook crashed: ${err instanceof Error ? err.message : String(err)}\n`);
    process.exit(0);
});
