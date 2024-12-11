import { encode } from "@ipld/dag-cbor";
import { invoke } from "@tauri-apps/api/core";
import { CID } from "multiformats";


export async function getCoState(co: string): Promise<[CID | undefined, CID[]]> {
    return await invoke("plugin:co-sdk|get_co_state", { co });
}
export async function pushAction(co: string, core: string, action: any): Promise<CID | undefined> {
    let body_raw = encode({ action, co, core });
    return await invoke("plugin:co-sdk|push_action", { body: Array.from(body_raw) });
}
export async function resolveCid(co: string, cid: CID): Promise<any> {
    return await invoke("plugin:co-sdk|resolve_cid", { co, cid });
}
export async function storageGet(co: string, cid: CID): Promise<Uint8Array> {
    return await invoke("plugin:co-sdk|storage_get", { co, cid });
}
export async function storageSet(co: string, data: Uint8Array, cid: CID): Promise<CID> {
    return await invoke("plugin:co-sdk|storage_set", { co, data, cid });
}

export interface GetActionsResponse {
    actions: CID[];
    next_heads: CID[];
}
export async function get_actions(co: string, heads: CID[], count: number, until: CID | undefined): Promise<GetActionsResponse> {
    return await invoke("plugin:co-sdk|get_actions", { co, heads, count, until });
}
