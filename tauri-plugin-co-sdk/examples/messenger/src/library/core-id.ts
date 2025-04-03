export function buildCoCoreId(co: string, core: string) {
    return `${co}/${core}`;
}

interface CoCoreResult {
    coreId: string;
    coId: string;
}

/**
 * 
 * @param coCoreId CoCoreId built with {@link buildCoCoreId} function
 * @returns A {@link CoCoreResult} if splitting was successful. Undefined otherwise
 */
export function splitCoCoreId(coCoreId: string): CoCoreResult | undefined {
    const splitString = coCoreId.split("/");
    if (splitString.length === 2 && splitString[0] && splitString[1]) {
        return { coId: splitString[0], coreId: splitString[1] };
    }
    return undefined;
}
