import { createPluginErroredAction, isPluginInitializeAction, tagValue, WellKnownTags } from "@1io/kui-application-sdk";
import { Action } from "redux";
import { filter, identity, mergeAll, mergeMap } from "rxjs";
import DefaultAvatar from "../../../assets/Users_48.svg";
import { splitCoCoreId } from "../../../library/core-id.js";
import { coappChatsListPluginId } from "../../coapp-chatslist-plugin/types/plugin.js";
import {
  GroupViewParticipantAddedAction,
  GroupViewPluginActionType,
  GroupViewSetAvatarAction,
  GroupViewSetNameAction,
} from "../actions/index.js";
import { GroupViewEpicType } from "../types/plugin.js";
import { GroupViewPluginRoomCoreIdTag } from "../types/tag.js";
import { getCoreState, getResolvedCoState } from "../../../../../../dist-js";

export const initializeEpic: GroupViewEpicType = (action$, _, context) =>
  action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async () => {
      const actions: Action[] = [
        context.api.subscribeState([{ key: WellKnownTags.Type, value: coappChatsListPluginId }], "chatsListState"),
      ];
      const roomCoreId = tagValue<GroupViewPluginRoomCoreIdTag>(context.pluginTags, "roomCoreId");
      // edit mode => load current data
      if (roomCoreId !== undefined) {
        const result = splitCoCoreId(roomCoreId);
        if (result === undefined) {
          // Error!
          return [
            createPluginErroredAction(
              {
                message: "Couldn't resolve ids from CoCoreId",
                name: "Invalid CoCOreId error",
              },
              context.plugin,
              context.pluginTags,
            ),
          ];
        }
        // get core state and cancel if failed
        const coreState = await getCoreState(result.coId, result.coreId);
        if (!coreState) {
          // Error!
          return [
            createPluginErroredAction(
              {
                message: "Couldn't fetch core state",
                name: "Invalid Core state error",
              },
              context.plugin,
              context.pluginTags,
            ),
          ];
        }
        // set information
        actions.push(
          identity<GroupViewSetNameAction>({
            payload: { name: coreState.name },
            type: GroupViewPluginActionType.SetName,
          }),
          // TODO avatar
          identity<GroupViewSetAvatarAction>({
            payload: { avatar: DefaultAvatar },
            type: GroupViewPluginActionType.SetAvatar,
          }),
        );
        // set participants
        const coState = await getResolvedCoState(result.coId);
        if (coState?.participants === undefined) {
          // Error!
          throw new Error("Couldn't fetch CO core participants state: " + coState);
        }
        for (const participant in coState.participants) {
          if (participant !== undefined) {
            actions.push(
              identity<GroupViewParticipantAddedAction>({
                payload: { participant },
                type: GroupViewPluginActionType.ParticipantAdded,
              }),
            );
          }
        }
      } else {
        // TODO new mode
      }
      return actions;
    }),
    mergeAll(),
  );
