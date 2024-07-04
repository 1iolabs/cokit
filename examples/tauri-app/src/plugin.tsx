import { MessengerPlugin } from "./types/plugin.js";
import { App } from "./components/App.js";
import React from "react";
import { TagList } from "@1io/kui-application-sdk";
import { messengerEpic } from "./epics/index.js";
import { messengerReducer } from "./reducers/index.js";


export default function plugin(pluginTags: TagList): MessengerPlugin {
    return {
        context: (context) => context,
        epic: messengerEpic,
        reducer: messengerReducer,
        render: (renderTags) => {
            return <App />;
        },
        tags: [{ key: "type", value: "coapp-messenger" }, ...pluginTags],

    };
}
