import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { EMPTY, filter, identity, mergeAll, mergeMap } from "rxjs";
import { GroupViewPluginActionType, GroupViewSetAvatarAction } from "../actions/index.js";
import { GroupViewEpicType } from "../types/plugin.js";

export const loadProfilePicEpic: GroupViewEpicType = (actions$, state$, context) => actions$.pipe(
    filter((action) => action.type === GroupViewPluginActionType.LoadProfilePicEpic),
    mergeMap(async () => {
        let avatar = await open({
            directory: false,
            multiple: false,
            title: "Choose new profile picture",
            filters: [{ name: "Images only", extensions: ["png", "svg", "jpg"] }],
        });
        if (avatar === null) {
            // Selection canceled
            return EMPTY;
        }
        avatar = convertFileSrc(avatar);
        console.log("test file select", avatar);
        return [identity<GroupViewSetAvatarAction>({
            payload: { avatar },
            type: GroupViewPluginActionType.SetAvatar,
        })];
    }),
    mergeAll(),
);
