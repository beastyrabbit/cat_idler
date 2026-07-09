-- MCP Bridge - TCP server for communicating with MCP server
local socket = require("socket")

local mcp_bridge = {}
local server = nil
local clients = {}
local objectGetter = nil

-- Initialize TCP server
function mcp_bridge.init(port)
    server = assert(socket.tcp())
    server:bind("*", port)
    server:listen(5)
    server:settimeout(0) -- Non-blocking
    print("MCP Bridge listening on port " .. port)
end

-- Set function to get game objects
function mcp_bridge.setObjectGetter(getter)
    objectGetter = getter
end

-- Handle incoming connections and commands
function mcp_bridge.update()
    if not server then return end

    -- Accept new clients
    local client = server:accept()
    if client then
        client:settimeout(0)
        table.insert(clients, client)
        print("MCP client connected")
    end

    -- Handle existing clients
    for i = #clients, 1, -1 do
        local client = clients[i]
        local line, err = client:receive("*l")

        if line then
            -- Process command
            local success, response = pcall(mcp_bridge.handleCommand, line)
            if success then
                client:send(response .. "\n")
            else
                client:send(json.encode({error = tostring(response)}) .. "\n")
            end
        elseif err == "closed" then
            -- Client disconnected
            client:close()
            table.remove(clients, i)
            print("MCP client disconnected")
        end
        -- err == "timeout" means no data available, continue
    end
end

-- Handle a command from MCP server
function mcp_bridge.handleCommand(line)
    local command = json.decode(line)

    if command.command == "list_objects" then
        return mcp_bridge.listObjects()
    elseif command.command == "get_object" then
        return mcp_bridge.getObject(command.id)
    elseif command.command == "run_lua" then
        return mcp_bridge.runLua(command.code)
    else
        return json.encode({error = "Unknown command: " .. tostring(command.command)})
    end
end

-- List all objects
function mcp_bridge.listObjects()
    if not objectGetter then
        return json.encode({error = "No object getter configured"})
    end

    local objects = objectGetter()
    local result = {}

    for id, obj in pairs(objects) do
        table.insert(result, {
            id = id,
            type = obj.type,
            x = obj.x,
            y = obj.y
        })
    end

    return json.encode({objects = result})
end

-- Get specific object details
function mcp_bridge.getObject(id)
    if not objectGetter then
        return json.encode({error = "No object getter configured"})
    end

    local objects = objectGetter()
    local obj = objects[id]

    if not obj then
        return json.encode({error = "Object not found: " .. tostring(id)})
    end

    return json.encode({object = obj})
end

-- Run arbitrary Lua code with access to game objects
function mcp_bridge.runLua(code)
    local func, err = loadstring(code)
    if not func then
        return json.encode({error = "Syntax error: " .. tostring(err)})
    end

    -- Set up environment with access to objects
    local env = {
        objects = objectGetter and objectGetter() or {},
        love = love,
        print = print,
        pairs = pairs,
        ipairs = ipairs,
        type = type,
        tostring = tostring,
        tonumber = tonumber,
        table = table,
        math = math,
        string = string,
    }
    setfenv(func, env)

    local success, result = pcall(func)
    if not success then
        return json.encode({error = "Runtime error: " .. tostring(result)})
    end

    -- Handle table results by encoding them
    if type(result) == "table" then
        return json.encode({result = result})
    else
        return json.encode({result = tostring(result)})
    end
end

-- Shutdown server
function mcp_bridge.shutdown()
    -- Close all clients
    for _, client in ipairs(clients) do
        client:close()
    end
    clients = {}

    -- Close server
    if server then
        server:close()
        server = nil
        print("MCP Bridge shut down")
    end
end

-- Simple JSON encoder/decoder
json = {}

function json.encode(obj)
    local t = type(obj)
    if t == "table" then
        local parts = {}
        local isArray = true
        local arraySize = 0

        -- Check if it's an array
        for k, v in pairs(obj) do
            if type(k) ~= "number" then
                isArray = false
                break
            end
            arraySize = arraySize + 1
        end

        if isArray and arraySize > 0 then
            for i, v in ipairs(obj) do
                table.insert(parts, json.encode(v))
            end
            return "[" .. table.concat(parts, ",") .. "]"
        else
            for k, v in pairs(obj) do
                local key = type(k) == "string" and json.encode(k) or tostring(k)
                table.insert(parts, key .. ":" .. json.encode(v))
            end
            return "{" .. table.concat(parts, ",") .. "}"
        end
    elseif t == "string" then
        return '"' .. obj:gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n') .. '"'
    elseif t == "number" or t == "boolean" then
        return tostring(obj)
    elseif t == "nil" then
        return "null"
    else
        return '"' .. tostring(obj) .. '"'
    end
end

function json.decode(str)
    local pos = 1

    local function skip_whitespace()
        while pos <= #str and str:sub(pos, pos):match("%s") do
            pos = pos + 1
        end
    end

    local function decode_string()
        local result = ""
        pos = pos + 1 -- skip opening quote
        while pos <= #str do
            local char = str:sub(pos, pos)
            if char == '"' then
                pos = pos + 1
                return result
            elseif char == "\\" then
                pos = pos + 1
                local escape = str:sub(pos, pos)
                if escape == "n" then result = result .. "\n"
                elseif escape == "t" then result = result .. "\t"
                elseif escape == "r" then result = result .. "\r"
                elseif escape == "\\" then result = result .. "\\"
                elseif escape == '"' then result = result .. '"'
                else result = result .. escape end
                pos = pos + 1
            else
                result = result .. char
                pos = pos + 1
            end
        end
        error("Unterminated string")
    end

    local function decode_value()
        skip_whitespace()
        local char = str:sub(pos, pos)

        if char == '"' then
            return decode_string()
        elseif char == "{" then
            local obj = {}
            pos = pos + 1
            skip_whitespace()
            if str:sub(pos, pos) == "}" then
                pos = pos + 1
                return obj
            end
            while true do
                skip_whitespace()
                local key = decode_string()
                skip_whitespace()
                if str:sub(pos, pos) ~= ":" then error("Expected :") end
                pos = pos + 1
                obj[key] = decode_value()
                skip_whitespace()
                char = str:sub(pos, pos)
                if char == "}" then
                    pos = pos + 1
                    return obj
                elseif char == "," then
                    pos = pos + 1
                else
                    error("Expected , or }")
                end
            end
        elseif char == "[" then
            local arr = {}
            pos = pos + 1
            skip_whitespace()
            if str:sub(pos, pos) == "]" then
                pos = pos + 1
                return arr
            end
            while true do
                table.insert(arr, decode_value())
                skip_whitespace()
                char = str:sub(pos, pos)
                if char == "]" then
                    pos = pos + 1
                    return arr
                elseif char == "," then
                    pos = pos + 1
                else
                    error("Expected , or ]")
                end
            end
        elseif str:sub(pos, pos + 3) == "true" then
            pos = pos + 4
            return true
        elseif str:sub(pos, pos + 4) == "false" then
            pos = pos + 5
            return false
        elseif str:sub(pos, pos + 3) == "null" then
            pos = pos + 4
            return nil
        else
            local num_str = str:match("^%-?%d+%.?%d*[eE]?[%+%-]?%d*", pos)
            if num_str then
                pos = pos + #num_str
                return tonumber(num_str)
            end
            error("Invalid JSON value at position " .. pos)
        end
    end

    return decode_value()
end

return mcp_bridge
