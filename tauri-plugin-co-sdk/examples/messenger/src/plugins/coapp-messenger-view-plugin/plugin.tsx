import { reducerWithInitialState, TagList, tagValue } from "@1io/kui-application-sdk";
import React from "react";
import { MessengerViewContainer } from "./components/chat-view.js";
import { messengerViewEpic } from "./epics/index.js";
import { messengerViewReducer } from "./reducers/index.js";
import { MessengerViewPlugin } from "./types/plugin.js";
import { CoreIdTag } from "./types/tags.js";


export default function plugin(pluginTags: TagList): MessengerViewPlugin {
    const coreId = tagValue<CoreIdTag>(pluginTags, "coreId")?.split("/");
    // default values
    let co = "1io";
    let core = "room";
    if (coreId?.length === 2) {
        // get values fromt tag
        co = coreId[0]!;
        core = coreId[1]!;
    }
    return {
        context: (context) => context,
        epic: messengerViewEpic,
        reducer: reducerWithInitialState(messengerViewReducer, { chatName: "", co, core, messages: [] }),
        render: (renderTags, props) => {
            return <MessengerViewContainer {...props} />;
        },
        tags: [{ key: "type", value: "coapp-messenger-view" }, ...pluginTags],

    };
}
