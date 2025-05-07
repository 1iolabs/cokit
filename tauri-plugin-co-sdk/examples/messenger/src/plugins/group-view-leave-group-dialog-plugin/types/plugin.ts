import { Plugin, RenderPlugin } from "@1io/kui-application-sdk";
import { DialogProps } from "../../../types/dialog-container-props.js";
import { LeaveGroupDialogActions } from "../actions/index.js";

export type LeaveGroupDialogPlugin = Plugin<{}, LeaveGroupDialogActions> & RenderPlugin<DialogProps>;
