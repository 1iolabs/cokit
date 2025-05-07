import { Tag, WellKnownTags } from "@1io/kui-application-sdk";

export const leaveGroupDialogPluginId = "coapp-leave-group";

export const leaveGroupDialogPluginTypeTag: Tag = { key: WellKnownTags.Type, value: leaveGroupDialogPluginId };
export type LeaveGroupDialogGroupNameTag = Tag<"groupName", string>;
