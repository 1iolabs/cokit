import { invoke } from "@tauri-apps/api/core";
import { CID } from "multiformats";

export async function invoke_get(co: string, core: string): Promise<any> {
    let a: [CID, CID[]] = await invoke("get_co_state", { co });
    let state: any = await invoke("resolve_cid", { co, cid: a[0] })
    const core_cid = state?.cores?.[core]?.state;
    if (core_cid) {
        let core_state = await invoke("resolve_cid", { co, cid: core_cid });
        console.log("bbb", core_state);
        return core_state;
    }
    console.log("aaa", state);
    return null;
}