import { CID } from "multiformats";
import React from "react";
import { resolveCid } from "../../../../../dist-js/index.js";

export function useCoCore(coCid: CID | undefined, coreId: string, session: string | undefined): CID | undefined {
  const [coreState, setCoreState] = React.useState<CID | undefined>(undefined);
  React.useEffect(() => {
    let canceled = false;
    async function getCoreCid() {
      if (session !== undefined && coCid !== undefined && !canceled) {
        const resolvedCoState = await resolveCid(session, coCid);
        const coreCid = resolvedCoState?.c?.[coreId]?.state;
        if (coreCid !== undefined && coreCid !== null) {
          setCoreState(coreCid);
        }
      }
    }
    getCoreCid();
    return () => {
      canceled = true;
    };
  }, [coCid?.toString(), coreId, session]);
  return coreState;
}
