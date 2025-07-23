import { TagList, tagValue } from "@1io/kui-application-sdk";
import React from "react";
import { MessengerViewContainer } from "./components/chat-view.js";
import { messengerViewEpic } from "./epics/index.js";
import { MessengerViewPlugin } from "./types/plugin.js";
import { CoreIdTag } from "./types/tags.js";

export default function plugin(pluginTags: TagList): MessengerViewPlugin {
  const coCoreId = tagValue<CoreIdTag>(pluginTags, "coreId")?.split("/");
  // default values
  let coId = "1io";
  let coreId = "room";
  if (coCoreId?.length === 2) {
    // get values fromt tag
    coId = coCoreId[0]!;
    coreId = coCoreId[1]!;
  }
  return {
    context: (context) => context,
    epic: messengerViewEpic,
    reducer: () => ({ coId, coreId }),
    render: (renderTags, props) => {
      return <MessengerViewContainer {...props} />;
    },
    tags: [{ key: "type", value: "coapp-messenger-view" }, ...pluginTags],
  };
}
