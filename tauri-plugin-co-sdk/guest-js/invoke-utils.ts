import { decode, encode } from "@ipld/dag-cbor";
import { invoke } from "@tauri-apps/api/core";
import { CID } from "multiformats";

export async function sessionOpen(coId: string): Promise<string> {
  return await invoke("plugin:co-sdk|session_open", { coId });
}

export async function sessionClose(sessionId: string) {
  await invoke("plugin:co-sdk|session_close", { sessionId });
}

export async function getCoState(co: string): Promise<[CID | undefined, CID[]]> {
  const result = await invoke<ArrayBuffer>("plugin:co-sdk|get_co_state", { co });
  return decode<[CID | undefined, CID[]]>(result);
}
export async function pushAction(
  session: string,
  core: string,
  action: any,
  identity: string,
): Promise<CID | undefined> {
  const body = encode({ session, core, action, identity });
  const result = await invoke<ArrayBuffer>("plugin:co-sdk|push_action", { body });
  return decode<CID | undefined>(result);
}
export async function resolveCid(session: string, cid: CID): Promise<any> {
  let body = encode({ session, cid });
  const result = await invoke<ArrayBuffer>("plugin:co-sdk|resolve_cid", { body });
  return decode(result);
}
export async function storageGet(session: string, cid: CID): Promise<Uint8Array> {
  const body = encode({ session, cid });
  const result = await invoke<ArrayBuffer>("plugin:co-sdk|storage_get", { body });
  return new Uint8Array(result);
}
export async function storageSet(session: string, data: Uint8Array): Promise<CID> {
  const result = await invoke<ArrayBuffer>("plugin:co-sdk|storage_set", {session, data});
  return decode(result);
}

export interface GetActionsResponse {
  actions: CID[];
  next_heads: CID[];
}
export async function getActions(
  session: string,
  heads: CID[],
  count: number,
  until: CID | undefined,
): Promise<GetActionsResponse> {
  let body;
  if (until !== undefined) {
    body = encode({ session, heads, count, until });
  } else {
    body = encode({ session, heads, count, until: null });
  }
  const result = await invoke<ArrayBuffer>("plugin:co-sdk|get_actions", { body });
  return decode(result);
}
export async function createIdentity(name: string, seed?: Uint8Array): Promise<string> {
  return await invoke("plugin:co-sdk|create_identity", { name, seed: seed ? Array.from(seed) : undefined });
}
export async function createCo(creatorDid: string, coName: string, isPublic: boolean, coId?: string): Promise<string> {
  return await invoke("plugin:co-sdk|create_co", {
    creatorDid,
    coId,
    coName,
    public: isPublic,
  });
}

export * from "./types/index.js";
