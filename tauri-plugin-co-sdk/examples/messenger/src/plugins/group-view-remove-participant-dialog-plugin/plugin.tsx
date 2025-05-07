import { enforceTag, TagList, WellKnownTags } from "@1io/kui-application-sdk";
import React from "react";
import { DialogProps } from "../../types/dialog-container-props.js";
import { RemoveParticipantDialogContainer } from "./components/remove-participant.js";
import { RemoveParticipantDialogDidTag, RemoveParticipantDialogGroupNameTag, RemoveParticipantDialogPlugin, removeParticipantDialogPluginId } from "./types/plugin.js";

export default function plugin(pluginTags: TagList): RemoveParticipantDialogPlugin {
    const did = enforceTag<RemoveParticipantDialogDidTag>(pluginTags, "did");
    const groupName = enforceTag<RemoveParticipantDialogGroupNameTag>(pluginTags, "groupName");
    return {
        render: (_, renderProps: DialogProps) => <RemoveParticipantDialogContainer {...renderProps} did={did} groupName={groupName} />,
        reducer: () => ({}),
        tags: [...pluginTags, { key: WellKnownTags.Type, value: removeParticipantDialogPluginId }],
    };
}
