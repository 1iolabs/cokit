export function buildCoCoreId(co: string, core: string) {
    return `${co}/${core}`;
}

export function splitCoCoreId(coCoreId: string): [string, string | undefined] {
    const splitString = coCoreId.split("/");
    if (splitString.length === 2 && splitString[0] && splitString[1]) {
        return [splitString[0], splitString[1]];
    }
    return [coCoreId, undefined];
}
