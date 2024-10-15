import { MatrixEvent } from "./types";

export interface RoomCoreEvent {
    f: string;
    c: string;
    t: number;
    p: MatrixEvent;
}
