import { createRequire } from "module"
import { platform, arch } from "process"
import { join, dirname } from "path"
import { fileURLToPath } from "url"

const require = createRequire(import.meta.url)
const __dirname = dirname(fileURLToPath(import.meta.url))

// Map Node.js platform/arch to your binding folder names
function getBindingPath() {
    const platformMap = {
        darwin: "apple-darwin",
        win32: "pc-windows-msvc",
        linux: "unknown-linux-gnu",
    }

    const archMap = {
        x64: "x86_64",
        arm64: "aarch64",
    }

    const nodeFileMap = {
        "linux-x64": "index.linux-x64-gnu.node",
        "linux-arm64": "index.linux-arm64-gnu.node",
        "darwin-x64": "index.darwin-x64.node",
        "darwin-arm64": "index.darwin-arm64.node",
        "win32-x64": "index.win32-x64-msvc.node",
    }

    const mappedPlatform = platformMap[platform]
    const mappedArch = archMap[arch]

    if (!mappedPlatform || !mappedArch) {
        throw new Error(`Unsupported platform: ${platform}-${arch}`)
    }

    const bindingFolder = `bindings-${mappedArch}-${mappedPlatform}`
    const nodeFile = nodeFileMap[`${platform}-${arch}`]

    if (!nodeFile) {
        throw new Error(`No binding available for ${platform}-${arch}`)
    }

    return join(__dirname, bindingFolder, nodeFile)
}

const native = require(getBindingPath())

export function getTags(block, index) {
    let tags = block.denseTags || block.tags
    const start = tags[2][index]
    const end = tags[2][index + 1]
    const result = []
    for (let i = start; i < end; i++) {
        result.push([block.stringTable[tags[0][i]], block.stringTable[tags[1][i]]])
    }
    return result
}

export function getNodeIds(block, index) {
    if (!block.nodeIds) {
        return []
    }
    const result = []
    const start = block.nodeIds[1][index]
    const end = block.nodeIds[1][index + 1]
    for (let i = start; i < end; i++) {
        result.push(block.nodeIds[0][i])
    }
    return result
}

export function getRelationMembers(block, index) {
    if (!block.relationMembers) {
        return []
    }
    const result = []
    const start = block.relationMembers[3][index]
    const end = block.relationMembers[3][index + 1]
    for (let i = start; i < end; i++) {
        result.push({
            id: block.relationMembers[0][i],
            type: mapMemberType(block.relationMembers[1][i]),
            role: block.stringTable[block.relationMembers[2][i]],
        })
    }
    return result
}

function mapMemberType(memberType) {
    switch (memberType) {
        case 0:
            return "Node"
        case 1:
            return "Way"
        case 2:
            return "Relation"
    }
}

// Re-export the NAPI exports
export const OsmReader = native.OsmReader
export const AsyncBlockIterator = native.AsyncBlockIterator
export const JsElementBlock = native.JsElementBlock
