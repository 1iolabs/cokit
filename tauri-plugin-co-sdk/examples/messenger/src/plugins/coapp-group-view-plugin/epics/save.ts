import { createPluginErroredAction } from "@1io/kui-application-sdk";
import { readFile } from "@tauri-apps/plugin-fs";
import moment from "moment";
import { CID } from "multiformats";
import { EMPTY, filter, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import * as uuid from "uuid";
import { Cores, createCo, Messaging, pushAction, sessionClose, sessionOpen } from "../../../../../../dist-js/index.js";
import { GroupViewPluginActionType, GroupViewSubmitAction } from "../actions/index.js";
import { GroupViewEpicType } from "../types/plugin.js";

export const submitEpic: GroupViewEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is GroupViewSubmitAction => action.type === GroupViewPluginActionType.Submit),
    withLatestFrom(state$),
    mergeMap(async ([, state]) => {
        if (state.chatsListState?.identity === undefined) {
            // can only create and edit with a loaded identity
            return [createPluginErroredAction({ name: "Save failed: ", message: "No identity" }, context.plugin, context.pluginTags)];
        }
        if (state.isNew) {
            // create CO
            let co_id;
            try {
                co_id = await createCo(state.chatsListState.identity, state.name, false);
            } catch (err) {
                console.error("Save failed: ", err);
                return [createPluginErroredAction({ name: "Save failed: ", message: "Couldn't create CO" }, context.plugin, context.pluginTags)];
            }

            // open session with co id
            const session = await sessionOpen(co_id);

            // add room core
            const roomCoreWasmCid = Cores.Cores["co-core-room"];
            const createRoomCoreAction = {
                CoreCreate: {
                    core: "room",
                    binary: CID.parse(roomCoreWasmCid),
                    tags: [["core", "co-core-room"]],
                }
            };
            const cid = await pushAction(session, "co", createRoomCoreAction, state.chatsListState.identity);
            console.log("pushed", cid);

            // save room core name
            const setNameAction: Messaging.MatrixEvent = {
                event_id: uuid.v4(),
                content: { name: state.name },
                room_id: "room",
                timestamp: moment.now(),
                type: "room_name",
            };
            await pushAction(session, "room", setNameAction, state.chatsListState.identity);

            // save room core image
            if (state.avatar) {
                const file = await readFile(state.avatar);
                console.log(file);
            }

            // invite participants
            for (const participant of state.participants) {
                const inviteAction = {
                    ParticipantInvite: {
                        participant,
                        tags: [],
                    },
                };
                await pushAction(session, "co", inviteAction, state.chatsListState.identity);
            }

            // close session
            await sessionClose(session);
        } else {
            // save changes to existing room core
        }
        return EMPTY;
    }),
    mergeAll(),
);
