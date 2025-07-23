import { reducerWithInitialState, TagList, WellKnownTags } from "@1io/kui-application-sdk";
import React from "react";
import { AddParticipantsDialogContainer } from "./components/add-participant.js";
import { addParticipantsDialogReducer } from "./reducers/index.js";
import { AddParticipantDialogPlugin, addParticipantDialogPluginId } from "./types/plugin.js";

export default function plugin(pluginTags: TagList): AddParticipantDialogPlugin {
  return {
    reducer: reducerWithInitialState(addParticipantsDialogReducer, { did: "" }),
    render: (_, renderProps) => {
      return <AddParticipantsDialogContainer {...renderProps} />;
    },
    tags: [...pluginTags, { key: WellKnownTags.Type, value: addParticipantDialogPluginId }],
  };
}
