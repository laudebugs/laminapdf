# Resolution: opening a PDF from Finder crashed the app when it wasn't running

[Diagnosis](detail.md).

## The fix

`open::PendingOpens`, which the builder manages before the app runs, also holds the files that come before `setup` has run (`hold_until_started`). `RunEvent::Opened` hands its files to it first, and routes them itself only once setup has run. At its end, `setup` takes what was held (`started`) and opens it in the first window, together with the command line's files, after the restored tabs, as a cold start's files from the command line always were. Both run on the main thread, so a request can't fall between the hold and the take.

## How it was checked

- `a_cold_start_should_hold_its_files_until_setup_has_run` and `files_after_setup_should_go_straight_on`: the hold keeps files in order until `started`, and hands them straight back after it.
- The reproduction again, with a release bundle of this branch on macOS 27: `open -a` on the closed app starts it and shows the file (row 1 of `apps/desktop/e2e/MANUAL-OPEN.md`), and `open -a` with it running shows the second file in the running window, with one process (row 2). A drop on the Dock icon (row 6) sends the same Apple event and wasn't tried separately.
- The app's tests and clippy pass.

## Left for later

- WebDriver can't send an Apple event, so no e2e test covers this path; `MANUAL-OPEN.md`'s row 1 is the check, and it has to be run by hand on a change to how files open.
