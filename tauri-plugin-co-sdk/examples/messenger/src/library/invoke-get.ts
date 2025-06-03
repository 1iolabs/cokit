import { isNonNull } from "@1io/compare";
import { getCoState, resolveCid, sessionClose, sessionOpen } from "tauri-plugin-co-sdk";
import { buildCoCoreId } from "./core-id.js";

export async function getResolvedCoState(sessionId: string, co: string): Promise<any | undefined> {
    let [stateCid] = await getCoState(co);
    return stateCid ? await resolveCid(sessionId, stateCid) : undefined;
}

export async function getCoHeads(sessionId: string, co: string): Promise<any[]> {
    let [_, headsCids] = await getCoState(co);
    const heads = [];
    for (const headCid of headsCids) {
        heads.push(await resolveCid(sessionId, headCid));
    }
    return heads;
}

export async function getCoreState(co: string, core: string, externalSessionId?: string): Promise<any> {
    let sessionId = externalSessionId ?? await sessionOpen(co);
    const state = await getResolvedCoState(sessionId, co);
    const core_cid = state?.cores?.[core]?.state;
    if (core_cid) {
        let core_state = await resolveCid(sessionId, core_cid);
        return core_state;
    }
    return null;
}

export async function getCoIds(): Promise<ReadonlyArray<string>> {
    const localCoSessionId = await sessionOpen("local");
    const memberships = await getCoreState("local", "membership", localCoSessionId);
    sessionClose(localCoSessionId);
    if (Array.isArray(memberships?.memberships)) {
        return memberships.memberships
            // only get joined COs
            .filter((membership: any) => membership?.membership_state === 0)
            .map((membership: any) => membership?.id)
            .filter(isNonNull);
    }
    return [];
}

export async function getInvitedCoIds(): Promise<ReadonlyArray<string>> {
    const localCoSessionId = await sessionOpen("local");
    const memberships = await getCoreState("local", "membership", localCoSessionId);
    sessionClose(localCoSessionId);
    if (Array.isArray(memberships?.memberships)) {
        return memberships.memberships
            // only get joined COs
            .filter((membership: any) => membership?.membership_state === 2)
            .map((membership: any) => membership?.id)
            .filter(isNonNull);
    }
    return [];
}

export async function getFilteredCoreIds(tags: string[], co?: string): Promise<string[]> {
    const coIds: string[] = [];
    if (co !== undefined) {
        coIds.push(co);
    } else {
        coIds.push(...(await getCoIds()));
    }
    const foundCores: string[] = [];
    for (const coId of coIds) {
        let sessionId = await sessionOpen(coId);
        const state = await getResolvedCoState(sessionId, coId);
        for (let [key, value] of Object.entries(state.cores)) {
            // TODO remove any cast when js interfaces are done
            const v = value as any;
            if (v?.tags) {
                if (v.tags[0].every((tag: string) => tags.includes(tag))) {
                    foundCores.push(buildCoCoreId(coId, key));
                }
            }
        }
        await sessionClose(sessionId);
    }
    return foundCores;
}

