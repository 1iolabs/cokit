import { Tag } from "@1io/kui-application-sdk";
const coreIdTagKey = "coCoreId";
// value expected in the form of "${co}/${core}"
export type CoCoreIdTag = Tag<typeof coreIdTagKey, string>;
