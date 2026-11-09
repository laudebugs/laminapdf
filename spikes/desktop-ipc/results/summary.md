## linux-wayland-dmabuf-off

```
date: 2026-11-09T16:01Z
uname: Linux 7.0.0-1017-azure x86_64
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
tauri: version = "2.12.5"
os: Ubuntu 24.04.5 LTS
cpu: AMD EPYC 9V74 80-Core Processor, 4 cores
memory: 15 GiB
webkit2gtk-4.1: 2.52.6
session: XDG_SESSION_TYPE= GDK_BACKEND=wayland DISPLAY= WAYLAND_DISPLAY=wayland-spike
WEBKIT_DISABLE_DMABUF_RENDERER=1
gl: 
unknown
```

| headers | crossOriginIsolated | SharedArrayBuffer | invoke | fetch tile (CORP same-origin) | `<img>` tile (CORP same-origin) | `<img>` tile (CORP cross-origin) | page origin |
| --- | --- | --- | --- | --- | --- | --- | --- |
| off | false | unavailable | ok | ok | ok | ok | tauri://localhost |
| on | false | unavailable | ok | ok | ok | ok | tauri://localhost |

In-flight sweep (512 px, DPR 2, one run each), burst tiles/s:

| transport | n = 4 | n = 8 | n = 16 |
| --- | --: | --: | --: |
| channel | 300 | 305.7 | 300 |
| response | 315.8 | 304.8 | 323.2 |
| scheme-raw | 302.8 | 323.2 | 341.6 |
| scheme-png | 388.7 | 395.1 | 390.2 |
| scheme-webp | 755.9 | 786.9 | 849.6 |
| scheme-qoi | 662.1 | 653.1 | 635.8 |

### linux-wayland-dmabuf-off: pdf-256-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 970 | 152 | 9.0 | 18.0 | 0 | 9 | 57 | 398 | 26 | 0.00 |
| response | 16 | 5 | 1231 | 193 | 6.0 | 19.0 | 0 | 9 | 39 | 410 | 25 | 0.00 |
| scheme-raw | 16 | 5 | 1231 | 193 | 5.0 | 20.0 | 1 | 9 | 48 | 410 | 25 | 0.00 |
| scheme-png | 8 | 5 | 1116 | 13 | 7.0 | 11.0 | 0 | 9 | 48 | 396 | 25 | 0.10 |
| scheme-webp | 16 | 5 | 1846 | 7 | 6.0 | 11.0 | 0 | 9 | 32 | 395 | 24 | 0.22 |
| scheme-qoi | 4 | 5 | 1846 | 16 | 5.0 | 6.0 | 0 | 10 | 96 | 401 | 24 | 0.09 |

### linux-wayland-dmabuf-off: pdf-256-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 857 | 177 | 15.0 | 24.0 | 0 | 21 | 244 | 473 | 47 | 0.00 |
| response | 16 | 5 | 960 | 198 | 9.0 | 21.0 | 2 | 17 | 127 | 494 | 43 | 0.00 |
| scheme-raw | 16 | 5 | 960 | 198 | 9.0 | 20.0 | 2 | 16 | 117 | 480 | 43 | 0.00 |
| scheme-png | 8 | 5 | 1032 | 4 | 11.0 | 14.0 | 0 | 18 | 183 | 465 | 40 | 0.10 |
| scheme-webp | 16 | 5 | 1959 | 1 | 9.0 | 14.0 | 0 | 16 | 96 | 463 | 38 | 0.25 |
| scheme-qoi | 4 | 5 | 1778 | 7 | 8.0 | 9.0 | 0 | 32 | 322 | 470 | 37 | 0.10 |

### linux-wayland-dmabuf-off: pdf-512-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 370 | 197 | 11.0 | 48.0 | 3 | 5 | 53 | 487 | 25 | 0.00 |
| response | 16 | 5 | 386 | 206 | 7.0 | 40.0 | 3 | 5 | 48 | 499 | 24 | 0.00 |
| scheme-raw | 16 | 5 | 386 | 206 | 6.0 | 36.0 | 2 | 5 | 49 | 498 | 24 | 0.00 |
| scheme-png | 8 | 5 | 455 | 16 | 7.0 | 23.0 | 2 | 5 | 32 | 480 | 22 | 0.26 |
| scheme-webp | 16 | 5 | 920 | 11 | 6.0 | 13.0 | 0 | 4 | 16 | 474 | 21 | 0.64 |
| scheme-qoi | 4 | 5 | 802 | 21 | 5.0 | 9.0 | 0 | 5 | 32 | 484 | 21 | 0.25 |

### linux-wayland-dmabuf-off: pdf-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 312 | 195 | 21.0 | 55.0 | 14 | 15 | 186 | 515 | 45 | 0.00 |
| response | 16 | 5 | 332 | 208 | 12.0 | 92.0 | 10 | 10 | 133 | 546 | 44 | 0.00 |
| scheme-raw | 16 | 5 | 330 | 207 | 12.0 | 88.0 | 10 | 10 | 130 | 542 | 44 | 0.00 |
| scheme-png | 8 | 5 | 417 | 12 | 14.0 | 36.0 | 7 | 10 | 163 | 487 | 37 | 0.27 |
| scheme-webp | 16 | 5 | 897 | 7 | 10.0 | 23.0 | 3 | 9 | 65 | 490 | 36 | 0.76 |
| scheme-qoi | 4 | 5 | 686 | 15 | 9.0 | 14.0 | 0 | 10 | 104 | 518 | 35 | 0.33 |

### linux-wayland-dmabuf-off: synthetic-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 266 | 185 | 26.0 | 56.0 | 137 | 51 | 204 | 572 | 75 | 0.00 |
| response | 16 | 5 | 293 | 203 | 13.0 | 40.0 | 16 | 39 | 145 | 624 | 71 | 0.00 |
| scheme-raw | 16 | 5 | 287 | 200 | 13.0 | 43.0 | 15 | 38 | 145 | 620 | 71 | 0.00 |
| scheme-png | 8 | 5 | 324 | 39 | 16.0 | 32.0 | 27 | 32 | 180 | 562 | 57 | 0.61 |
| scheme-webp | 16 | 5 | 733 | 10 | 10.0 | 20.0 | 4 | 30 | 71 | 555 | 46 | 0.82 |
| scheme-qoi | 4 | 5 | 365 | 52 | 13.0 | 17.0 | 5 | 31 | 139 | 613 | 53 | 0.66 |

## linux-wayland-dmabuf-on

```
date: 2026-11-09T16:00Z
uname: Linux 7.0.0-1017-azure x86_64
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
tauri: version = "2.12.5"
os: Ubuntu 24.04.5 LTS
cpu: AMD EPYC 9V45 96-Core Processor, 4 cores
memory: 15 GiB
webkit2gtk-4.1: 2.52.6
session: XDG_SESSION_TYPE= GDK_BACKEND=wayland DISPLAY= WAYLAND_DISPLAY=wayland-spike
WEBKIT_DISABLE_DMABUF_RENDERER=0
gl: 
unknown
```

| headers | crossOriginIsolated | SharedArrayBuffer | invoke | fetch tile (CORP same-origin) | `<img>` tile (CORP same-origin) | `<img>` tile (CORP cross-origin) | page origin |
| --- | --- | --- | --- | --- | --- | --- | --- |
| off | false | unavailable | ok | ok | ok | ok | tauri://localhost |
| on | false | unavailable | ok | ok | ok | ok | tauri://localhost |

**Run stopped:** no animation frame for 3000 ms

In-flight sweep (512 px, DPR 2, one run each), burst tiles/s:

| transport | n = 4 | n = 8 | n = 16 |
| --- | --: | --: | --: |
| channel | 342.9 | 334.5 | 347.8 |
| response | 390.2 | 352.9 |  |

## linux-x11-dmabuf-off

```
date: 2026-11-09T16:01Z
uname: Linux 7.0.0-1017-azure x86_64
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
tauri: version = "2.12.5"
os: Ubuntu 24.04.5 LTS
cpu: AMD EPYC 9V74 80-Core Processor, 4 cores
memory: 15 GiB
webkit2gtk-4.1: 2.52.6
session: XDG_SESSION_TYPE= GDK_BACKEND=x11 DISPLAY=:99 WAYLAND_DISPLAY=
WEBKIT_DISABLE_DMABUF_RENDERER=1
gl: llvmpipe (LLVM 20.1.2, 256 bits)
```

| headers | crossOriginIsolated | SharedArrayBuffer | invoke | fetch tile (CORP same-origin) | `<img>` tile (CORP same-origin) | `<img>` tile (CORP cross-origin) | page origin |
| --- | --- | --- | --- | --- | --- | --- | --- |
| off | false | unavailable | ok | ok | ok | ok | tauri://localhost |
| on | false | unavailable | ok | ok | ok | ok | tauri://localhost |

In-flight sweep (512 px, DPR 2, one run each), burst tiles/s:

| transport | n = 4 | n = 8 | n = 16 |
| --- | --: | --: | --: |
| channel | 312.7 | 296.3 | 291.8 |
| response | 328.8 | 321.1 | 335.7 |
| scheme-raw | 329.9 | 332.2 | 305.7 |
| scheme-png | 400 | 415.6 | 417.4 |
| scheme-webp | 774.2 | 774.2 | 780.5 |
| scheme-qoi | 690.6 | 680.9 | 680.9 |

### linux-x11-dmabuf-off: pdf-256-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 4 | 5 | 941 | 147 | 9.0 | 11.0 | 0 | 10 | 97 | 401 | 26 | 0.00 |
| response | 16 | 5 | 1231 | 193 | 5.0 | 21.0 | 1 | 9 | 37 | 412 | 25 | 0.00 |
| scheme-raw | 8 | 5 | 1200 | 188 | 5.0 | 10.0 | 0 | 9 | 48 | 415 | 25 | 0.00 |
| scheme-png | 16 | 5 | 1185 | 13 | 7.0 | 16.0 | 0 | 9 | 35 | 401 | 24 | 0.09 |
| scheme-webp | 16 | 5 | 1882 | 7 | 6.0 | 10.0 | 0 | 9 | 32 | 408 | 24 | 0.22 |
| scheme-qoi | 4 | 5 | 1920 | 16 | 5.0 | 6.0 | 0 | 10 | 97 | 403 | 24 | 0.09 |

### linux-x11-dmabuf-off: pdf-256-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 4 | 5 | 774 | 160 | 13.0 | 16.0 | 0 | 33 | 370 | 461 | 49 | 0.00 |
| response | 16 | 5 | 923 | 191 | 9.0 | 22.0 | 4 | 17 | 121 | 488 | 47 | 0.00 |
| scheme-raw | 8 | 5 | 950 | 196 | 9.0 | 12.0 | 1 | 18 | 161 | 508 | 47 | 0.00 |
| scheme-png | 16 | 5 | 1067 | 4 | 11.0 | 28.0 | 3 | 18 | 170 | 459 | 42 | 0.10 |
| scheme-webp | 16 | 5 | 1920 | 1 | 9.0 | 14.0 | 0 | 16 | 97 | 465 | 41 | 0.25 |
| scheme-qoi | 4 | 5 | 1714 | 6 | 8.0 | 10.0 | 0 | 32 | 321 | 480 | 39 | 0.10 |

### linux-x11-dmabuf-off: pdf-512-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 4 | 5 | 346 | 185 | 11.0 | 30.0 | 3 | 6 | 68 | 483 | 28 | 0.00 |
| response | 16 | 5 | 363 | 194 | 7.0 | 44.0 | 2 | 5 | 49 | 507 | 27 | 0.00 |
| scheme-raw | 8 | 5 | 382 | 204 | 6.0 | 37.0 | 2 | 5 | 50 | 500 | 27 | 0.00 |
| scheme-png | 16 | 5 | 458 | 17 | 7.0 | 25.0 | 2 | 4 | 32 | 475 | 25 | 0.26 |
| scheme-webp | 16 | 5 | 910 | 11 | 6.0 | 13.0 | 0 | 4 | 17 | 473 | 24 | 0.61 |
| scheme-qoi | 4 | 5 | 794 | 21 | 5.0 | 10.0 | 0 | 5 | 32 | 482 | 24 | 0.26 |

### linux-x11-dmabuf-off: pdf-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 4 | 5 | 305 | 191 | 21.0 | 37.0 | 10 | 18 | 247 | 522 | 47 | 0.00 |
| response | 16 | 5 | 331 | 208 | 12.0 | 71.0 | 11 | 10 | 141 | 553 | 45 | 0.00 |
| scheme-raw | 8 | 5 | 324 | 203 | 12.0 | 43.0 | 8 | 11 | 141 | 552 | 45 | 0.00 |
| scheme-png | 16 | 5 | 425 | 12 | 14.0 | 59.0 | 8 | 10 | 117 | 496 | 39 | 0.28 |
| scheme-webp | 16 | 5 | 820 | 7 | 10.0 | 25.0 | 3 | 9 | 56 | 502 | 38 | 0.76 |
| scheme-qoi | 4 | 5 | 691 | 15 | 10.0 | 13.0 | 0 | 10 | 98 | 532 | 37 | 0.31 |

### linux-x11-dmabuf-off: synthetic-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 4 | 5 | 282 | 196 | 27.0 | 35.0 | 131 | 54 | 243 | 568 | 76 | 0.00 |
| response | 16 | 5 | 290 | 202 | 13.0 | 52.0 | 17 | 41 | 137 | 626 | 74 | 0.00 |
| scheme-raw | 8 | 5 | 291 | 202 | 13.0 | 28.0 | 13 | 39 | 154 | 612 | 73 | 0.00 |
| scheme-png | 16 | 5 | 323 | 39 | 16.0 | 57.0 | 28 | 31 | 161 | 567 | 60 | 0.62 |
| scheme-webp | 16 | 5 | 733 | 10 | 10.0 | 21.0 | 3 | 30 | 70 | 553 | 48 | 0.84 |
| scheme-qoi | 4 | 5 | 368 | 52 | 13.0 | 18.0 | 2 | 31 | 138 | 610 | 54 | 0.66 |

## linux-x11-dmabuf-on

```
date: 2026-11-09T16:02Z
uname: Linux 7.0.0-1017-azure x86_64
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
tauri: version = "2.12.5"
os: Ubuntu 24.04.5 LTS
cpu: AMD EPYC 7763 64-Core Processor, 4 cores
memory: 15 GiB
webkit2gtk-4.1: 2.52.6
session: XDG_SESSION_TYPE= GDK_BACKEND=x11 DISPLAY=:99 WAYLAND_DISPLAY=
WEBKIT_DISABLE_DMABUF_RENDERER=0
gl: llvmpipe (LLVM 20.1.2, 256 bits)
```

| headers | crossOriginIsolated | SharedArrayBuffer | invoke | fetch tile (CORP same-origin) | `<img>` tile (CORP same-origin) | `<img>` tile (CORP cross-origin) | page origin |
| --- | --- | --- | --- | --- | --- | --- | --- |
| off | false | unavailable | ok | ok | ok | ok | tauri://localhost |
| on | false | unavailable | ok | ok | ok | ok | tauri://localhost |

In-flight sweep (512 px, DPR 2, one run each), burst tiles/s:

| transport | n = 4 | n = 8 | n = 16 |
| --- | --: | --: | --: |
| channel | 189.3 | 189 | 192.8 |
| response | 203 | 209.2 | 206.9 |
| scheme-raw | 201.7 | 210.1 | 202.5 |
| scheme-png | 316.8 | 333.3 | 334.5 |
| scheme-webp | 607.6 | 690.6 | 671.3 |
| scheme-qoi | 417.4 | 408.5 | 446.5 |

### linux-x11-dmabuf-on: pdf-256-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 16 | 5 | 619 | 97 | 15.0 | 60.0 | 8 | 11 | 101 | 692 | 119 | 0.00 |
| response | 8 | 5 | 696 | 109 | 11.0 | 32.0 | 7 | 10 | 100 | 698 | 118 | 0.00 |
| scheme-raw | 8 | 5 | 733 | 115 | 9.0 | 35.0 | 4 | 11 | 116 | 694 | 118 | 0.00 |
| scheme-png | 16 | 5 | 906 | 10 | 8.0 | 24.0 | 3 | 9 | 58 | 692 | 121 | 0.13 |
| scheme-webp | 8 | 5 | 1215 | 4 | 9.0 | 22.0 | 1 | 10 | 86 | 691 | 120 | 0.30 |
| scheme-qoi | 16 | 5 | 1129 | 10 | 9.0 | 28.0 | 4 | 9 | 48 | 693 | 116 | 0.13 |

### linux-x11-dmabuf-on: pdf-256-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 16 | 5 | 485 | 100 | 50.0 | 75.0 | 125 | 27 | 378 | 872 | 158 | 0.00 |
| response | 8 | 5 | 546 | 113 | 26.0 | 52.0 | 127 | 29 | 469 | 895 | 159 | 0.00 |
| scheme-raw | 8 | 5 | 561 | 116 | 26.0 | 49.0 | 124 | 29 | 429 | 898 | 158 | 0.00 |
| scheme-png | 16 | 5 | 787 | 3 | 21.0 | 46.0 | 176 | 23 | 260 | 864 | 155 | 0.13 |
| scheme-webp | 8 | 5 | 1280 | 1 | 17.0 | 32.0 | 125 | 25 | 279 | 860 | 157 | 0.34 |
| scheme-qoi | 16 | 5 | 980 | 4 | 20.0 | 32.0 | 98 | 22 | 177 | 888 | 157 | 0.15 |

### linux-x11-dmabuf-on: pdf-512-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 16 | 5 | 221 | 118 | 17.0 | 90.0 | 15 | 6 | 90 | 876 | 118 | 0.00 |
| response | 8 | 5 | 232 | 124 | 14.0 | 82.0 | 12 | 5 | 88 | 885 | 117 | 0.00 |
| scheme-raw | 8 | 5 | 242 | 129 | 12.0 | 82.0 | 9 | 6 | 92 | 882 | 118 | 0.00 |
| scheme-png | 16 | 5 | 366 | 13 | 11.0 | 34.0 | 5 | 5 | 45 | 854 | 115 | 0.35 |
| scheme-webp | 8 | 5 | 664 | 8 | 11.0 | 26.0 | 4 | 5 | 64 | 856 | 116 | 0.88 |
| scheme-qoi | 16 | 5 | 468 | 12 | 10.0 | 39.0 | 6 | 5 | 45 | 863 | 114 | 0.44 |

### linux-x11-dmabuf-on: pdf-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 16 | 5 | 186 | 116 | 44.0 | 152.0 | 93 | 20 | 273 | 900 | 157 | 0.00 |
| response | 8 | 5 | 205 | 129 | 30.0 | 85.0 | 93 | 16 | 271 | 918 | 157 | 0.00 |
| scheme-raw | 8 | 5 | 205 | 128 | 28.0 | 85.0 | 80 | 16 | 270 | 910 | 158 | 0.00 |
| scheme-png | 16 | 5 | 333 | 10 | 25.0 | 90.0 | 62 | 15 | 167 | 874 | 155 | 0.39 |
| scheme-webp | 8 | 5 | 636 | 5 | 20.0 | 38.0 | 48 | 14 | 128 | 863 | 158 | 1.06 |
| scheme-qoi | 16 | 5 | 421 | 9 | 21.0 | 69.0 | 67 | 11 | 127 | 892 | 157 | 0.61 |

### linux-x11-dmabuf-on: synthetic-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 16 | 5 | 181 | 126 | 57.0 | 124.0 | 303 | 68 | 264 | 989 | 158 | 0.00 |
| response | 8 | 5 | 190 | 132 | 41.0 | 69.0 | 301 | 69 | 257 | 1030 | 160 | 0.00 |
| scheme-raw | 8 | 5 | 190 | 132 | 40.0 | 72.0 | 303 | 69 | 271 | 1018 | 159 | 0.00 |
| scheme-png | 16 | 5 | 252 | 30 | 30.0 | 89.0 | 257 | 66 | 236 | 969 | 153 | 0.88 |
| scheme-webp | 8 | 5 | 549 | 7 | 22.0 | 41.0 | 174 | 55 | 158 | 960 | 160 | 1.17 |
| scheme-qoi | 16 | 5 | 239 | 34 | 29.0 | 74.0 | 223 | 60 | 185 | 1005 | 155 | 0.99 |

## macos-15-hosted

```
date: 2026-11-09T16:16Z
uname: Darwin 24.6.0 arm64
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
tauri: version = "2.12.5"
os: macOS 15.8.2 (24H232); WKWebView ships with the OS
cpu: Apple M1 (Virtual), 3 cores
memory: 7 GiB
```

| headers | crossOriginIsolated | SharedArrayBuffer | invoke | fetch tile (CORP same-origin) | `<img>` tile (CORP same-origin) | `<img>` tile (CORP cross-origin) | page origin |
| --- | --- | --- | --- | --- | --- | --- | --- |
| off | false | unavailable | ok | ok | ok | ok | tauri://localhost |
| on | false | unavailable | ok | ok | ok | ok | tauri://localhost |

In-flight sweep (512 px, DPR 2, one run each), burst tiles/s:

| transport | n = 4 | n = 8 | n = 16 |
| --- | --: | --: | --: |
| channel | 223.3 | 267.4 | 228.6 |
| response | 194.7 | 234.7 | 240 |
| scheme-raw | 271.2 | 277.5 | 235.3 |
| scheme-png | 320 | 268.2 | 421.1 |
| scheme-webp | 341.6 | 356.9 | 390.2 |
| scheme-qoi | 220.7 | 278.3 | 170.5 |

### macos-15-hosted: pdf-256-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 338 | 53 | 13.0 | 29.0 | 4 | 11 | 84 | 240 | 29 | 0.00 |
| response | 16 | 5 | 362 | 57 | 13.0 | 44.0 | 4 | 11 | 82 | 240 | 30 | 0.00 |
| scheme-raw | 8 | 5 | 397 | 62 | 12.0 | 32.0 | 4 | 10 | 98 | 241 | 25 | 0.00 |
| scheme-png | 16 | 5 | 568 | 6 | 8.0 | 39.0 | 1 | 10 | 81 | 206 | 35 | 0.28 |
| scheme-webp | 16 | 5 | 667 | 2 | 7.0 | 31.0 | 1 | 10 | 51 | 199 | 34 | 0.56 |
| scheme-qoi | 8 | 5 | 400 | 3 | 13.0 | 31.0 | 2 | 11 | 83 | 239 | 28 | 0.22 |

### macos-15-hosted: pdf-256-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 307 | 62 | 15.0 | 34.0 | 4 | 24 | 270 | 395 | 36 | 0.00 |
| response | 16 | 5 | 354 | 72 | 17.0 | 57.0 | 5 | 23 | 260 | 395 | 35 | 0.00 |
| scheme-raw | 8 | 5 | 421 | 86 | 16.0 | 31.0 | 8 | 22 | 221 | 395 | 33 | 0.00 |
| scheme-png | 16 | 5 | 706 | 2 | 12.0 | 41.0 | 3 | 20 | 189 | 255 | 42 | 0.22 |
| scheme-webp | 16 | 5 | 780 | 0 | 10.0 | 31.0 | 1 | 18 | 153 | 258 | 40 | 0.40 |
| scheme-qoi | 8 | 5 | 417 | 2 | 16.0 | 30.0 | 8 | 22 | 249 | 401 | 30 | 0.17 |

### macos-15-hosted: pdf-512-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 289 | 154 | 15.0 | 38.0 | 3 | 6 | 64 | 370 | 29 | 0.00 |
| response | 16 | 5 | 288 | 154 | 14.0 | 40.0 | 3 | 5 | 53 | 370 | 29 | 0.00 |
| scheme-raw | 8 | 5 | 319 | 170 | 13.0 | 44.0 | 4 | 5 | 46 | 370 | 28 | 0.00 |
| scheme-png | 16 | 5 | 422 | 15 | 10.0 | 31.0 | 1 | 5 | 38 | 332 | 33 | 0.95 |
| scheme-webp | 16 | 5 | 503 | 6 | 10.0 | 32.0 | 1 | 5 | 49 | 334 | 33 | 1.90 |
| scheme-qoi | 8 | 5 | 274 | 7 | 15.0 | 48.0 | 4 | 6 | 67 | 370 | 27 | 0.76 |

### macos-15-hosted: pdf-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 267 | 167 | 15.0 | 46.0 | 6 | 13 | 120 | 495 | 28 | 0.00 |
| response | 16 | 5 | 319 | 200 | 13.0 | 64.0 | 4 | 11 | 119 | 495 | 26 | 0.00 |
| scheme-raw | 8 | 5 | 316 | 198 | 14.0 | 34.0 | 5 | 11 | 105 | 500 | 25 | 0.00 |
| scheme-png | 16 | 5 | 429 | 12 | 9.0 | 55.0 | 5 | 10 | 90 | 351 | 34 | 0.55 |
| scheme-webp | 16 | 5 | 397 | 3 | 9.0 | 60.0 | 4 | 10 | 86 | 348 | 35 | 1.71 |
| scheme-qoi | 8 | 5 | 224 | 5 | 18.0 | 55.0 | 13 | 12 | 166 | 495 | 27 | 0.77 |

### macos-15-hosted: synthetic-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 233 | 162 | 15.0 | 35.0 | 11 | 39 | 136 | 527 | 54 | 0.00 |
| response | 16 | 5 | 270 | 188 | 15.0 | 48.0 | 13 | 34 | 109 | 530 | 49 | 0.00 |
| scheme-raw | 8 | 5 | 276 | 192 | 16.0 | 34.0 | 15 | 33 | 132 | 533 | 43 | 0.00 |
| scheme-png | 16 | 5 | 306 | 36 | 13.0 | 53.0 | 10 | 36 | 142 | 357 | 79 | 1.40 |
| scheme-webp | 16 | 5 | 379 | 5 | 11.0 | 44.0 | 7 | 33 | 121 | 355 | 71 | 1.92 |
| scheme-qoi | 8 | 5 | 165 | 23 | 23.0 | 49.0 | 106 | 42 | 189 | 527 | 59 | 1.41 |

## macos-m1pro

```
date: 2026-11-09T15:58Z
uname: Darwin 27.0.0 arm64
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
tauri: version = "2.12.5"
os: macOS 27.0.1 (26A434); WKWebView ships with the OS
cpu: Apple M1 Pro, 8 cores
memory: 16 GiB
gpu/display: Chipset Model: Apple M1 Pro
gpu/display: Total Number of Cores: 14
gpu/display: Display Type: Built-in Liquid Retina XDR Display
gpu/display: Resolution: 3024 x 1964 Retina
```

| headers | crossOriginIsolated | SharedArrayBuffer | invoke | fetch tile (CORP same-origin) | `<img>` tile (CORP same-origin) | `<img>` tile (CORP cross-origin) | page origin |
| --- | --- | --- | --- | --- | --- | --- | --- |
| off | false | unavailable | ok | ok | ok | ok | tauri://localhost |
| on | true | unavailable | ok | ok | ok | ok | tauri://localhost |

In-flight sweep (512 px, DPR 2, one run each), burst tiles/s:

| transport | n = 4 | n = 8 | n = 16 |
| --- | --: | --: | --: |
| channel | 932 | 989.7 | 950.5 |
| response | 849.6 | 979.6 | 923.1 |
| scheme-raw | 1280 | 979.6 | 1103.4 |
| scheme-png | 676.1 | 755.9 | 813.6 |
| scheme-webp | 941.2 | 1090.9 | 914.3 |
| scheme-qoi | 780.5 | 744.2 | 744.2 |

### macos-m1pro: pdf-256-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 1391 | 218 | 4.0 | 9.0 | 1 | 10 | 51 | 123 | 21 | 0.00 |
| response | 8 | 5 | 1548 | 242 | 4.0 | 10.0 | 0 | 10 | 50 | 136 | 20 | 0.00 |
| scheme-raw | 4 | 5 | 1548 | 242 | 4.0 | 6.0 | 0 | 11 | 101 | 123 | 21 | 0.00 |
| scheme-png | 16 | 5 | 1231 | 14 | 4.0 | 14.0 | 0 | 9 | 33 | 123 | 21 | 0.16 |
| scheme-webp | 8 | 5 | 1600 | 6 | 4.0 | 9.0 | 0 | 10 | 50 | 91 | 22 | 0.35 |
| scheme-qoi | 4 | 5 | 1655 | 14 | 3.0 | 7.0 | 0 | 11 | 100 | 121 | 20 | 0.16 |

### macos-m1pro: pdf-256-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 1185 | 245 | 5.0 | 10.0 | 0 | 19 | 166 | 217 | 25 | 0.00 |
| response | 8 | 5 | 1352 | 279 | 6.0 | 10.0 | 0 | 19 | 166 | 224 | 25 | 0.00 |
| scheme-raw | 4 | 5 | 1524 | 315 | 4.0 | 7.0 | 1 | 32 | 334 | 241 | 26 | 0.00 |
| scheme-png | 16 | 5 | 1157 | 4 | 5.0 | 13.0 | 0 | 17 | 84 | 235 | 26 | 0.18 |
| scheme-webp | 8 | 5 | 1811 | 1 | 4.0 | 8.0 | 0 | 19 | 167 | 144 | 26 | 0.38 |
| scheme-qoi | 4 | 5 | 1600 | 6 | 3.0 | 7.0 | 0 | 32 | 334 | 211 | 24 | 0.19 |

### macos-m1pro: pdf-512-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 920 | 491 | 6.0 | 15.0 | 0 | 5 | 28 | 165 | 21 | 0.00 |
| response | 8 | 5 | 942 | 502 | 4.0 | 20.0 | 0 | 5 | 23 | 168 | 21 | 0.00 |
| scheme-raw | 4 | 5 | 988 | 527 | 4.0 | 10.0 | 1 | 5 | 33 | 157 | 20 | 0.00 |
| scheme-png | 16 | 5 | 609 | 22 | 5.0 | 16.0 | 1 | 5 | 19 | 165 | 20 | 0.37 |
| scheme-webp | 8 | 5 | 810 | 9 | 5.0 | 20.0 | 2 | 5 | 39 | 133 | 20 | 1.00 |
| scheme-qoi | 4 | 5 | 717 | 19 | 4.0 | 9.0 | 1 | 6 | 48 | 145 | 19 | 0.37 |

### macos-m1pro: pdf-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 923 | 579 | 6.0 | 15.0 | 1 | 10 | 66 | 270 | 23 | 0.00 |
| response | 8 | 5 | 932 | 584 | 6.0 | 17.0 | 2 | 10 | 51 | 275 | 23 | 0.00 |
| scheme-raw | 4 | 5 | 970 | 608 | 5.0 | 11.0 | 0 | 11 | 100 | 252 | 24 | 0.00 |
| scheme-png | 16 | 5 | 793 | 22 | 6.0 | 29.0 | 4 | 9 | 60 | 228 | 22 | 0.49 |
| scheme-webp | 8 | 5 | 990 | 8 | 5.0 | 14.0 | 0 | 10 | 50 | 186 | 23 | 1.06 |
| scheme-qoi | 4 | 5 | 744 | 16 | 5.0 | 11.0 | 0 | 11 | 99 | 233 | 21 | 0.48 |

### macos-m1pro: synthetic-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 8 | 5 | 881 | 612 | 5.0 | 12.0 | 1 | 31 | 56 | 382 | 31 | 0.00 |
| response | 8 | 5 | 923 | 642 | 5.0 | 12.0 | 0 | 31 | 50 | 418 | 33 | 0.00 |
| scheme-raw | 4 | 5 | 1021 | 710 | 5.0 | 10.0 | 0 | 32 | 100 | 426 | 33 | 0.00 |
| scheme-png | 16 | 5 | 482 | 58 | 6.0 | 20.0 | 4 | 31 | 73 | 367 | 33 | 0.79 |
| scheme-webp | 8 | 5 | 828 | 11 | 5.0 | 10.0 | 0 | 31 | 54 | 244 | 31 | 1.07 |
| scheme-qoi | 4 | 5 | 459 | 65 | 8.0 | 12.0 | 0 | 32 | 100 | 347 | 28 | 0.73 |

## windows-2025

```
date: 2026-11-09T16:06Z
os: Microsoft Windows Server 2025 Datacenter 10.0.26100 (build 26100)
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
cpu: AMD EPYC 9V74 80-Core Processor                , 4 logical cores
memory: 16 GiB
gpu/display: Microsoft Hyper-V Video 1024x768
webview2 runtime: 156.0.4314.58
```

| headers | crossOriginIsolated | SharedArrayBuffer | invoke | fetch tile (CORP same-origin) | `<img>` tile (CORP same-origin) | `<img>` tile (CORP cross-origin) | page origin |
| --- | --- | --- | --- | --- | --- | --- | --- |
| off | false | unavailable | ok | ok | ok | ok | http://tauri.localhost |
| on | true | ok | ok | ok | ok | ok | http://tauri.localhost |

In-flight sweep (512 px, DPR 2, one run each), burst tiles/s:

| transport | n = 4 | n = 8 | n = 16 |
| --- | --: | --: | --: |
| channel | 186 | 201.6 | 212.3 |
| response | 240.4 | 252 | 250.5 |
| scheme-raw | 242.2 | 257.7 | 259.2 |
| scheme-png | 667.1 | 717.5 | 731.7 |
| scheme-webp | 698.7 | 794 | 817 |
| scheme-qoi | 497.9 | 515.3 | 500.3 |

### windows-2025: pdf-256-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 16 | 5 | 443 | 69 | 8.9 | 34.3 | 0 | 10 | 62 | 242 | 0 | 0.00 |
| response | 8 | 5 | 645 | 101 | 6.5 | 14.0 | 0 | 9 | 47 | 241 | 0 | 0.00 |
| scheme-raw | 16 | 5 | 683 | 107 | 6.4 | 25.7 | 0 | 9 | 47 | 243 | 0 | 0.00 |
| scheme-png | 16 | 5 | 1119 | 13 | 5.3 | 17.2 | 0 | 9 | 31 | 231 | 0 | 0.24 |
| scheme-webp | 16 | 5 | 1211 | 4 | 5.1 | 14.4 | 0 | 9 | 31 | 233 | 0 | 0.38 |
| scheme-qoi | 8 | 5 | 1112 | 9 | 5.0 | 9.3 | 0 | 9 | 47 | 243 | 0 | 0.18 |

### windows-2025: pdf-256-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 16 | 5 | 387 | 80 | 18.9 | 41.8 | 0 | 24 | 188 | 428 | 0 | 0.00 |
| response | 8 | 5 | 521 | 108 | 10.0 | 21.2 | 0 | 20 | 172 | 434 | 0 | 0.00 |
| scheme-raw | 16 | 5 | 590 | 122 | 11.4 | 36.5 | 0 | 19 | 156 | 422 | 0 | 0.00 |
| scheme-png | 16 | 5 | 1132 | 4 | 10.9 | 26.0 | 0 | 17 | 94 | 414 | 0 | 0.22 |
| scheme-webp | 16 | 5 | 1176 | 1 | 10.3 | 24.6 | 0 | 16 | 94 | 409 | 0 | 0.39 |
| scheme-qoi | 8 | 5 | 1045 | 4 | 10.7 | 14.3 | 0 | 17 | 125 | 423 | 0 | 0.20 |

### windows-2025: pdf-512-dpr1

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 16 | 5 | 240 | 125 | 14.6 | 45.1 | 0 | 6 | 47 | 245 | 0 | 0.00 |
| response | 8 | 5 | 290 | 151 | 8.3 | 35.6 | 0 | 5 | 47 | 241 | 0 | 0.00 |
| scheme-raw | 16 | 5 | 308 | 161 | 7.9 | 34.4 | 0 | 5 | 47 | 245 | 0 | 0.00 |
| scheme-png | 16 | 5 | 754 | 28 | 5.5 | 13.5 | 0 | 4 | 16 | 219 | 0 | 0.71 |
| scheme-webp | 16 | 5 | 827 | 10 | 5.1 | 11.7 | 0 | 4 | 16 | 216 | 0 | 1.37 |
| scheme-qoi | 8 | 5 | 572 | 16 | 5.3 | 15.9 | 0 | 4 | 16 | 256 | 0 | 0.47 |

### windows-2025: pdf-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 16 | 5 | 211 | 132 | 23.2 | 107.2 | 0 | 18 | 172 | 411 | 0 | 0.00 |
| response | 8 | 5 | 248 | 156 | 19.2 | 51.6 | 0 | 18 | 156 | 412 | 0 | 0.00 |
| scheme-raw | 16 | 5 | 260 | 163 | 16.4 | 89.3 | 0 | 17 | 141 | 410 | 0 | 0.00 |
| scheme-png | 16 | 5 | 724 | 21 | 11.1 | 29.4 | 0 | 9 | 63 | 381 | 0 | 0.51 |
| scheme-webp | 16 | 5 | 701 | 6 | 11.2 | 28.3 | 0 | 9 | 47 | 380 | 0 | 1.46 |
| scheme-qoi | 8 | 5 | 488 | 10 | 12.4 | 28.2 | 0 | 10 | 109 | 436 | 0 | 0.50 |

### windows-2025: synthetic-512-dpr2

| transport | n | runs | burst tiles/s | burst MB/s | ready p50 ms | ready p95 ms | dropped frames | blank frames % | 400 % fill ms | webview RSS MB | CPU % | encode ms |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| channel | 16 | 5 | 185 | 128 | 27.4 | 66.7 | 0 | 66 | 141 | 495 | 0 | 0.00 |
| response | 8 | 5 | 215 | 150 | 22.7 | 39.9 | 0 | 59 | 125 | 490 | 0 | 0.00 |
| scheme-raw | 16 | 5 | 216 | 150 | 21.5 | 54.0 | 0 | 57 | 125 | 530 | 0 | 0.00 |
| scheme-png | 16 | 5 | 441 | 53 | 14.2 | 30.6 | 0 | 38 | 63 | 502 | 0 | 0.92 |
| scheme-webp | 16 | 5 | 654 | 8 | 12.7 | 26.7 | 0 | 32 | 47 | 437 | 0 | 1.55 |
| scheme-qoi | 8 | 5 | 324 | 46 | 17.1 | 36.0 | 0 | 48 | 109 | 507 | 0 | 0.95 |

