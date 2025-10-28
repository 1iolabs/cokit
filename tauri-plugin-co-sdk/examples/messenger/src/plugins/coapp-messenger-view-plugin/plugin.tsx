import { enforcePickTag, TagList } from "@1io/kui-application-sdk";
import React from "react";
import { MessengerViewContainer } from "./components/chat-view.js";
import { messengerViewEpic } from "./epics/index.js";
import { MessengerViewPlugin } from "./types/plugin.js";
import { CoCoreIdTag } from "./types/tags.js";
import { splitCoCoreId } from "../../library/core-id.js";

export default function plugin(pluginTags: TagList): MessengerViewPlugin {
  const coCoreIdTag = enforcePickTag<CoCoreIdTag>(pluginTags, "coCoreId").value;
  const coCoreId = splitCoCoreId(coCoreIdTag);

  if (coCoreId === undefined) {
    throw new Error("Wrong tag coCoreId: " + coCoreIdTag.toString());
  }
  return {
    context: (context) => context,
    epic: messengerViewEpic,
    reducer: () => ({ coId: coCoreId.coId, coreId: coCoreId.coreId }),
    render: (renderTags, props) => {
      return <MessengerViewContainer {...props} />;
    },
    tags: [{ key: "type", value: "coapp-messenger-view" }, ...pluginTags],
  };
}
