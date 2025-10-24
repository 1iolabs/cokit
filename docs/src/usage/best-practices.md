# Best Practices
In the [getting started](../getting-started/installation.md#building-your-first-app) chapter, we've already built a simple to-do list together.
Now let's take a look at how you really can kick things off in your development process with CO-kit.
You'll find some handy best practices in this chapter and we'll also hand you some do's and don'ts to ease the start of development with CO-kit.

## Do's
### Core: Use stable identifiers
Use stable identifiers for example UUID's when referencing data.
Monotonic counters may change when some conflict resolution takes place.

### Core: Design actions to be task based
Ideally design actions to be task based, meaning not to split a single logical task over multiple actions 

For further information see:
- [Core](../reference/core.md#actions)

## Don'ts
### Proof-of-Authority Misconfiguration
The Proof-of-Authority Consensus can be configured to allow a minority to reach consensus.
While this is technically possible and may be useful for certain tasks, it would make things unintuitive to unaware users.

For further information see:
- [Consensus](../reference/consensus.md#proof-of-authority)
