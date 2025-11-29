import { createRequire } from "module"
const require = createRequire(import.meta.url)
const native = require("./index.node")

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
