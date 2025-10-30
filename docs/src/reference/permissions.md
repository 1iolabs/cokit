# Permissions
Permissions are checks for states.
Since we introduced a new concept with guards, it is sensible to briefly explain how permissions work within COs. Guards and permissions serve similar purposes, but are not the same.

It is important to differentiate between the two for reasons of efficiency and security.
- The permissions are more granular then guards but got possible storage overhead.
- They take effect after possible conflicts are resolved.
- They essentially describe what makes it into the state of a core.
- They are permanent and will be re-evaluated after conflicts.

Some examples:
- Someone is allowed to comment on blog entries but not to create new blog entries.
- Someone is allowed to post new messages but not to delete them.

These checks are implemented as simple checks or conditions in the Core.

For an implementation example click [here](../getting-started/first-steps.md#permissions).

## When to use Guards or Permissions
Guards are evaluated _before_ join transactions into the log.
Permissions are evaluated _after_ transactions made it into the log, meaning the Guards are executed before any conflict-resolving logic takes place.

A quick comparison of Permissions and Guards:

|                                          | Guard                             | Permission                        |
| ---------------------------------------- | --------------------------------- | --------------------------------- |
| Instant (Evaluated once)                 | <input type="checkbox" style="pointer-events: none;" checked /> | <input type="checkbox" style="pointer-events: none;" />         |
| Permanent (Re-evaluated after conflicts) | <input type="checkbox" style="pointer-events: none;" />         | <input type="checkbox" style="pointer-events: none;" checked /> |
| Applies to Transactions                  | <input type="checkbox" style="pointer-events: none;" checked /> | <input type="checkbox" style="pointer-events: none;" />         |
| Applies to State                         | <input type="checkbox" style="pointer-events: none;" />         | <input type="checkbox" style="pointer-events: none;" checked /> |

## See also
- [Guards](../reference/guards.md)
- [Core](../reference/core.md)
