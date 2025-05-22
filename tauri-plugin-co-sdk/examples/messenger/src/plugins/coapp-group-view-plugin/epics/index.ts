import { combineEpics } from "@1io/kui-application-sdk";
import { GroupViewEpicType } from "../types/plugin.js";
import { addOwnIdentityEpic } from "./add-own-identity.js";
import { openInviteParticipantDialogEpic, openRemoveParticipantDialogEpic, participantAddedEpic, participantRemovedEpic } from "./dialog.js";
import { initializeEpic } from "./init.js";
import { loadProfilePicEpic } from "./load-profile-pic.js";
import { submitEpic } from "./save.js";

export const groupViewPluginEpic: GroupViewEpicType = combineEpics(
    loadProfilePicEpic,
    initializeEpic,
    openInviteParticipantDialogEpic,
    participantAddedEpic,
    openRemoveParticipantDialogEpic,
    participantRemovedEpic,
    addOwnIdentityEpic,
    submitEpic,
);
