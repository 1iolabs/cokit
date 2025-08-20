import { Event, listen } from "@tauri-apps/api/event";
import { CID } from "multiformats";
import { fromEventPattern, Observable } from "rxjs";
import { decode } from "@ipld/dag-cbor";
export type CoSdkStateEvent = [string, CID | undefined, CID[]];

/**
 * Starts listening to the "co-sdk-new-state" event channel and emits all events that are
 * received this way in the returned observable.
 */
export function createCoSdkStateEventListener(): Observable<CoSdkStateEvent> {
  const retObservable = fromEventPattern<CoSdkStateEvent>(
    async (handler) => {
      // start listening to state changes from co sdk
      const decodeAndCall = (event: Event<number[]>) => {
        console.log("event", event.payload);
        const data: [string, CID | undefined, CID[]] = decode(Uint8Array.from(event.payload));
        handler(data);
      };
      return await listen("co-sdk-new-state", decodeAndCall);
    },
    async (_handler, unlistenPromise) => {
      // stop listening
      const unlisten = await unlistenPromise;
      unlisten();
    },
  );
  return retObservable;
}
