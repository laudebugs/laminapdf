### linux-x11

```
date: 2026-11-09T21:38Z
uname: Linux 7.0.0-1017-azure x86_64
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
tauri: version = "2.12.5"
os: Ubuntu 24.04.5 LTS
cpu: AMD EPYC 9V45 96-Core Processor, 4 cores
memory: 15 GiB
webkit2gtk-4.1: 2.52.6
session: XDG_SESSION_TYPE= GDK_BACKEND=x11 DISPLAY=:99 WAYLAND_DISPLAY=
WEBKIT_DISABLE_DMABUF_RENDERER=1
gl: 
unknown
```

| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |
| --- | --- | --: | --: | --: | --- | --: | --: | --: |
| A webview | mixed-50 | 50/50 (100 pages printed for 50) | 25 | 0.673 / 0.746 | no | 10.5 | 2192 / 2044 | 30.3 |
| A webview | mixed-sizes | 1/4 | 4 | 0.909 / 0.921 | no |  |  |  |
| A webview | vectors | 1/1 (2 pages printed for 1) | 1 | 0.818 / 0.818 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 0 | 0.961 / 0.964 | no | 10.7 | 249 / 198 | 30.2 |
| B native raster | mixed-sizes | 4/4 | 0 | 0.997 / 0.998 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.960 / 0.960 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 0 | 0.952 / 0.952 | no | 6.7 | 393 / 323 | 4.0 |
| C native vector | mixed-50 | 50/50 | 0 | 0.949 / 0.951 | no | 1.3 | 186 / 198 | 0.1 |
| C native vector | mixed-sizes | 1/4 | 2 | 0.902 / 0.944 | no |  |  |  |
| C native vector | vectors | 1/1 | 0 | 0.911 / 0.911 | no |  |  |  |

### macos-15-hosted

```
date: 2026-11-09T21:45Z
uname: Darwin 24.6.0 arm64
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
tauri: version = "2.12.5"
os: macOS 15.8.2 (24H232); WKWebView ships with the OS
cpu: Apple M1 (Virtual), 3 cores
memory: 7 GiB
```

| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |
| --- | --- | --: | --: | --: | --- | --: | --: | --: |
| A webview | mixed-50 | 50/50 | 0 | 0.958 / 0.959 | no | 13.1 | 1762 / 2372 | 37.8 |
| A webview | mixed-sizes | 1/4 | 4 | 0.904 / 0.928 | no |  |  |  |
| A webview | vectors | 1/1 | 0 | 0.954 / 0.954 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 0 | 0.958 / 0.959 | no | 11.7 | 1830 / 54 | 37.8 |
| B native raster | mixed-sizes | 2/4 | 3 | 0.894 / 0.957 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.954 / 0.954 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 0 | 0.958 / 0.959 | no | 1.0 | 954 / 872 | 3.8 |
| C native vector | mixed-50 | 50/50 | 50 | 0.936 / 0.938 | no | 0.1 | 217 / 64 | 0.1 |
| C native vector | mixed-sizes | 1/4 | 4 | 0.892 / 0.912 | no |  |  |  |
| C native vector | vectors | 1/1 | 1 | 0.872 / 0.872 | no |  |  |  |

### macos-m1pro

```
date: 2026-11-09T23:45Z
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
| A webview | mixed-50 | 50/50 | 0 | 0.961 / 0.964 | no | 14.3 | 1651 / 1074 | 37.8 |
| A webview | mixed-sizes | 1/4 | 4 | 0.905 / 0.928 | no |  |  |  |
| A webview | vectors | 1/1 | 0 | 0.957 / 0.957 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 0 | 0.958 / 0.959 | no | 10.5 | 1676 / 57 | 37.8 |
| B native raster | mixed-sizes | 2/4 | 3 | 0.894 / 0.957 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.954 / 0.954 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 0 | 0.958 / 0.959 | no | 1.0 | 511 / 310 | 3.8 |
| C native vector | mixed-50 | 50/50 | 50 | 0.936 / 0.938 | no | 0.1 | 218 / 58 | 0.1 |
| C native vector | mixed-sizes | 1/4 | 4 | 0.892 / 0.912 | no |  |  |  |
| C native vector | vectors | 1/1 | 1 | 0.872 / 0.872 | no |  |  |  |

### windows-2025

```
date: 2026-11-09T21:44Z
os: Microsoft Windows Server 2025 Datacenter 10.0.26100 (build 26100)
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
cpu: INTEL(R) XEON(R) PLATINUM 8573C, 4 logical cores
memory: 16 GiB
webview2 runtime: 156.0.4314.58
printers: Microsoft Print to PDF
```

| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |
| --- | --- | --: | --: | --: | --- | --: | --: | --: |
| A webview | mixed-50 | 0/50 | 50 | 0.782 / 0.797 | no | 13.3 | 1552 / 840 | 31.1 |
| A webview | mixed-sizes | 1/4 | 3 | 0.893 / 0.944 | no |  |  |  |
| A webview | vectors | 0/1 | 1 | 0.864 / 0.864 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 0 | 0.979 / 0.982 | no | 31.0 | 130 / 281 | 41.6 |
| B native raster | mixed-sizes | 4/4 | 0 | 0.994 / 0.999 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.976 / 0.976 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 0 | 0.979 / 0.981 | no | 3.2 | 132 / 512 | 4.6 |
| C native vector | mixed-50 | 50/50 | 0 | 0.980 / 0.980 | no | 43.4 | 65 / 279 | 16.6 |
| C native vector | mixed-sizes | 4/4 | 0 | 0.998 / 0.999 | no |  |  |  |
| C native vector | vectors | 1/1 | 0 | 0.997 / 0.997 | no |  |  |  |

