import { combineEpics } from "@1io/kui-application-sdk";
import { MessengerViewEpicType } from "../types/plugin.js";
import { initEpic } from "./init.js";
import { sendEpic } from "./send.js";

export const messengerViewEpic: MessengerViewEpicType = combineEpics<MessengerViewEpicType>(initEpic, sendEpic);
