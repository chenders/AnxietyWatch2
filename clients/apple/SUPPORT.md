# Apple platform support baseline

The owner's existing generation-one project pins iOS 18.0 and watchOS 11.0 in
`Config/Project.xcconfig` and `Config/watchOS.xcconfig`. Stage 7 preserves those minimum versions:

- iPhone: iOS 18.0 or later, device family 1.
- Apple Watch: watchOS 11.0 or later, device family 4, paired through the iPhone.

This is deliberately an iPhone client, not a new iPad commitment. Generation one listed device
families 1 and 2; Stage 7 narrows the target to the owner's actual iPhone/Apple Watch topology defined
by the accepted specification. Raising either deployment version requires an explicit support decision,
physical-device confirmation, and an update to this file in the same change.
