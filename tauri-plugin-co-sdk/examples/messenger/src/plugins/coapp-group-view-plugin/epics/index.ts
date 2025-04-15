import { combineEpics } from "@1io/kui-application-sdk";
import { GroupViewEpicType } from "../types/plugin.js";
import { initializeEpic } from "./init.js";
import { loadProfilePicEpic } from "./load-profile-pic.js";

export const groupViewPluginEpic: GroupViewEpicType = combineEpics(
    loadProfilePicEpic,
    initializeEpic,
);
