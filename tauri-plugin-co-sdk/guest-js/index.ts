import { encode } from "@ipld/dag-cbor";
import { invoke } from "@tauri-apps/api/core";
import { CID } from "multiformats";

export async function sessionOpen(coId: string): Promise<string> {
    return await invoke("plugin:co-sdk|session_open", { coId });
}

export async function sessionClose(sessionId: string) {
    await invoke("plugin:co-sdk|session_close", { sessionId });
}

export async function getCoState(co: string): Promise<[CID | undefined, CID[]]> {
    return await invoke("plugin:co-sdk|get_co_state", { co });
}
export async function pushAction(co: string, core: string, action: any, identity: string): Promise<CID | undefined> {
    let body_raw = encode({ action, co, core, identity });
    return await invoke("plugin:co-sdk|push_action", { body: Array.from(body_raw) });
}
export async function resolveCid(sessionId: string, cid: CID): Promise<any> {
    return await invoke("plugin:co-sdk|resolve_cid", { sessionId, cid });
}
export async function storageGet(sessionId: string, cid: CID): Promise<Uint8Array> {
    return await invoke("plugin:co-sdk|storage_get", { co: sessionId, cid });
}
export async function storageSet(sessionId: string, cid: CID, data: Uint8Array): Promise<CID> {
    return await invoke("plugin:co-sdk|storage_set", { co: sessionId, cid, data });
}

export interface GetActionsResponse {
    actions: CID[];
    next_heads: CID[];
}
export async function get_actions(sessionId: string, heads: CID[], count: number, until: CID | undefined): Promise<GetActionsResponse> {
    return await invoke("plugin:co-sdk|get_actions", { sessionId, heads, count, until });
}
export async function createIdentity(name: string, seed?: Uint8Array) {
    return await invoke("plugin:co-sdk|create_identity", { name, seed: seed ? Array.from(seed) : undefined });
}
export async function createCo(
    creatorDid: string,
    coName: string,
    isPublic: boolean,
    coId?: string,
) {
    return await invoke("plugin:co-sdk|create_co", {
        creatorDid,
        coId,
        coName,
        public: isPublic,
    });
}
