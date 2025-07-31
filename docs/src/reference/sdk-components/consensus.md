# Consensus

#todo

## Abstract
When heads from different participants are joined the event order in the [Log](./log.md#Example) may change.
This is necessary to make the Merkle-CRDT work technically.
The consensus on the other hand is used to allow the network of participants of an CO to commit to an known state/heads combination.
After a successful consensus round (depending on the consensus algorithm) there are no more heads accepted that would alter the sorting of events before this point.

## Checkpoint
For each set of heads a materialized state is calculated.
A checkpoint (or snapshot) is the combination of this state and heads.
Each successful consensus round produces a checkpoint.
For example:
- a checkpoint functions like a block in a Blockchain, but it's more versatile because it is not dependent on a predefined mining period - but rather on demand.

## Consensus on demand (CoD)
A consensus round can be triggered manually on demand.
One participant can start a validation at any given time.
Other participants that received the initial validation, then collaborate and sign the validation.

## Finality
Finality means a certain checkpoint is agreed upon and no events can be inserted before it.

## Trust
Finality is cryptographically verifiable once reached and therefore can be trusted to be immutable.

## Optional
Consensus algorithm in a CO are completely optional.
The Merkle-CRDT solves the technical part of syncing, conflict handling and validation.
If no other mechanism is applied (proof of authority, etc.), the Merkle-CRDT serves as a single source of truth.
Depending on your project requirements, you can implement any other consensus mechanism.

##  Proof of authority
The built-in consensus mechanism used in CO-kit.
An authority is defined using DIDs and for example, a majority of votes needed to have a state validated on.

See: 
- [co-core-poa](/reference/core.md#co-core-poa)
- [guards](/reference/guards.md)

