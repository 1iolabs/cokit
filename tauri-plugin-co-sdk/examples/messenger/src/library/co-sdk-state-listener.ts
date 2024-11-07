import { Event, listen } from "@tauri-apps/api/event";
import { CID } from "multiformats";
import { fromEventPattern, Observable } from "rxjs";

export type CoSdkStateEvent = Event<[string, CID | undefined, Set<CID>]>;

/**
 * Starts listening to the "co-sdk-new-state" event channel and emits all events that are 
 * received this way in the returned observable.
 */
export function createCoSdkStateEventListener(): Observable<CoSdkStateEvent> {
    const retObservable = fromEventPattern<CoSdkStateEvent>(
        async (handler) => {
            // start listening to state changes from co sdk
            return await listen<CoSdkStateEvent>("co-sdk-new-state", handler);
        },
        (_handler, unlisten) => {
            //stop listening
            console.log("unlisten: ", unlisten);
            unlisten();
        },
    );
    return retObservable;
}
