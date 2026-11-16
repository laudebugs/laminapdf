# Resolution: signing the .rpm failed on the space in the package's name

[Diagnosis](detail.md).

## The fix

The build action's signing step copies each `.rpm` to `$RUNNER_TEMP/rpmsign/package.rpm`, signs it there and moves it back over the original, so rpm never sees the space. The `.rpm` keeps its name, and the step after it checks its signature as before (`rpm -K`).

## How it was checked

In `ubuntu:22.04` with rpm 4.17.0, the old command fails with `gpg exec failed (2)`, as on the runner; the new steps sign the package, and `rpm -K` answers `digests signatures OK` once the test key is imported. Only a `desktop-release` run signs with the owner's key, so the next staging release is the check on the runners.

## Left for later

Nothing. Renaming the package instead would change the file names the release publishes.
