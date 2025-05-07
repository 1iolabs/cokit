import { createPluginErroredAction } from "@1io/kui-application-sdk";
import { filter, identity, map, take } from "rxjs";
import { GroupViewParticipantInvitedAction, GroupViewPluginActionType } from "../actions/index.js";
import { GroupViewEpicType } from "../types/plugin.js";

export const addOwnIdentityEpic: GroupViewEpicType = (_, state$, context) => state$.pipe(
    map((state) => state?.chatsListState),
    // wait until public state is loaded
    filter((state) => state !== undefined),
    take(1),
    map((state) => {
        if (state?.identity === undefined) {

            return createPluginErroredAction(
                { name: "No identity error", message: "CHatslist plugin state is missing own identity" },
                context.plugin,
                context.pluginTags,
            );
        }
        return identity<GroupViewParticipantInvitedAction>({
            payload: { participant: state.identity },
            type: GroupViewPluginActionType.ParticipantInvited,
        });
    }),
);
