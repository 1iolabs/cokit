import { CID } from "multiformats";
import React from "react";
import { GetActionsResponse, getActions } from "../../invoke-utils.js";

export function useCoCoreActions(
  co: string,
  core: string,
  heads: CID[] | undefined,
  session: string | undefined,
  count: number,
) {
  const [actions, setActions] = React.useState<GetActionsResponse>();
  React.useEffect(() => {
    async function getCoreActions() {
      console.log("get actions");
      if (heads !== undefined && session !== undefined) {
        setActions(await getActions(session, heads, count, undefined));
      }
    }
    getCoreActions();
  }, [co, core, heads, session, count]);
  return actions;
}
