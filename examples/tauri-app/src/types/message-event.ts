export interface RoomCoreEvent {
    f: string;
    c: string;
    t: number;
    p: {
        event_id: string;
        timestamp: number;
        room_id: string;
        type: string;
        content: {
            msgtype: string;
            body: string;
            name: string;
        };
    };
}
