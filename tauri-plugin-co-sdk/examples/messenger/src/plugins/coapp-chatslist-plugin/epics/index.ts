import { combineEpics } from "@1io/kui-application-sdk";
import { ChatsListEpicType } from "../types/plugin.js";
import { copyIdentityEpic } from "./copy-identity.js";
import { groupDetailsEpic } from "./group-details.js";
import { initEpic } from "./init.js";
import { loadMessengerIdentityEpic } from "./load-chats.js";
import { loadDialogEpic } from "./load-dialog.js";
import { openChatEpic } from "./open-chat.js";

export const chatsListEpic: ChatsListEpicType = combineEpics<ChatsListEpicType>(
  initEpic,
  loadMessengerIdentityEpic,
  openChatEpic,
  groupDetailsEpic,
  loadDialogEpic,
  copyIdentityEpic,
);
