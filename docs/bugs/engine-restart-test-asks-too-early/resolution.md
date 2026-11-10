# Resolution: desktop engine tests fail now and then right after a restart

[Diagnosis](detail.md).

## The fix

The three tests ask again while the supervisor answers `Lost` (the app's `engine-restarted`), every 10 ms for up to 5 s, as a caller of the supervisor does after a restart. The supervisor and the app are unchanged.

## How it was checked

The desktop crates' tests (97) pass three runs in a row locally. The race needs a slow runner, so CI's Linux ARM64 job is the check that matters.
