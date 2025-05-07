import { enforceTag, TagList } from "@1io/kui-application-sdk";
import React from "react";
import { LeaveGroupDialogContainer } from "./components/leave-group.js";
import { LeaveGroupDialogPlugin } from "./types/plugin.js";
import { LeaveGroupDialogGroupNameTag, leaveGroupDialogPluginTypeTag } from "./types/tag.js";

export default function plugin(pluginTags: TagList): LeaveGroupDialogPlugin {
    const groupName = enforceTag<LeaveGroupDialogGroupNameTag>(pluginTags, "groupName");
    return {
        reducer: () => ({}),
        render: (_, renderProps) => <LeaveGroupDialogContainer {...renderProps} groupName={groupName} />,
        tags: [...pluginTags, leaveGroupDialogPluginTypeTag],
    };
}
