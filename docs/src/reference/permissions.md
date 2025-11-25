# Permissions
Permissions are checks for states.  
Given that we introduced a new concept with Guards, we should now briefly explain how permissions work within COs.  

Guards and permissions serve similar purposes, but are not the same.  
It is important to differentiate between the two for reasons of efficiency and security.
- Permissions are more granular than Guards, but have a possible storage overhead.
- Permissions take effect _after_ possible conflicts are resolved.
- Permissions essentially describe what makes it into the state of a Core.
- Permissions are permanent, and will be re-evaluated after conflicts.

Some examples:
- Someone is allowed to comment on blog entries but not to create new blog entries.
- Someone is allowed to post new messages but not to delete them.

These checks are implemented as simple checks or conditions in the Core.

For an implementation example, click [here](../getting-started/next-steps.md#permissions).

## When to use Guards or Permissions
Guards are evaluated _before_ join transactions into the log.  
Permissions are evaluated _after_ transactions made it into the log, meaning the Guards are executed before any conflict-resolving logic takes place.

A quick comparison of Permissions and Guards:

|                                          | Guard                             | Permission                        |
| ---------------------------------------- | --------------------------------- | --------------------------------- |
| Instant (Evaluated once)                 | <div style="text-align: center;"><input type="checkbox" style="pointer-events: none;" checked /></div> | <div style="text-align: center;"><input type="checkbox" style="pointer-events: none;" /></div>         |
| Permanent (Re-evaluated after conflicts) | <div style="text-align: center;"><input type="checkbox" style="pointer-events: none;" /></div>         | <div style="text-align: center;"><input type="checkbox" style="pointer-events: none;" checked /></div> |
| Applies to Transactions                  | <div style="text-align: center;"><input type="checkbox" style="pointer-events: none;" checked /></div> | <div style="text-align: center;"><input type="checkbox" style="pointer-events: none;" /></div>         |
| Applies to State                         | <div style="text-align: center;"><input type="checkbox" style="pointer-events: none;" /></div>         | <div style="text-align: center;"><input type="checkbox" style="pointer-events: none;" checked /></div> |

## See also
- [Guards](../reference/guards.md)
- [Core](../reference/core.md)
