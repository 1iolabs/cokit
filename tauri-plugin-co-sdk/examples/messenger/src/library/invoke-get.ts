import { isNonNull } from "@1io/compare";
import { CID } from "multiformats";
import { getCoState, resolveCid } from "tauri-plugin-co-sdk";
import { buildCoCoreId } from "./core-id.js";

export async function invokeResolveCid(co: string, cid: CID): Promise<any> {
    return await resolveCid(co, cid);
}

export async function invokeGetCoState(co: string): Promise<any | undefined> {
    let [stateCid] = await getCoState(co);
    return stateCid ? await invokeResolveCid(co, stateCid) : undefined;
}

export async function invokeGetCoHeads(co: string): Promise<any[]> {
    let [_, headsCids] = await getCoState(co);
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
        return core_state;
    }
    return null;
}

export async function invokeGetCoIds() {
    const memberships = await invokeGetCoreState("local", "membership");
    if (Array.isArray(memberships?.memberships)) {
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
    const foundCores: string[] = [];
    for (const coId of coIds) {
        const state = await invokeGetCoState(coId);
        for (let [key, value] of Object.entries(state.cores)) {
            // TODO remove any cast when js interfaces are done
            const v = value as any;
            if (v?.tags) {
                if (v.tags[0].every((tag: string) => tags.includes(tag))) {
                    foundCores.push(buildCoCoreId(coId, key));
                }
            }
        }
    }
    return foundCores;
}

