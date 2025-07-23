import { Plugin, RenderPlugin, Tag, TagList } from "@1io/kui-application-sdk";
import { DialogProps } from "../../../types/dialog-container-props.js";
import { RemoveParticipantDialogActions } from "../actions/index.js";

export const removeParticipantDialogPluginId = "coapp-remove-participant";

export type RemoveParticipantDialogPlugin = Plugin<{}, RemoveParticipantDialogActions> & RenderPlugin<DialogProps>;

export type RemoveParticipantDialogDidTag = Tag<"did">;
export type RemoveParticipantDialogGroupNameTag = Tag<"groupName">;
export type RemoveParticipantDialogRequiredTags = TagList<
  RemoveParticipantDialogDidTag | RemoveParticipantDialogGroupNameTag
>;
