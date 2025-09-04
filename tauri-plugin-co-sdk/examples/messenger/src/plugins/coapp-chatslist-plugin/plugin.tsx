import { COApp1ioClassName } from "@1io/coapp-style";
import { TagList } from "@1io/kui-application-sdk";
import { clsx } from "clsx";
import React from "react";
import { coappChatsListApi } from "./api/index.js";
import { ChatListViewContainer } from "./containers/chat-list-view.js";
import { chatsListEpic } from "./epics/index.js";
import { chatsListReducer } from "./reducers/index.js";
import { ChatsListPlugin, coappChatsListPluginId } from "./types/plugin.js";

document.body.className = clsx(document.body.className, COApp1ioClassName);

export default function plugin(pluginTags: TagList): ChatsListPlugin {
  return {
    api: coappChatsListApi,
    context: (context) => context,
    epic: chatsListEpic,
    reducer: chatsListReducer,
    render: () => <ChatListViewContainer />,
    state: (state) => ({ identity: state.identity }),
    tags: [{ key: "type", value: coappChatsListPluginId }, ...pluginTags],
  };
}
