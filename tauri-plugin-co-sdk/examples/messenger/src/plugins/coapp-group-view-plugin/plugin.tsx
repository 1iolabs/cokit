import { reducerWithInitialState, TagList, tagValue } from "@1io/kui-application-sdk";
import React from "react";
import { GroupViewContainer, GroupViewContainerProps } from "./components/index.js";
import { groupViewPluginEpic } from "./epics/index.js";
import { groupViewPluginReducer } from "./reducers/index.js";
import { GroupViewPlugin } from "./types/plugin.js";
import { GroupViewPluginRoomCoreIdTag, groupViewPluginTag } from "./types/tag.js";

export default function plugin(pluginTags: TagList): GroupViewPlugin {
  const roomCoreId = tagValue<GroupViewPluginRoomCoreIdTag>(pluginTags, "roomCoreId");
  return {
    epic: groupViewPluginEpic,
    reducer: reducerWithInitialState(groupViewPluginReducer, {
      isNew: roomCoreId === undefined,
      name: "New group",
      participants: [],
    }),
    render: (_, props: GroupViewContainerProps) => <GroupViewContainer {...props} />,
    tags: [groupViewPluginTag, ...pluginTags],
  };
}
