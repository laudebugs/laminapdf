# Desktop cold start

`cold_start.py` measures how long the desktop app takes from launch to the first command its window sends, the moment its empty state accepts input (ADR 0008 §4). It needs a release build with the `e2e` feature, whose first command logs the app's own timings:

```sh
pnpm --filter @laminapdf/desktop run desktop:build --no-bundle --features e2e
python3 tools/desktop/cold_start.py target/release/laminapdf-desktop 10
```

`floor/` is the floor: a page that sends the first command as soon as it runs, with none of the app's code. Built in place of the app's frontend, it shows how much of the cold start Tauri and the webview take on their own:

```sh
cd apps/desktop
cp ../../target/release/laminapdf-desktop /tmp/app
pnpm tauri build --no-bundle --features e2e --config '{"build":{"frontendDist":"../../../tools/desktop/floor","beforeBuildCommand":""}}'
cp ../../target/release/laminapdf-desktop /tmp/floor
python3 ../../tools/desktop/cold_start.py /tmp/app 10 /tmp/floor
```

Given two apps, the script launches them in turn, so a busy machine slows both alike. Compare medians from the same session, never absolute numbers from different days or machines.
