#!/usr/bin/env node

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { z } from "zod";
import net from "net";

const LOVE2D_HOST = "localhost";
const LOVE2D_PORT = 12345;

// TCP client to communicate with LÖVE2D game
class Love2DClient {
  private client: net.Socket | null = null;

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.client = net.createConnection({ host: LOVE2D_HOST, port: LOVE2D_PORT }, () => {
        console.error("Connected to LÖVE2D game");
        resolve();
      });

      this.client.on("error", (err) => {
        console.error("TCP connection error:", err);
        reject(err);
      });
    });
  }

  async sendCommand(command: any): Promise<any> {
    if (!this.client) {
      await this.connect();
    }

    return new Promise((resolve, reject) => {
      const data = JSON.stringify(command) + "\n";

      this.client!.write(data, (err) => {
        if (err) {
          reject(err);
          return;
        }
      });

      // Wait for response
      const onData = (buffer: Buffer) => {
        try {
          const response = JSON.parse(buffer.toString());
          this.client!.off("data", onData);
          resolve(response);
        } catch (err) {
          reject(err);
        }
      };

      this.client!.on("data", onData);
    });
  }

  disconnect(): void {
    if (this.client) {
      this.client.end();
      this.client = null;
    }
  }
}

const love2dClient = new Love2DClient();

// Create MCP server
const server = new Server(
  {
    name: "love2d-mcp",
    version: "1.0.0",
  },
  {
    capabilities: {
      tools: {},
    },
  }
);

// List available tools
server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: [
      {
        name: "list_objects",
        description: "List all objects in the current game scene",
        inputSchema: {
          type: "object",
          properties: {},
        },
      },
      {
        name: "get_object",
        description: "Get detailed information about a specific object by ID",
        inputSchema: {
          type: "object",
          properties: {
            id: {
              type: "string",
              description: "The ID of the object to retrieve",
            },
          },
          required: ["id"],
        },
      },
      {
        name: "run_lua",
        description: "Execute arbitrary Lua code in the game context",
        inputSchema: {
          type: "object",
          properties: {
            code: {
              type: "string",
              description: "The Lua code to execute",
            },
          },
          required: ["code"],
        },
      },
    ],
  };
});

// Handle tool calls
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  try {
    switch (name) {
      case "list_objects": {
        const response = await love2dClient.sendCommand({
          command: "list_objects",
        });
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(response, null, 2),
            },
          ],
        };
      }

      case "get_object": {
        const objectId = (args as any).id;
        const response = await love2dClient.sendCommand({
          command: "get_object",
          id: objectId,
        });
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(response, null, 2),
            },
          ],
        };
      }

      case "run_lua": {
        const code = (args as any).code;
        const response = await love2dClient.sendCommand({
          command: "run_lua",
          code: code,
        });
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(response, null, 2),
            },
          ],
        };
      }

      default:
        throw new Error(`Unknown tool: ${name}`);
    }
  } catch (error) {
    return {
      content: [
        {
          type: "text",
          text: `Error: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
      isError: true,
    };
  }
});

// Start the server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("LÖVE2D MCP server running on stdio");
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
