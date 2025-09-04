import { CID } from "multiformats";
import React from "react";
import { createCoSdkStateEventListener } from "./co-sdk-state-listener.js";
import { getCoState } from "../../../../dist-js/index.js";

export function useCo(co: string): [CID | undefined, CID[] | undefined] {
  const [coState, setCoState] = React.useState<[CID | undefined, CID[]]>();

  React.useEffect(() => {
    // get initial state
    async function loadInitState() {
      const initState = await getCoState(co);
      setCoState(initState);
    }
    loadInitState();

    // get updated state when new event is triggered
    const coSdkEventSubscription = createCoSdkStateEventListener().subscribe({
      next: (event) => {
        const [coId, state, heads] = event;
        if (co === coId) {
          setCoState([state, heads]);
        }
      },
    });
    return () => {
      coSdkEventSubscription.unsubscribe();
    };
  }, [co]);
  if (coState !== undefined) {
    return coState;
  }
  return [undefined, undefined];
}
