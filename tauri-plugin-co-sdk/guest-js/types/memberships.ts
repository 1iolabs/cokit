import { CID } from "multiformats";
import { CoId, Did, Tags } from "./co.js";

export enum MembershipState {
  /// Active membership.
  Active = 10,

  /// Pending join by us.
  ///
  /// Use Cases:
  /// - This is a pending join triggered by an invite waiting for completion.
  /// - This is waiting for CO participant acception/rejection (remote).
  ///
  /// Related membership Tags:
  ///  `co-invite: CoInviteMetadata`
  ///  `join-date: Date`
  Join = 20,

  /// Pending invite by some participant of the CO.
  ///
  /// Use Cases:
  /// - This is waiting for our acception/rejection.
  /// - Accept invite by change membership state to [`MembershipState::Join`].
  /// - Reject invite by removing the membership using [`MembershipsAction::Remove`].
  ///
  /// Related membership Tags:
  ///  `co-invite: CoInviteMetadata`
  Invite = 30,

  /// Inactive membership.
  Inactive = 40,
}

/**
 * JSON wire format of the Rust struct:
 *
 * pub struct Membership {
 *   id: CoId,
 *   did: Did,
 *   state: BTreeSet<CoState>,
 *   key: Option<String>,
 *   membership_state: MembershipState,
 *   tags: Tags,
 * }
 */
export interface Membership {
  /** CO Unique Identifier */
  id: CoId;

  /** The DID used for the membership */
  did: Did;

  /**
   * currently not needed
   */
  state: any;

  /**
   * Optional encryption key URI.
   * Rust: Option<String> → optional property
   */
  key?: string;

  /** Membership state */
  membership_state: MembershipState;

  /** Membership tags */
  tags: Tags;
}

export interface Memberships {
  /** List of membership entries */
  memberships: Membership[];
}

export type MembershipsAction =
  /// Join a Co. The membership state indicates if it was an invite from someone.
  | { Join: { Membership: Membership } }
  | {
      Update: {
        id: CoId;
        state: CID;
        /// Remove all [`CoState`] which heads are fully covered.
        remove: CID[];
      };
    }
  | {
      ChangeMembershipState: {
        id: CoId;
        did: Did;
        membership_state: MembershipState;
      };
    }
  /// Change the active encryption key reference which is used the read the current heads/state.
  | {
      ChangeKey: {
        id: CoId;
        did: Did;
        key: String;
      };
    }
  | {
      TagsInsert: {
        id: CoId;
        did: Did;
        tags: Tags;
      };
    }
  | {
      TagsRemove: {
        id: CoId;
        did: Did;
        tags: Tags;
      };
    }
  | {
      Remove: {
        id: CoId;
        did?: Did;
      };
    };
