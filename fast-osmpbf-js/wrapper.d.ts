import { JsElementBlock } from "./index"

export * from "./index"
export function getTags(block: JsElementBlock, index: number): [string, string][]
export function getNodeIds(block: JsElementBlock, index: number): number[]
export function getRelationMembers(block: JsElementBlock, index: number): { id: number; type: string; role: number }[]
