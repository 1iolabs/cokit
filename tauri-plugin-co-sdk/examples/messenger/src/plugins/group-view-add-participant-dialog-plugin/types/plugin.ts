import { Plugin, RenderPlugin } from "@1io/kui-application-sdk";
import { DialogProps } from "../../../types/dialog-container-props.js";
import { AddParticipantDialogActions } from "../actions/index.js";
import { AddParticipantDialogPluginState } from "./state.js";

export const addParticipantDialogPluginId = "coapp-add-participant";

export type AddParticipantDialogPlugin = Plugin<AddParticipantDialogPluginState, AddParticipantDialogActions> &
  RenderPlugin<DialogProps>;
