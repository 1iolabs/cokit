import { invoke } from "@tauri-apps/api/core";
import { Event, listen } from "@tauri-apps/api/event";
import { Action } from "redux";
import { fromEventPattern, Observable } from "rxjs";
import { buildCoCoreId, splitCoCoreId } from "./core-id";

export type SubscriptionEventHandler<E> = (event: Event<E>) => Action[];

/**
 * Subscribes to changes in the given CO/CORE or CO if core is undefined and starts emitting events which 
 * are piped to the given handler method. Creates and returns an Observable emitting the received events. 
 * Subscription stops when the returned Observable is stopped.
 * 
 * @param source Some unique identifier to discern the source of the subscription, e.g. plugin ID
 * @param co ID of the CO the subscription is for
 * @param core Optional ID of a CORE. Will only emit events of that core when set
 * @returns An Observable that emits any received events
 */
export function createTauriSubscription<E>(
    source: string,
    coCoreId: string,
): Observable<Event<E>> {
    const [co, core] = splitCoCoreId(coCoreId);
    // subscribe to "co/core" event
    invoke("subscribe", { co, core, source });
    const retObservable = fromEventPattern<Event<E>>(
        async (handler) => {
            // start listening on co/core or co if core is undefined
            return await listen(core ? buildCoCoreId(co, core) : co, handler);
        },
        (_handler, unlisten) => {
            // unsubscribe and stop listening
            console.log("unsub", co, core);
            invoke("unsubscribe", { co, core, source });
            unlisten();
        },
    );
    return retObservable;
}
