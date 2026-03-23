import { CID } from "multiformats";
import { CoId, Did, Tags } from "./co.js";

export enum MembershipState {
  /// Active membership.
  Active = 10,

  /// Pending state resolution.
  /// Has CoInviteMetadata to connect, but needs to resolve CO state
  /// (and optionally encryption key) from network before use.
  Pending = 15,

  /// Pending join by us.
  ///
  /// Use Cases:
  /// - This is a pending join triggered by an invite waiting for completion.
  /// - This is waiting for CO participant acceptation/rejection (remote).
  ///
  /// Related membership Tags:
  ///  `co-invite: CoInviteMetadata`
  ///  `join-date: Date`
  Join = 20,

  /// Pending invite by some participant of the CO.
  ///
  /// Use Cases:
  /// - This is waiting for our acceptation/rejection.
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
 *   did: Record<Did, MembershipState>,
 *   state: BTreeSet<CoState>,
 *   key: Option<String>,
 *   tags: Tags,
 * }
 */
export interface Membership {
  /** CO Unique Identifier */
  id: CoId;

  /** The membership states per DID */
  did: Record<Did, MembershipState>;

  /**
   * currently not needed
   */
  state: any;

  /**
   * Optional encryption key URI.
   * Rust: Option<String> → optional property
   */
  key?: string;

  /** Membership tags */
  tags: Tags;
}

export interface Memberships {
  /** List of membership entries */
  memberships: Membership[];
}

export interface MembershipOptions {
  state?: any;
  key?: string;
  tags?: Tags;
}

export type MembershipsAction =
  /// Active membership — CO creation, direct join, or activation.
  | {
      Join: {
        id: CoId;
        did: Did;
        options?: MembershipOptions;
      };
    }
  /// Received invite, awaiting user acceptance.
  | {
      Invited: {
        id: CoId;
        did: Did;
        options?: MembershipOptions;
      };
    }
  /// Auto-accepted or user-accepted invite, pending join completion.
  | {
      JoinRequest: {
        id: CoId;
        did: Did;
        options?: MembershipOptions;
      };
    }
  /// Pending state resolution.
  | {
      JoinPending: {
        id: CoId;
        did: Did;
        options?: MembershipOptions;
      };
    }
  /// User accepts invite. Invite -> Join.
  | {
      InviteAccept: {
        id: CoId;
        did: Did;
        options?: MembershipOptions;
      };
    }
  /// Membership deactivated. * -> Inactive.
  | {
      Deactivate: {
        id: CoId;
        did: Did;
      };
    }
  | {
      Update: {
        id: CoId;
        state: CID;
        /// Remove all [`CoState`] which heads are fully covered.
        remove: CID[];
      };
    }
  /// Change the active encryption key reference which is used the read the current heads/state.
  | {
      ChangeKey: {
        id: CoId;
        key: String;
      };
    }
  | {
      TagsInsert: {
        id: CoId;
        tags: Tags;
      };
    }
  | {
      TagsRemove: {
        id: CoId;
        tags: Tags;
      };
    }
  | {
      Remove: {
        id: CoId;
        did?: Did;
      };
    };
