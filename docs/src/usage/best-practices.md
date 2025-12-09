# Best Practices
In the [getting started](../getting-started/installation.md#building-your-first-app) chapter, we built a simple to-do list together.  
Now let's take a look at how you really can kick things off in your development process with CO-kit.

## Do
### Core: Use stable identifiers
Use stable identifiers (for example UUIDs) when referencing data.  
Monotonic counters may change when some conflict resolution takes place.

### Core: Design actions to be task-based
Ideally design actions to be task-based – meaning don't split a single logical task over multiple actions. 

For further information, see:
- [Core](../reference/core.md#actions)

## Don't
### Proof-of-Authority Misconfiguration
The Proof-of-Authority Consensus _can_ be configured to allow a minority to reach consensus.  
While this is technically possible, and may even be useful for certain tasks, it would make things unintuitive to unaware users.

For further information, see:
- [Consensus](../reference/consensus.md#proof-of-authority)

### Core: Reference previous state(s)
When referencing a previous state, this effectively disables the ability to free up any of the previous states.  
This means that all states will be kept alive, as the latest state is marked as an active root in the storage core.

For further information, see:
- [Core: Storage](../reference/core.md#co-core-storage)
