import React from "react";
import { MembershipState } from "../../types/index.js";

// TODO: type for memberships state
/**
 * Hook that resolves to all Co ids that can be found as an active membership in the given membership state
 */
export function useCoIds(membershipsState: any): string[] | undefined {
  const [coIds, setCoIds] = React.useState();
  React.useEffect(() => {
    if (Array.isArray(membershipsState?.memberships)) {
      setCoIds(
        membershipsState.memberships
          // only get joined COs
          .filter((membership: any) => membership?.membership_state === MembershipState.Active)
          .map((membership: any) => membership?.id)
          .filter((i: any) => i !== undefined && i !== null),
      );
    }
  }, [membershipsState]);
  return coIds;
}
