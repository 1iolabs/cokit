import { isNonNull } from "@1io/compare";
import React from "react";

// TODO: type for memberships state
export function useCoIds(membershipsState: any): string[] {
  const [coIds, setCoIds] = React.useState([]);
  React.useEffect(() => {
    if (Array.isArray(membershipsState?.memberships)) {
      setCoIds(
        membershipsState.memberships
          // only get joined COs
          .filter((membership: any) => membership?.membership_state === 10)
          .map((membership: any) => membership?.id)
          .filter(isNonNull),
      );
    }
  }, [membershipsState]);
  return coIds;
}
