import { MatrixEvent } from "./matrix-event.js";

export interface RoomCoreEvent {
    f: string;
    c: string;
    t: number;
    p: MatrixEvent;
}
