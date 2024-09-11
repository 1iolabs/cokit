import { isNonNull } from "@1io/kui-application-sdk";
import { invoke } from "@tauri-apps/api/core";
import { CID } from "multiformats";
import { buildCoCoreId } from "./core-id";

export async function invokeResolveCid(co: string, cid: CID): Promise<any> {
    return await invoke("resolve_cid", { co, cid });
}

export async function invokeGetCoState(co: string): Promise<any> {
    let [stateCid]: [CID, CID[]] = await invoke("get_co_state", { co });
    let state: any = await invokeResolveCid(co, stateCid);
    return state;
}

export async function invokeGetCoHeads(co: string): Promise<any[]> {
    let [_, headsCids]: [CID, CID[]] = await invoke("get_co_state", { co });
    const heads = [];
    for (const headCid of headsCids) {
        heads.push(await invokeResolveCid(co, headCid));
    }
    return heads;
}

export async function invokeGetCoreState(co: string, core: string): Promise<any> {
    const state = await invokeGetCoState(co);
    const core_cid = state?.cores?.[core]?.state;
    if (core_cid) {
        let core_state = await invokeResolveCid(co, core_cid);
        console.log("bbb", core_state);
        return core_state;
    }
    console.log("aaa", state);
    return null;
}

export async function invokeGetCoIds() {
    const memberships = await invokeGetCoreState("local", "membership");
    console.log("memberships", memberships, Array.isArray(memberships?.memberships));
    if (Array.isArray(memberships?.memberships)) {
        console.log("isArray");
        return memberships.memberships.map((membership: any) => membership?.id).filter(isNonNull);
    }
    return [];
}

export async function invokeGetFilteredCores(tags: string[], co?: string): Promise<string[]> {
    const coIds: string[] = [];
    if (co !== undefined) {
        coIds.push(co);
    } else {
        coIds.push(...(await invokeGetCoIds()));
    }
    console.log("checking cores of coIds:", coIds);
    const foundCores: string[] = [];
    for (const coId of coIds) {
        const state = await invokeGetCoState(coId);
        for (let [key, value] of Object.entries(state.cores)) {
            // TODO remove any cast when js interfaces are done
            const v = value as any;
            if (v?.tags) {
                console.log("value", v);
                if (v.tags[0].every((tag: string) => tags.includes(tag))) {
                    foundCores.push(buildCoCoreId(coId, key));
                }
            }
        }
    }
    return foundCores;
}

