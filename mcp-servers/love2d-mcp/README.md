# LÖVE2D MCP Server

A Model Context Protocol (MCP) server that enables AI assistants to interact with running LÖVE2D games. This allows for real-time introspection, debugging, and manipulation of game state through natural language.

## Overview

This project bridges LÖVE2D games with MCP-compatible AI assistants, enabling:

- **Real-time introspection**: Query game objects, positions, properties, and state
- **Dynamic code execution**: Run Lua code directly in the game context
- **AI-assisted debugging**: Let AI assistants analyze and manipulate your game
- **Interactive development**: Modify game behavior without restarting

## Architecture

```
┌─────────────┐ stdio  ┌─────────────┐  TCP   ┌─────────────┐
│ MCP Client  │◄──────►│  MCP Server │◄──────►│  LÖVE2D     │
│ (Inspector) │        │ (TypeScript)│        │  Game (Lua) │
└─────────────┘        └─────────────┘        └─────────────┘
```

- **MCP Server**: Node.js/TypeScript server communicating via stdio
- **LÖVE2D Bridge**: Lua TCP server embedded in your game
- **Communication**: JSON-RPC over TCP between server and game

## Installation

### Prerequisites

- Node.js 18+ and npm
- LÖVE2D 11.0+ ([download here](https://love2d.org))
- Git

### Setup

1. Clone the repository:
```bash
git clone https://github.com/shayarnett/love2d-mcp.git
cd love2d-mcp
```

2. Install dependencies:
```bash
npm install
```

3. Build the TypeScript server:
```bash
npm run build
```

## Quick Start

### 1. Start the Example Game

```bash
love game/
```

You should see a window with 5 bouncing balls. The game starts a TCP server on port 12345.

### 2. Connect with MCP Inspector

In a new terminal:

```bash
npx @modelcontextprotocol/inspector node build/index.js
```

This opens the MCP Inspector in your browser where you can interact with the game.

### 3. Try Some Commands

**List all objects:**
- Select the `list_objects` tool
- Execute it to see all game objects

**Get object details:**
- Select the `get_object` tool
- Enter an object ID from the list (e.g., `ball_1`)
- View detailed properties

**Run custom Lua code:**
- Select the `run_lua` tool
- Try this code to count purple balls:

```lua
local count = 0
for id, obj in pairs(objects) do
  if obj.type == "ball" then
    if obj.b > 0.5 and obj.r > 0.5 and obj.g < 0.5 then
      count = count + 1
    end
  end
end
return count
```

## Available MCP Tools

### `list_objects`

Lists all objects in the current game scene.

**Parameters:** None

**Returns:** Array of objects with `id`, `type`, `x`, and `y` properties.

### `get_object`

Get detailed information about a specific object.

**Parameters:**
- `id` (string): The object ID

**Returns:** Complete object data including all properties.

### `run_lua`

Execute arbitrary Lua code in the game context with access to game objects.

**Parameters:**
- `code` (string): Lua code to execute

**Returns:** Result of the code execution (string or JSON-encoded table).

**Available in code context:**
- `objects`: Table of all game objects
- `love`: LÖVE2D API
- Standard Lua functions and libraries

## Integrating with Your Game

To add MCP support to your own LÖVE2D game:

1. Copy `game/mcp_bridge.lua` to your game directory

2. In your `main.lua`:

```lua
local mcp_bridge = require("mcp_bridge")

function love.load()
    -- Initialize MCP bridge on port 12345
    mcp_bridge.init(12345)

    -- Provide access to your game objects
    mcp_bridge.setObjectGetter(function()
        return your_game_objects_table
    end)
end

function love.update(dt)
    -- Call this every frame to handle MCP commands
    mcp_bridge.update()

    -- Your game logic here
end

function love.quit()
    mcp_bridge.shutdown()
end
```

3. Start your game and connect via MCP Inspector

## Development

### Watch Mode

For development with auto-rebuild:

```bash
npm run dev
```

### Project Structure

```
love2d-mcp/
├── src/
│   └── index.ts          # MCP server implementation
├── game/
│   ├── main.lua          # Example LÖVE2D game
│   └── mcp_bridge.lua    # TCP bridge module
├── build/                # Compiled TypeScript
├── package.json
├── tsconfig.json
└── README.md
```

## Use Cases

- **Debugging**: Query game state without adding debug UI
- **Testing**: Automate game testing through AI
- **Prototyping**: Quickly test ideas by running Lua snippets
- **Learning**: Explore LÖVE2D by asking AI to analyze your game
- **AI-assisted development**: Let AI help build game features

## Troubleshooting

### Game won't start
- Ensure LÖVE2D is installed: `love --version`
- Check for syntax errors in Lua files

### MCP connection fails
- Verify the game is running first
- Check port 12345 isn't already in use
- Look for "MCP Bridge listening" message in game console

### Inspector shows errors
- Rebuild the TypeScript: `npm run build`
- Check Node.js version: `node --version` (requires 18+)

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Built with the [Model Context Protocol](https://modelcontextprotocol.io/)
- Powered by [LÖVE2D](https://love2d.org/)
- Inspired by the need for better game development tools

## Links

- [MCP Documentation](https://modelcontextprotocol.io/)
- [LÖVE2D Documentation](https://love2d.org/wiki/)
- [Report Issues](https://github.com/shayarnett/love2d-mcp/issues)
