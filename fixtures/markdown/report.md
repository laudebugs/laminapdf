---
title: The reader's field notes
lang: en
author: The reader's team
subject: What a year of offline reading taught us
---

# The reader's field notes

This report is written in Markdown and set by Lamina PDF. It uses *every* construct the
builder reads: **strong** and *emphasised* words, `inline code`, ~~struck-out text~~, links to
[the website](https://laminapdf.com) and to [a later section](#what-comes-next), and a
footnote.[^offline]

## Contents of a day

A reader opens a file, searches it, prints a page and closes it again. Each step stays on the
device.  
This second line follows a hard break, in the same paragraph.

![A test photo: a grid of coloured squares, orange at the top left fading to blue at the bottom right](photo.jpg)

## Numbers

| Month | Files opened | Pages printed |
| :--- | ---: | ---: |
| January | 120 | 40 |
| February | 135 | 52 |
| March | 150 | 61 |
| April | 162 | 58 |
| May | 171 | 66 |
| June | 180 | 70 |
| July | 176 | 64 |
| August | 190 | 72 |
| September | 214 | 80 |
| October | 240 | 91 |
| November | 275 | 104 |
| December | 310 | 118 |

The table above runs across pages when it doesn't fit, and its header row repeats.

## What people asked for

1. Search that finds words in scanned pages
2. Printing that keeps the page size
3. Signing without an account

- [x] Open files without a network
- [ ] Fill in forms
- [ ] Compare two versions

### In their words

> People don't want their files to leave the computer. They want the reader to work on a
> plane, in a hospital and at home in the same way.
>
> — a reader, writing to the team

---

## Other languages

The reader is used in many languages. Here is the same sentence in Arabic and in Hindi.

يعمل القارئ دون اتصال بالشبكة، فتُفتح الملفات وتُقرأ وتُبحث على الجهاز نفسه.

नमस्ते दुनिया। हिन्दी पाठ का परीक्षण।

## What comes next

The next release adds forms. A short program shows how a file is opened:

```
let doc = Document::open_bytes(bytes, OpenOptions::default())?;
println!("{} pages", doc.page_count()?);
```

[^offline]: Offline here means that no part of a file is sent over the network.
