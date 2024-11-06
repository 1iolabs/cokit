import { invoke } from "@tauri-apps/api/core";
import { CID } from "multiformats";

export async function getState(coId: String): Promise<[CID | undefined, Set<CID>]> {
    return await invoke("plugin:co-sdk|get_state", { coId });
}