import json,re,random,sys,collections
# V2: a named reader or the words reader/viewer (not bare "pdf")
V2=r'(pdf (reader|viewer)s?|(reader|viewer)s?\b|acrobat|adobe|sumatra\w*|preview(\.app)?|okular|zathura|evince|foxit|\bskim\b|mupdf|pdf\.?js|pdfium|xodo|pdf expert|sioyek|goodreader|koreader|xpdf|qpdfview|atril|drawboard|xchange|zotero|firefox|chrome|\bedge\b)'
PDF=r'(pdfs?|papers?|documents?|textbooks?|annotat\w*|datasheets?|manuals?)'
T={
# ---- wants
'speed_light': (r"\b(fast(er|est)?|snappy|zippy|lightweight|light-weight|lightning|starts? (up )?(instantly|quickly|immediately)|opens? (instantly|quickly|immediately|in (under )?a second)|instant(ly)? (open|start|load)|low overhead|small footprint|tiny (install|footprint|binary))\b",120,V2),
'annotation': (r"\b(annotat\w*|highlight(s|ed|ing)?|mark(ing)?[- ]?up|markups?|sticky notes?|(apple )?pencil|stylus|handwritten notes?)\b",150,V2+'|'+PDF),
'dark_mode': (r"\b(dark[- ]mode|night[- ]mode|dark theme|invert(s|ed|ing)?( the)? colou?rs?|colou?r inver\w*|invert(s|ed|ing)? (the )?(pdf|page|document|text)s?|recolou?r\w*|white text on (a )?black|(dark|black) background)\b",150,V2+'|pdfs?|papers?|documents?'),
'reflow_small_screen': (r"\b(re-?flow\w*|liquid mode|reading mode|reader mode|k2pdfopt|small(er)? screens?|phone screens?|mobile screens?|on (my |a |your )?(phone|mobile)|two[- ]column|2[- ]column|single[- ]column|crop(ping|ped|s)? (the )?margins?|pinch(-| )?(to-? ?)?zoom)\b",150,PDF+'|'+V2),
'keyboard_vim': (r"\b(vim[- ]?(like|style|ish)|vim[- ](keys?|keybind\w*|bindings?|motions?|navigation|mode)|vi[- ]like|vi keys|hjkl|key ?bindings?|keybinds?|keyboard[- ](navigation|shortcuts?|driven|centric|centred|centered|only|friendly|control|based)|keyboard-shortcut\w*|shortcuts?|hotkeys?|vimium|tridactyl)\b",150,V2),
'remember_position': (r"(remember\w* (the |my |your |where |its )?(last |reading )?(position|page|place|location|spot|where)|last (read )?(position|page)|where (i|you) left off|reading (position|progress)|(save|keep)s? (your|my|the) (place|position|page)|resume (reading|where)|opens? (it )?at the (position|page)|remembers? (opened|open) (files|tabs))",0,None),
'tabs': (r"\b(tabs|tabbed|tab (bar|support|switching))\b",80,r'(sumatra\w*|okular|zathura|evince|foxit|\bskim\b|preview\.app|acrobat|pdf expert|sioyek|qpdfview|pdf (reader|viewer)|xchange|mupdf)'),
'search': (r"(search(ing)? (in|within|inside|through|across) (the |a |large |big |long |my |a whole )?(pdf|document|file|text|book)s?|text search|search (results|matches|index|speed|panel|sidebar|function\w*|feature|history|box|widget|story)|find (all|in page|bar)|ctrl[- +]?f\b|(fast|quick|slow)(er)? search|search(es|ing)? (is |are )?(fast|quick|slow|instant)|search(es)? (large|big|long)|(fast|quick)(ly)? (to )?search|searching (is|was) (fast|slow|quick))",0,None),
'citations_links': (r"\b(clickable (links|references|citations)|internal links?|jump(ing|s)? (back|to (the )?(reference|citation|figure|definition|bibliography))|back button|link previews?|preview(s|ing)? (of )?(the )?(references?|links?|citations?|figures?)|hover\w* over|portals?|citations? (pop\w*|previews?|cards?)|pop(s|ping)?[- ]?up\w* (a |the )?(reference|citation|figure)s?|click(ing)? (on )?(a |the )?(citation|reference|link)s?|follow(ing)? (a |the )?(internal )?(link|reference|citation)s?|go(ing)? back to where)\b",150,V2+'|'+PDF),
'auto_reload_latex': (r"\b(auto[- ]?reload\w*|auto[- ]?refresh\w*|reloads? (the |a )?(pdf|file|document|automatically|on change)|live[- ]?reload\w*|locks? the file|file (is )?locked|synctex|vimtex|inverse search|forward search|watches the file)\b",0,None),
'form_fill': (r"\b(form[- ]?fill\w*|fill(ing|ed|s)? (in |out )?(a |the |pdf |these |those |government |tax |some )?forms?|fillable|pdf forms|xfa|acro ?forms?|forms? (in|with) (preview|acrobat|okular|evince|firefox|chrome|edge|foxit))\b",0,None),
'sign': (r"\b(sign(ing|ed)? (a |the |my )?(pdf|document|contract|form|lease)s?|signatures?|e-?sign\w*|digital(ly)? sign\w*)\b",150,V2),
'page_layout': (r"\b(two[- ]page|2[- ]page|side[- ]by[- ]side|facing pages?|book (mode|view|layout)|continuous (scroll\w*|mode|view)|non-continuous|single[- ]page (mode|view)|page at a time|wrapped scrolling|dual[- ]page|double[- ]page|fit (page|width))\b",0,None),
'tablet_eink': (r"\b(ipad\w*|tablets?|e-?ink|eink|e-?readers?|e-?paper|remarkable|boox|onyx|kobo|kindles?|supernote|dpt-\w+|digital paper|koreader|daylight computer)\b",150,r'(pdfs?|papers|textbooks?|annotat\w*)'),
'sync_devices': (r"\b(sync(s|ed|ing|hroni[sz]\w*)?|across (all )?(my |your )?devices|between devices|multiple devices)\b",150,V2+'|'+PDF),
'accessibility': (r"\b(screen[- ]?readers?|accessibility|blind (people|users?|person|reader)s?|visually[- ]impaired|low vision|text[- ]to[- ]speech|tts|read(s)? (it )?aloud|tagged pdfs?|dyslex\w*)\b",0,None),
'text_select_copy': (r"\b(text selection|select(ing)? (the |some )?text|copy(ing)? (the |some )?text|copy[- ]?(and[- ]?|&[- ]?|/)?past\w*|ligatures?|copy (a )?column)\b",150,V2+'|'+PDF),
'huge_files': (r"\b(\d{3,}[- ]?pages?|\d,\d{3}[- ]pages?|thousands of pages|(large|big|huge|massive|long|complex) (pdf|file|document)s?|textbook-sized)\b",150,V2),
# ---- complaints
'slow_bloat': (r"\b(slow(er|est|ly|ness)?|sluggish|bloat(ed|ware)?|takes? (forever|ages|\d+ seconds?|a minute|several seconds)|resource[- ]hog|memory[- ]hog|crawling horror|dog slow|\d{2,4} ?(mb|gb)\b)",120,V2),
'ads_upsell_sub_signin': (r"\b(subscriptions?|subscribe|upsell\w*|nag(s|ging|ware)?|(full of|filled with|built-in|with|shows?|serves?) ads|ads (in|inside|for)|advert\w* (in|inside|for)|sign[- ]?in|log[- ]?in|adobe (id|account)|creative cloud|paywall\w*|in-app purchases?|pro (version|feature|subscription)s?|pay(ing)? (for|to) (open|edit|rotate|use|sign|read)|subscriptionware|adware|dark patterns?|bundled (software|extension)|mcafee|installs? (a |the )?(chrome )?extension|(buy|purchase|upgrade to) (the )?(pro|paid|premium))\b",120,r'(acrobat|adobe|foxit|pdf (reader|viewer|expert)|reader dc|xodo|sumatra|preview|okular|readdle)'),
'security': (r"\b(exploit\w*|vulnerab\w*|cves?|rce|remote code|code execution|malware|malicious|sandbox\w*|attack surface|0[- ]?days?|zero[- ]days?|security (hole|issue|flaw|bug|risk|track record)s?|xss|privilege escalation|attack vector|trojan)\b",150,V2+'|pdfs?'),
'crash_memory': (r"\b(crash\w*|hang(s|ing)?|hung|freez\w*|froze|unresponsive|beach ?ball\w*|memory (usage|use|leak|hog)|(lot of|lots of|tons of|much) (memory|ram)|gigabytes? of (ram|memory)|\d+ ?(gb|mb) of (ram|memory)|oom|out of memory)\b",120,V2),
'render_fidelity': (r"\b(blurr?y|fuzzy|kerning|garbled|subpixel|anti-?alias\w*|render\w* (wrong|incorrect\w*|badly|poorly|differently|horribly|terribly)|(doesn'?t|didn'?t|won'?t|can'?t|fails? to|failed to|not) (render|display) (it |them |the |this |that |some |properly|correctly)|display(s|ed)? (wrong|incorrect\w*|differently)|missing (fonts?|images?|text)|fonts? (look|looks|render|renders) (bad|wrong|awful|terrible|horrible|different|torn)|looks? (wrong|broken|terrible|awful) in|broken (fonts?|rendering|layout)|rendering (bugs?|issues?|problems?|errors?|glitch\w*))\b",150,V2+'|pdfs?'),
'acrobat_only_docs': (r"(only (works?|opens?|displays?|renders?|shows?) (correctly |properly )?(in|with) (adobe|acrobat)|(requires?|need(s|ed)?|had to (use|install)|have to (use|install)|forced to use) (the )?(adobe|acrobat)|xfa\b|dynamic (pdf )?forms?|please wait\.\.\.|only (adobe|acrobat) (can|supports?|handles?|works))",0,None),
'telemetry_privacy': (r"\b(telemetry|phones? home|phoning home|spyware|tracks? (where|what) (you|i)|tracking (where|what|users?|you)|analytics|privacy[- ](respecting|focused|friendly|invasive|issues?|nightmare|concerns?)|data collection|no tracking|upload(s|ed|ing)? (my |your |the )?(files?|documents?|pdfs?) to|without uploading|never leaves? (your|the) (device|computer|machine)|runs? (entirely )?(locally|client[- ]side|in the browser))\b",120,V2+'|pdfs?'),
'print_problems': (r"(print(ing)? (fails?|failed|issues?|problems?|is broken|broken|is blurry|blurry|dialog|preview|quality|settings|options)|can'?t print|won'?t print|prints? (wrong|differently|blurry|bitmaps?|a different)|blurry print\w*|print (out )?(correctly|properly))",120,V2),
'ai_in_reader': (r"\b(ai (assistant|toolbar|features?|button|summar\w*|chat|reader|sidebar|pdf)|copilot|chat with (your |a |the )?(pdf|document)s?|chatpdf|ask (the |your )?pdf|summari[sz]\w* (the |a |this |any |your )?(pdf|document|paper|article)s?|llm[- ](based|powered)|ai[- ](powered|enhanced|based)|(pdf|reader|viewer) with (an? )?(ai|llm))\b",150,V2+'|pdfs?'),

'multi_format': (r"\b(epubs?|djvu|cbz|cbr|mobi|chm|xps|comics?)\b",100,r'(sumatra\w*|okular|zathura|evince|foxit|mupdf|koreader|sioyek|foliate|qpdfview|atril|(pdf|document) (reader|viewer)s?|universal (document )?viewer)'),
'toc_outline': (r"\b(table of contents|toc|outlines?|bookmarks? (panel|sidebar|pane|tree|list)|document outline|sidebar (with|of) (the )?(outline|bookmarks|toc)|thumbnails?)\b",120,V2),
'smooth_zoom_scroll': (r"\b(smooth(ly)?[- ]?(scroll\w*|zoom\w*|pan\w*)|(scroll\w*|zoom\w*|pan\w*) (is |feels |was )?(smooth|choppy|janky|laggy|sluggish|jerky)|choppy scroll\w*|zoom(ing)? (is |feels )?(fluid|fast|slow)|pinch[- ]to[- ]zoom|zoom (focus|point)|marquee zoom)\b",120,V2+'|pdfs?'),
'portable_noinstall': (r"\b(portable (version|app|exe|edition)|no install\w*|without install\w*|single (exe|executable|binary)|no admin\w*|without admin\w*|runs? from a usb|tiny installer|small installer)\b",120,V2),
'open_source_free': (r"\b(open[- ]source|foss|floss|free software|libre|gpl|mit[- ]licen[cs]ed|free (and|&) open)\b",120,V2),
'browser_viewer': (r"(pdf\.?js|pdfjs|(firefox|chrome|chromium|edge|safari|browser)('s|s)? (built[- ]in |builtin |embedded |integrated |native |internal |own )?(pdf )?(viewer|reader|renderer)|(built[- ]in|builtin|embedded|in-browser|browser) pdf (viewer|reader|support))",0,None),

'save_damage': (r"\b(corrupt\w*|destroy(s|ed|ing)?|damag\w*|mangl\w*|strip(s|ped|ping)? (the )?(ocr|text layer|annotations?|metadata)|(breaks?|broke|ruin\w*) (the |my )?(pdf|file|document|text layer|ocr)s?|scrambled)\b",120,r'(preview|acrobat|sav(e|ed|ing)|okular|foxit|evince|reader|viewer)'),
}
TR={k:(re.compile(v[0],re.I),v[1],re.compile(v[2],re.I) if v[2] else None) for k,v in T.items()}
def period(d):
    y=int(d[:4]); return '2016-19' if y<=2019 else ('2020-22' if y<=2022 else '2023-26')
def hit(rx,w,ctx,t):
    for m in rx.finditer(t):
        if not ctx: return m
        a=max(0,m.start()-w); b=m.end()+w
        if ctx.search(t[a:b]): return m
    return None
if __name__=='__main__':
    # usage: themes.py                 -> counts per theme and period over areas/readers.jsonl
    #        themes.py - <theme> [N]   -> also print N random matches of <theme> (spot-check)
    import os
    HN=os.environ.get("HN_DATA", ".")
    src=os.path.join(HN,'areas','readers.jsonl')
    rows=[json.loads(l) for l in open(src)]
    tot=collections.Counter(period(o['date']) for o in rows)
    res={}; ex=collections.defaultdict(list)
    for k,(rx,w,ctx) in TR.items():
        c=collections.Counter()
        for o in rows:
            m=hit(rx,w,ctx,o['text'])
            if m:
                c[period(o['date'])]+=1
                ex[k].append((o['id'],o['date'],o['text'][max(0,m.start()-110):m.end()+110].replace('\n',' ')))
        res[k]=c
    print('period totals',dict(tot))
    print(f"{'theme':24s} {'all':>5s} {'16-19':>12s} {'20-22':>12s} {'23-26':>12s}")
    for k,c in sorted(res.items(),key=lambda x:-sum(x[1].values())):
        s=sum(c.values()); f=lambda p: f"{c[p]:4d} ({100*c[p]/tot[p]:4.1f}%)"
        print(f"{k:24s} {s:5d} {f('2016-19')} {f('2020-22')} {f('2023-26')}")
    if len(sys.argv)>2:
        random.seed(11); k=sys.argv[2]; n=int(sys.argv[3]) if len(sys.argv)>3 else 10
        for i,d,t in random.sample(ex[k],min(n,len(ex[k]))): print(f'{i} {d}: {t[:240]}')
