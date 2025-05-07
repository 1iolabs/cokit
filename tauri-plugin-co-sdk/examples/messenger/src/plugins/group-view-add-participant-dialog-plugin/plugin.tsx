import { reducerWithInitialState, TagList, WellKnownTags } from "@1io/kui-application-sdk";
import React from "react";
import { AddParticipantsDialogContainer } from "./components/add-participant.js";
import { addParticipantsDialogReducer } from "./reducers/index.js";
import { AddParticipantDialogPlugin, addParticipantDialogPluginId } from "./types/plugin.js";

export default function plugin(pluginTags: TagList): AddParticipantDialogPlugin {
    return {
        render: (_, renderProps) => { return <AddParticipantsDialogContainer {...renderProps} /> },
        reducer: reducerWithInitialState(addParticipantsDialogReducer, { did: "" }),
        tags: [...pluginTags, { key: WellKnownTags.Type, value: addParticipantDialogPluginId }],
    };
}
