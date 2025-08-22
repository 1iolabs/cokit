# Permissions
Permissions are checks for states.
Since we introduced a new concept with guards, it is sensible to briefly explain how permissions work within COs. Guards and permissions serve similar purposes, but are not the same.

It is important to differentiate between the two for reasons of efficiency and security.
- The permissions are more granular then guards but got possible storage overhead.
- They take effect after possible conflicts are resolved.
- They essentially describe what makes it into the state of a core.
- They are permanent and will be re-evaluated after conflicts.

## When to use Guards or Permissions
Guard are evaluated before join transactions into the log.
Permissions are evaluated after transactions made it into the log, meaning the Guards are executed before any conflict-resolving logic takes place.

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
