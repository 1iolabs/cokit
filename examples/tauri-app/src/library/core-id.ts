export function buildCoCoreId(co: string, core: string) {
    return `${co}/${core}`;
}

export function splitCoCoreId(coreId: string): [string, string] {
    const splitString = coreId.split("/");
    if (splitString.length === 2 && splitString[0] && splitString[1]) {
        return [splitString[0], splitString[1]];
    }
    throw Error("Couldn't split to co and core ids");
}
