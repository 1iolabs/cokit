// import { Action } from "redux";
// import { pushAction, sessionClose, sessionOpen } from "../../../../../../dist-js/index.js";

// async function handleLocalCoAction(action: any, ownIdentity: string): Promise<Action[]> {
//   if (action.c === "membership") {
//     if (action.p?.Join !== undefined) {
//       // only continue if we are the invited did
//       if (action.p.Join.did !== ownIdentity) {
//         return [];
//       }
//       // only continue if the join action is on invite state
//       if (action.p.Join.membership_state !== 2) {
//         return [];
//       }
//       // accept join action
//       const joinAction = {
//         ChangeMembershipState: {
//           did: ownIdentity,
//           id: action.p.Join.id,
//           membership_state: 3,
//         },
//       };
//       console.log("accept action", joinAction);
//       try {
//         const session = await sessionOpen("local");
//         await pushAction(session, "membership", joinAction, ownIdentity);
//         await sessionClose(session);
//       } catch (e) {
//         console.log(e);
//       }
//     }
//     if (action.p?.ChangeMembershipState !== undefined) {
//       const changeMemberAction = action.p?.ChangeMembershipState;

//       // only actions that added us are relevant
//       if (changeMemberAction.did !== ownIdentity) {
//         return [];
//       }

//       // only continue if join accepted
//       if (changeMemberAction.membership_state !== 0) {
//         return [];
//       }
//     }
//   }
//   return [];
// }
