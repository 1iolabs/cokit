import { combineEpics } from "@1io/kui-application-sdk";
import { GroupViewEpicType } from "../types/plugin.js";

export const groupViewPluginEpic: GroupViewEpicType = combineEpics();
