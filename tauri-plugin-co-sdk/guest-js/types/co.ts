import { CID } from "multiformats";
import { Network } from "./network.js";

export type CoId = string;
export type Did = string;

export type Tags = string[][];
/**
 * Rust: #[co] pub struct Core
 */
export interface Core {
  /** The CID of the core binary. */
  binary: CID;

  /** Core tags. */
  tags: Tags;

  /** The latest stream state (Option<Cid>). */
  state?: CID;
}

/**
 * Rust: #[co] pub struct Guard
 */
export interface Guard {
  /** The CID of the guard binary. */
  binary: CID;

  /** Guard tags. */
  tags: Tags;
}

export enum ParticipantState {
  Active = "Active",
  Invite = "Invite",
  // add more if needed
}

export type Participant = {
  did: Did;
  state: ParticipantState;
  tags?: Tags;
};

/**
 * Rust: #[co] pub struct Key
 */
export interface Key {
  /** Key ID string. */
  id: string;

  /** Key state (repr(u8) in Rust). */
  state: KeyState;
}

/**
 * Rust: #[co(repr)] #[repr(u8)] pub enum KeyState
 */
export enum KeyState {
  Inactive = 0,
  Active = 1,
}

export interface Co {
  /** CO UUID. */
  id: CoId;

  /** CO Tags. (default, omitted if empty) */
  t?: Tags;

  /** CO Name. */
  n: string;

  /** CO Core Binary (CID). */
  b: CID;

  /** CO Current heads. (BTreeSet<Cid> -> array) */
  heads: CID[];

  /**
   * CO Participants. (CoMap<Did, Participant> -> object map)
   * default, omitted if empty
   */
  p?: CID;

  /**
   * CO Streams with the associated state reference.
   * Key: Core Instance (string)
   * TODO Comes as object but should be map
   */
  c: { [key: string]: Core };

  /** Co Guards. default, omitted if empty */
  g?: Map<string, Guard>;

  /**
   * CO Encryption Keys.
   * The first (index 0) key is the active key.
   * Option<Vec<Key>> -> omitted if null/undefined.
   */
  k?: Key[];

  /**
   * CO network services. default, omitted if empty
   * Assuming CoSet<Network> serializes like a set -> array.
   */
  s?: Network[];
}

export type CoAction =
  | {
      Upgrade: {
        binary: CID;
        migrate?: CID;
      };
    }
  | {
      Heads: {
        heads: Set<CID>;
      };
    }
  | {
      TagsInsert: {
        tags: Tags;
      };
    }
  | {
      TagsRemove: {
        tags: Tags;
      };
    }
  | {
      ParticipantInvite: {
        participant: Did;
        tags: Tags;
      };
    }
  | {
      ParticipantJoin: {
        participant: Did;
        tags: Tags;
      };
    }
  | {
      ParticipantPending: {
        participant: Did;
        tags: Tags;
      };
    }
  | {
      ParticipantRemove: {
        participant: Did;
        tags: Tags;
      };
    }
  | {
      ParticipantTagsInsert: {
        participant: Did;
        tags: Tags;
      };
    }
  | {
      ParticipantTagsRemove: {
        participant: Did;
        tags: Tags;
      };
    }
  | {
      NetworkInsert: {
        network: Network;
      };
    }
  | {
      NetworkRemove: {
        network: Network;
      };
    }
  | {
      CoreCreate: {
        core: String;
        // both work somehow even though the action only takes CID in rust
        binary: CID | Uint8Array;
        tags: Tags;
      };
    }
  | {
      CoreRemove: {
        core: String;
      };
    }
  | {
      CoreChange: {
        core: String;
        state?: CID;
      };
    }
  | {
      CoreUpgrade: {
        core: String;

        /// The new binary.
        binary: CID;

        /// Migrate action.
        /// Must deserialize to a action using the new `binary`.
        migrate?: CID;
      };
    }
  | {
      CoreTagsInsert: {
        core: String;
        tags: Tags;
      };
    }
  | {
      CoreTagsRemove: {
        core: String;
        tags: Tags;
      };
    }
  | {
      GuardCreate: {
        guard: String;
        binary: CID;
        tags: Tags;
      };
    }
  | {
      GuardRemove: {
        guard: String;
      };
    }
  | {
      GuardUpgrade: {
        guard: String;
        /// The new binary.
        binary: CID;
      };
    }
  | {
      GuardTagsInsert: {
        guard: String;
        tags: Tags;
      };
    }
  | {
      GuardTagsRemove: {
        guard: String;
        tags: Tags;
      };
    };
