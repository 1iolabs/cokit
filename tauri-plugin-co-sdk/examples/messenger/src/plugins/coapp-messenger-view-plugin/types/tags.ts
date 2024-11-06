import { Tag } from "@1io/kui-application-sdk";
const coreIdTagKey = "coreId";
// value expected in the form of "${co}/${core}"
export type CoreIdTag = Tag<typeof coreIdTagKey, string>; 
