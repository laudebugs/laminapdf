### linux-x11

```
date: 2026-11-09T21:18Z
uname: Linux 7.0.0-1017-azure x86_64
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
tauri: version = "2.12.5"
os: Ubuntu 24.04.5 LTS
cpu: AMD EPYC 9V74 80-Core Processor, 4 cores
memory: 15 GiB
webkit2gtk-4.1: 2.52.6
session: XDG_SESSION_TYPE= GDK_BACKEND=x11 DISPLAY=:99 WAYLAND_DISPLAY=
WEBKIT_DISABLE_DMABUF_RENDERER=1
gl: 
unknown
```

| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |
| --- | --- | --: | --: | --: | --- | --: | --: | --: |
| A webview | mixed-50 | 50/50 (100 pages printed for 50) | 50 | 0.693 / 0.746 | no | 13.6 | 2019 / 2053 | 30.9 |
| A webview | mixed-sizes | 1/4 | 4 | 0.912 / 0.923 | no |  |  |  |
| A webview | vectors | 1/1 (2 pages printed for 1) | 1 | 0.818 / 0.818 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 0 | 0.961 / 0.964 | no | 13.5 | 255 / 198 | 30.2 |
| B native raster | mixed-sizes | 4/4 | 0 | 0.997 / 0.998 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.960 / 0.960 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 0 | 0.952 / 0.952 | no | 8.3 | 391 / 519 | 4.0 |
| C native vector | mixed-50 | 50/50 | 0 | 0.949 / 0.951 | no | 1.3 | 186 / 198 | 0.1 |
| C native vector | mixed-sizes | 1/4 | 2 | 0.900 / 0.936 | no |  |  |  |
| C native vector | vectors | 1/1 | 0 | 0.911 / 0.911 | no |  |  |  |

### macos-15-hosted

```
date: 2026-11-09T20:29Z
uname: Darwin 24.6.0 arm64
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
tauri: version = "2.12.5"
os: macOS 15.8.2 (24H232); WKWebView ships with the OS
cpu: Apple M1 (Virtual), 3 cores
memory: 7 GiB
```

| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |
| --- | --- | --: | --: | --: | --- | --: | --: | --: |
| A webview | mixed-50 | 50/50 | 0 | 0.958 / 0.959 | no | 20.1 | 1978 / 2307 | 37.8 |
| A webview | mixed-sizes | 1/4 | 4 | 0.909 / 0.925 | no |  |  |  |
| A webview | vectors | 1/1 | 0 | 0.954 / 0.954 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 25 | 0.943 / 0.952 | no | 14.0 | 3491 / 41 | 37.8 |
| B native raster | mixed-sizes | 2/4 | 3 | 0.894 / 0.957 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.954 / 0.954 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 2 | 0.943 / 0.958 | no | 2.3 | 1114 / 741 | 3.8 |
| C native vector | mixed-50 | 50/50 | 50 | 0.936 / 0.938 | no | 0.7 | 218 / 65 | 0.1 |
| C native vector | mixed-sizes | 1/4 | 4 | 0.895 / 0.911 | no |  |  |  |
| C native vector | vectors | 1/1 | 1 | 0.872 / 0.872 | no |  |  |  |

### macos-m1pro

```
date: 2026-11-09T20:21Z
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

| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |
| --- | --- | --: | --: | --: | --- | --: | --: | --: |
| A webview | mixed-50 | 50/50 | 0 | 0.961 / 0.964 | no | 11.6 | 1737 / 1582 | 37.8 |
| A webview | mixed-sizes | 1/4 | 4 | 0.909 / 0.927 | no |  |  |  |
| A webview | vectors | 1/1 | 0 | 0.957 / 0.957 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 25 | 0.943 / 0.952 | no | 8.8 | 3287 / 60 | 37.8 |
| B native raster | mixed-sizes | 2/4 | 3 | 0.894 / 0.957 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.954 / 0.954 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 2 | 0.943 / 0.958 | no | 1.3 | 714 / 216 | 3.8 |
| C native vector | mixed-50 | 50/50 | 50 | 0.936 / 0.938 | no | 0.6 | 224 / 60 | 0.1 |
| C native vector | mixed-sizes | 1/4 | 4 | 0.895 / 0.911 | no |  |  |  |
| C native vector | vectors | 1/1 | 1 | 0.872 / 0.872 | no |  |  |  |

### windows-2025

```
date: 2026-11-09T20:29Z
os: Microsoft Windows Server 2025 Datacenter 10.0.26100 (build 26100)
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
cpu: AMD EPYC 7763 64-Core Processor                , 4 logical cores
memory: 16 GiB
webview2 runtime: 156.0.4314.58
printers: Microsoft Print to PDF
```

| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |
| --- | --- | --: | --: | --: | --- | --: | --: | --: |
| A webview | mixed-50 | 0/50 | 50 | 0.782 / 0.797 | no | 15.6 | 1405 / 861 | 31.1 |
| A webview | mixed-sizes | 1/4 | 3 | 0.887 / 0.946 | no |  |  |  |
| A webview | vectors | 0/1 | 1 | 0.863 / 0.863 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 0 | 0.979 / 0.982 | no | 31.4 | 127 / 286 | 41.6 |
| B native raster | mixed-sizes | 4/4 | 0 | 0.994 / 0.999 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.976 / 0.976 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 0 | 0.979 / 0.981 | no | 3.8 | 130 / 517 | 4.6 |
| C native vector | mixed-50 | 50/50 | 0 | 0.980 / 0.980 | no | 50.5 | 63 / 292 | 16.6 |
| C native vector | mixed-sizes | 4/4 | 0 | 0.998 / 0.999 | no |  |  |  |
| C native vector | vectors | 1/1 | 0 | 0.997 / 0.997 | no |  |  |  |

