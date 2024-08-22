import { TagList } from "@1io/kui-application-sdk";
import React from "react";
import { MessengerViewContainer } from "./components/chat-view.js";
import { messengerViewEpic } from "./epics/index.js";
import { messengerViewReducer } from "./reducers/index.js";
import { MessengerViewPlugin } from "./types/plugin.js";


export default function plugin(pluginTags: TagList): MessengerViewPlugin {
    return {
        context: (context) => context,
        epic: messengerViewEpic,
        reducer: messengerViewReducer,
        render: (renderTags) => {
            return <MessengerViewContainer />;
        },
        tags: [{ key: "type", value: "coapp-messenger-view" }, ...pluginTags],

    };
}
