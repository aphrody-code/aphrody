import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { SSEClientTransport } from "@modelcontextprotocol/sdk/client/sse.js";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { FunctionDeclaration, Tool } from "@google/genai";

const clients = new Map<string, Client>();
let mcpServersConfig: any = null;

function loadMcpConfig() {
    if (mcpServersConfig) return mcpServersConfig;
    try {
        const pluginJson = JSON.parse(readFileSync(join(process.cwd(), ".claude", "plugins", "aphrody", ".claude-plugin", "plugin.json"), "utf-8"));
        if (pluginJson.mcpServers) {
            mcpServersConfig = pluginJson.mcpServers;
            return mcpServersConfig;
        }
    } catch (e) {
        // Fallback or ignore
    }
    return {};
}

export async function initMcpClients() {
    const config = loadMcpConfig() as Record<string, any>;
    for (const [name, server] of Object.entries(config)) {
        if (clients.has(name)) continue;
        
        let transport;
        if (server.type === "stdio" || (server.command && !server.type)) {
            transport = new StdioClientTransport({
                command: server.command,
                args: server.args,
                env: { ...process.env, ...server.env }
            });
        } else if (server.type === "streamable-http" || server.httpUrl) {
            // Context7 uses SSE/HTTP
            const url = server.url || server.httpUrl;
            transport = new SSEClientTransport(new URL(url), {
                eventSourceInit: {
                    headers: Object.fromEntries(
                        Object.entries(server.headers || {}).map(([k, v]) => {
                            const val = String(v);
                            if (val.startsWith("${") && val.endsWith("}")) {
                                const envVar = val.slice(2, -1);
                                return [k, process.env[envVar] || ""];
                            }
                            return [k, val];
                        })
                    )
                } as any
            });
        } else {
            console.warn(`Unknown MCP transport for ${name}`);
            continue;
        }
        
        try {
            const client = new Client({
                name: "aphrody-gemini-client",
                version: "1.0.0"
            }, {
                capabilities: {}
            });
            await client.connect(transport);
            clients.set(name, client);
        } catch (e) {
            console.error(`Failed to connect MCP server ${name}`, e);
        }
    }
}

export async function getMcpTools(): Promise<Tool[]> {
    await initMcpClients();
    const declarations: FunctionDeclaration[] = [];
    
    for (const [serverName, client] of clients.entries()) {
        try {
            const toolsResponse = await client.listTools();
            for (const tool of toolsResponse.tools) {
                declarations.push({
                    name: `${serverName}__${tool.name.replace(/[^a-zA-Z0-9_]/g, "_")}`,
                    description: tool.description,
                    parametersJsonSchema: tool.inputSchema as any
                });
            }
        } catch (e) {
            console.error(`Failed to list tools for ${serverName}`, e);
        }
    }
    
    if (declarations.length === 0) return [];
    return [{ functionDeclarations: declarations }];
}

export async function handleMcpToolCall(functionName: string, args: Record<string, unknown>): Promise<any> {
    const parts = functionName.split("__");
    if (parts.length < 2) throw new Error(`Invalid tool name ${functionName}`);
    
    const serverName = parts[0] as string;
    const toolName = parts.slice(1).join("__");
    
    const client = clients.get(serverName);
    if (!client) throw new Error(`MCP server ${serverName} not found`);
    
    const toolsResponse = await client.listTools();
    const tool = toolsResponse.tools.find(t => t.name.replace(/[^a-zA-Z0-9_]/g, "_") === toolName);
    
    if (!tool) throw new Error(`Tool ${toolName} not found on server ${serverName}`);
    
    const result = await client.callTool({
        name: tool.name,
        arguments: args
    });
    
    return result.content;
}
