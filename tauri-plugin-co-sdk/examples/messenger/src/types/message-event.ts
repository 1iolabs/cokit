import { Messaging } from "../../../../dist-js/index.js";

export interface RoomCoreEvent {
    f: string;
    c: string;
    t: number;
    p: Messaging.MatrixEvent;
}
