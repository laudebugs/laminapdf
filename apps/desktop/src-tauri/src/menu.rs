//! The native menus (Paper S03), built from `menus.json`, which the webview's menu
//! layout and command registry generate (`apps/desktop/src/menus`): an item's title and keys are
//! the registry's, so the menus, the palette and the shortcut help can't disagree.
//!
//! macOS has one menu bar for the app; on Windows and Linux each window shows the same menus
//! (Tauri's app-wide menu). Choosing a command sends it to the window focused last, which runs
//! it through its registry as a key press or the palette would ([`COMMAND_EVENT`]). Rust handles
//! the rest itself: the OS's own items (Quit, Hide, Copy…), the window list, Open recent's files,
//! the Help menu's links, and, with no window open, Open and New window.
//!
//! Each window reports which of its commands are enabled and ticked (`menu_state`), and the menus
//! show the focused window's. Their shape changes only when the windows or the recent list do.
//! Menu calls run on the main thread, so every change is posted there and done in order.

use std::collections::BTreeMap;
#[cfg(any(test, feature = "e2e"))]
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use tauri::menu::{AboutMetadata, CheckMenuItem, Menu, MenuEvent, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter as _, EventTarget, Manager as _, Wry};
use tauri_plugin_opener::OpenerExt as _;

use crate::recent::RecentFiles;
use crate::windows::{self, WindowEntry, Windows};

/// A menu command for the focused window: `{ id, arg }`. The name is repeated in
/// `src/menus/use-native-menus.tsx`.
pub(crate) const COMMAND_EVENT: &str = "menu://command";

/// The menus as data, generated from the webview's layout and registry.
const MENUS_JSON: &str = include_str!("../menus.json");

/// The commands that work with no window open (macOS keeps running without one): Rust opens a
/// window for them.
const WITHOUT_WINDOW: [&str; 2] = ["file.open", "window.new"];

/// The platform a menu or an item is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Os {
    /// macOS.
    Mac,
    /// Windows and Linux.
    Other,
}

impl Os {
    /// The platform this build runs on.
    pub(crate) const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Other
        }
    }
}

/// `menus.json`.
#[derive(Debug, Deserialize)]
pub(crate) struct MenusJson {
    menus: Vec<MenuJson>,
}

impl MenusJson {
    /// The menus this build embeds.
    ///
    /// # Panics
    /// When `menus.json` doesn't parse: its test in `src/menus` and this module's tests catch
    /// that before a build ships.
    pub(crate) fn embedded() -> Self {
        #[expect(clippy::expect_used, reason = "the embedded file is checked by the tests")]
        serde_json::from_str(MENUS_JSON).expect("menus.json is the generator's output")
    }
}

#[derive(Debug, Deserialize)]
struct MenuJson {
    title: String,
    on: Option<Os>,
    items: Vec<ItemJson>,
}

#[derive(Debug, Deserialize)]
struct ItemJson {
    on: Option<Os>,
    #[serde(flatten)]
    kind: KindJson,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum KindJson {
    Command {
        id: String,
        title: String,
        mac: Option<String>,
        other: Option<String>,
        #[serde(default)]
        check: bool,
    },
    Choices {
        id: String,
        title: String,
        choices: Vec<ChoiceJson>,
    },
    Predefined {
        name: Predefined,
    },
    Link {
        title: String,
        url: String,
    },
    Placeholder {
        title: String,
    },
    Recent {
        title: String,
        empty: String,
        clear: String,
    },
    MoveToWindow {
        title: String,
        empty: String,
    },
    Windows,
    Separator,
}

#[derive(Debug, Deserialize)]
struct ChoiceJson {
    value: String,
    title: String,
}

/// An item the OS provides: its title, keys and work are the OS's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Predefined {
    About,
    Services,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    Copy,
    SelectAll,
    Minimize,
    Zoom,
    FullScreen,
    BringAllToFront,
}

/// What a window says of one of its commands: whether it can run now, and, for a toggle or a
/// choice, whether it is on or which value is chosen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CommandState {
    pub(crate) id: String,
    pub(crate) enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) checked: Option<Checked>,
}

/// A toggle's state or a choice's value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum Checked {
    On(bool),
    Value(String),
}

/// A menu item, after the platform and the focused window's state are applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Item {
    /// What [`on_menu_event`] reads: `cmd:<command>`, `cmd:<command>=<value>`, `recent:<id>`,
    /// `move:<label>`, `window:<label>`, `link:<url>`.
    pub(crate) id: String,
    pub(crate) title: String,
    /// Muda's notation for this platform: `CmdOrCtrl+Shift+N`.
    pub(crate) accelerator: Option<String>,
    pub(crate) enabled: bool,
    /// `Some` for a check item, ticked or not.
    pub(crate) checked: Option<bool>,
}

/// A node of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Node {
    Item(Item),
    Predefined(Predefined),
    Separator,
    Submenu { title: String, items: Vec<Node> },
}

/// A menu of the menu bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TopMenu {
    pub(crate) title: String,
    pub(crate) items: Vec<Node>,
}

/// What the menus show besides the layout: the recent files (id, name), the open windows, the
/// focused one, and its commands' state (`None` until it reports, and with no window).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Context<'a> {
    pub(crate) os: Os,
    pub(crate) recent: &'a [(String, String)],
    pub(crate) windows: &'a [WindowEntry],
    pub(crate) focused: Option<&'a str>,
    pub(crate) state: Option<&'a [CommandState]>,
}

impl Context<'_> {
    fn command(&self, id: &str) -> Option<&CommandState> {
        self.state?.iter().find(|state| state.id == id)
    }

    /// Whether command `id` can run: as the focused window says, or with no window, only the
    /// commands Rust does then.
    fn enabled(&self, id: &str) -> bool {
        match self.state {
            Some(_) => self.command(id).is_some_and(|state| state.enabled),
            None => self.focused.is_none() && WITHOUT_WINDOW.contains(&id),
        }
    }
}

const fn for_os(on: Option<Os>, os: Os) -> bool {
    match on {
        Some(Os::Mac) => matches!(os, Os::Mac),
        Some(Os::Other) => matches!(os, Os::Other),
        None => true,
    }
}

/// The menu bar for `ctx`, from `menus`.
pub(crate) fn menu_bar(menus: &MenusJson, ctx: &Context<'_>) -> Vec<TopMenu> {
    menus
        .menus
        .iter()
        .filter(|menu| for_os(menu.on, ctx.os))
        .map(|menu| TopMenu {
            // Windows and Linux: Alt and the first letter open the menu (File, Edit, View, Go,
            // Tools, Window and Help start with different letters).
            title: match ctx.os {
                Os::Mac => menu.title.clone(),
                Os::Other => format!("&{}", menu.title),
            },
            items: tidy(
                menu.items
                    .iter()
                    .filter(|item| for_os(item.on, ctx.os))
                    .flat_map(|item| nodes(&item.kind, ctx))
                    .collect(),
            ),
        })
        .collect()
}

/// The nodes one entry of the layout makes.
fn nodes(kind: &KindJson, ctx: &Context<'_>) -> Vec<Node> {
    match kind {
        KindJson::Command {
            id,
            title,
            mac,
            other,
            check,
        } => {
            let keys = match ctx.os {
                Os::Mac => mac,
                Os::Other => other,
            };
            let checked = check.then(|| {
                matches!(
                    ctx.command(id).and_then(|s| s.checked.as_ref()),
                    Some(Checked::On(true))
                )
            });
            vec![Node::Item(Item {
                id: format!("cmd:{id}"),
                title: title.clone(),
                accelerator: keys.as_deref().map(|keys| accelerator(keys, ctx.os)),
                enabled: ctx.enabled(id),
                checked,
            })]
        }
        KindJson::Choices { id, title, choices } => {
            let chosen = match ctx.command(id).and_then(|s| s.checked.as_ref()) {
                Some(Checked::Value(value)) => Some(value.as_str()),
                _ => None,
            };
            vec![Node::Submenu {
                title: title.clone(),
                items: choices
                    .iter()
                    .map(|choice| {
                        Node::Item(Item {
                            id: format!("cmd:{id}={}", choice.value),
                            title: choice.title.clone(),
                            accelerator: None,
                            enabled: ctx.enabled(id),
                            checked: Some(chosen == Some(choice.value.as_str())),
                        })
                    })
                    .collect(),
            }]
        }
        KindJson::Predefined { name } => vec![Node::Predefined(*name)],
        KindJson::Link { title, url } => vec![plain(format!("link:{url}"), title, true)],
        KindJson::Placeholder { title } => vec![plain(format!("later:{title}"), title, false)],
        KindJson::Recent { title, empty, clear } => vec![Node::Submenu {
            title: title.clone(),
            items: recent_items(ctx, empty, clear),
        }],
        KindJson::MoveToWindow { title, empty } => vec![Node::Submenu {
            title: title.clone(),
            items: move_items(ctx, empty),
        }],
        KindJson::Windows => ctx
            .windows
            .iter()
            .map(|window| {
                Node::Item(Item {
                    id: format!("window:{}", window.label),
                    title: window.title.clone(),
                    accelerator: None,
                    enabled: true,
                    checked: Some(Some(window.label.as_str()) == ctx.focused),
                })
            })
            .collect(),
        KindJson::Separator => vec![Node::Separator],
    }
}

/// Open recent's items: the files, then Clear menu; or one disabled "No recent files".
fn recent_items(ctx: &Context<'_>, empty: &str, clear: &str) -> Vec<Node> {
    if ctx.recent.is_empty() {
        return vec![plain("recent:none".to_owned(), empty, false)];
    }
    let mut items: Vec<Node> = ctx
        .recent
        .iter()
        .map(|(id, name)| plain(format!("recent:{id}"), name, true))
        .collect();
    items.push(Node::Separator);
    items.push(plain("recent-clear".to_owned(), clear, true));
    items
}

/// Move tab to window's items. The tab can go where "Move tab to new window" could take it
/// from. Every window is listed and the focused one disabled, so focusing another changes no
/// item's place.
fn move_items(ctx: &Context<'_>, empty: &str) -> Vec<Node> {
    if ctx.windows.len() < 2 {
        return vec![plain("move:none".to_owned(), empty, false)];
    }
    let can_move = ctx.enabled("tab.move-to-new-window");
    ctx.windows
        .iter()
        .map(|window| {
            let here = Some(window.label.as_str()) == ctx.focused;
            plain(format!("move:{}", window.label), &window.title, can_move && !here)
        })
        .collect()
}

fn plain(id: String, title: &str, enabled: bool) -> Node {
    Node::Item(Item {
        id,
        title: title.to_owned(),
        accelerator: None,
        enabled,
        checked: None,
    })
}

/// `nodes` without separators at either end or two in a row (an item for the other platform
/// can leave them).
fn tidy(nodes: Vec<Node>) -> Vec<Node> {
    let mut tidy: Vec<Node> = Vec::with_capacity(nodes.len());
    for node in nodes {
        if node == Node::Separator && matches!(tidy.last(), None | Some(Node::Separator)) {
            continue;
        }
        tidy.push(node);
    }
    if tidy.last() == Some(&Node::Separator) {
        tidy.pop();
    }
    tidy
}

/// The registry's keys (`Mod+Shift+N`, `Mod++`, `Alt+ArrowLeft`) in muda's notation for `os`.
/// `Mod` is ⌘ on macOS and Ctrl elsewhere; `+` is the key it shares with `=`, as the browsers
/// and other apps bind ⌘+.
pub(crate) fn accelerator(keys: &str, os: Os) -> String {
    // The key is after the last `+` that isn't the key itself (`Mod++`).
    let (mods, key) = match keys.strip_suffix("++") {
        Some(mods) => (mods, "+"),
        None => keys.rsplit_once('+').unwrap_or(("", keys)),
    };
    let key = match key {
        "+" | "=" => "Equal",
        "-" => "Minus",
        "\\" => "Backslash",
        "[" => "BracketLeft",
        "]" => "BracketRight",
        "," => "Comma",
        "." => "Period",
        "/" => "Slash",
        other => other,
    };
    let mut parts: Vec<&str> = mods
        .split('+')
        .filter(|part| !part.is_empty())
        .map(|part| match (part, os) {
            ("Mod", Os::Mac) => "Cmd",
            ("Mod", Os::Other) => "Ctrl",
            (other, _) => other,
        })
        .collect();
    parts.push(key);
    parts.join("+")
}

/// The menu bar as text, one node a line: the snapshot the tests compare and `e2e_menu_snapshot`
/// returns.
#[cfg(any(test, feature = "e2e"))]
pub(crate) fn render(menus: &[TopMenu]) -> String {
    let mut text = String::new();
    for menu in menus {
        text.push_str(&menu.title);
        text.push('\n');
        render_nodes(&menu.items, 1, &mut text);
    }
    text
}

#[cfg(any(test, feature = "e2e"))]
fn render_nodes(nodes: &[Node], depth: usize, text: &mut String) {
    let indent = "  ".repeat(depth);
    for node in nodes {
        text.push_str(&indent);
        match node {
            Node::Separator => text.push_str("---"),
            Node::Predefined(predefined) => {
                let _ = write!(text, "({predefined:?})");
            }
            Node::Submenu { title, .. } => {
                let _ = write!(text, "{title} ▸");
            }
            Node::Item(item) => {
                text.push_str(&item.title);
                if let Some(keys) = &item.accelerator {
                    let _ = write!(text, " [{keys}]");
                }
                match item.checked {
                    Some(true) => text.push_str(" ✓"),
                    Some(false) => text.push_str(" ☐"),
                    None => {}
                }
                if !item.enabled {
                    text.push_str(" (disabled)");
                }
                let _ = write!(text, "  <{}>", item.id);
            }
        }
        text.push('\n');
        if let Node::Submenu { items, .. } = node {
            render_nodes(items, depth + 1, text);
        }
    }
}

/// The menus' state in the running app: each window's report, and the items last built.
#[derive(Default)]
pub(crate) struct NativeMenus(Mutex<Inner>);

#[derive(Default)]
struct Inner {
    states: BTreeMap<String, Vec<CommandState>>,
    /// The built items, by id, with what each shows now (enabled, ticked).
    items: Built,
}

impl NativeMenus {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        // Only the main thread takes it, one change at a time.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The recent list as the menu shows it: files that are still there, newest first.
fn recent_entries(app: &AppHandle) -> Vec<(String, String)> {
    app.state::<RecentFiles>()
        .list()
        .into_iter()
        .filter(|entry| !entry.missing)
        .map(|entry| (entry.id, entry.name))
        .collect()
}

/// The focused window, if it is still open.
fn focused(app: &AppHandle) -> Option<String> {
    app.state::<Windows>()
        .frontmost()
        .filter(|label| app.get_webview_window(label).is_some())
}

/// The menu bar the app should show now.
fn current(app: &AppHandle, menus: &MenusJson, states: &BTreeMap<String, Vec<CommandState>>) -> Vec<TopMenu> {
    let recent = recent_entries(app);
    let windows = windows::others(app, "");
    let focused = focused(app);
    let ctx = Context {
        os: Os::current(),
        recent: &recent,
        windows: &windows,
        focused: focused.as_deref(),
        state: focused.as_ref().and_then(|label| states.get(label)).map(Vec::as_slice),
    };
    menu_bar(menus, &ctx)
}

/// On the main thread: the menu bar the app should show now, with the menus' state.
fn with_bar(app: &AppHandle, work: impl FnOnce(&AppHandle, &mut Inner, &[TopMenu]) + Send + 'static) {
    on_main(app, |app| {
        let menus = app.state::<MenusJson>();
        let native = app.state::<NativeMenus>();
        let mut inner = native.lock();
        let bar = current(app, &menus, &inner.states);
        work(app, &mut inner, &bar);
    });
}

/// Builds the menus again (the windows or the recent list changed) and shows them.
pub(crate) fn rebuild(app: &AppHandle) {
    with_bar(app, |app, inner, bar| match build(app, bar) {
        Ok((menu, items)) => {
            if let Err(error) = app.set_menu(menu) {
                tracing::warn!("could not show the menus: {error}");
            }
            inner.items = items;
        }
        Err(error) => tracing::warn!("could not build the menus: {error}"),
    });
}

/// Shows the focused window's state in the menus as they are: what is enabled and ticked.
pub(crate) fn refresh(app: &AppHandle) {
    with_bar(app, |_, inner, bar| {
        let mut stack: Vec<&Node> = bar.iter().flat_map(|menu| menu.items.iter()).collect();
        while let Some(node) = stack.pop() {
            match node {
                Node::Submenu { items, .. } => stack.extend(items.iter()),
                Node::Item(item) => {
                    if let Some(shown) = inner.items.get_mut(&item.id) {
                        shown.show(item);
                    }
                }
                Node::Predefined(_) | Node::Separator => {}
            }
        }
    });
}

/// A built item and what it shows now.
struct Shown {
    built: MenuItemKind<Wry>,
    enabled: bool,
    checked: Option<bool>,
}

impl Shown {
    /// Brings the item to `item`'s enabled and ticked state, calling the OS only for a change.
    fn show(&mut self, item: &Item) {
        let result = (|| {
            if self.enabled != item.enabled {
                match &self.built {
                    MenuItemKind::MenuItem(menu_item) => menu_item.set_enabled(item.enabled)?,
                    MenuItemKind::Check(check) => check.set_enabled(item.enabled)?,
                    _ => {}
                }
                self.enabled = item.enabled;
            }
            if self.checked != item.checked {
                if let (MenuItemKind::Check(check), Some(on)) = (&self.built, item.checked) {
                    check.set_checked(on)?;
                }
                self.checked = item.checked;
            }
            tauri::Result::Ok(())
        })();
        if let Err(error) = result {
            tracing::warn!("could not update a menu item: {error}");
        }
    }
}

/// Window `label`'s commands' state: kept, and shown while it is the focused window.
pub(crate) fn set_state(app: &AppHandle, label: &str, state: Vec<CommandState>) {
    let label = label.to_owned();
    on_main(app, move |app| {
        app.state::<NativeMenus>().lock().states.insert(label, state);
    });
    refresh(app);
}

/// Window `label` closed: its state goes, and the window list with it.
pub(crate) fn closed(app: &AppHandle, label: &str) {
    let label = label.to_owned();
    on_main(app, move |app| {
        app.state::<NativeMenus>().lock().states.remove(&label);
    });
}

fn on_main(app: &AppHandle, work: impl FnOnce(&AppHandle) + Send + 'static) {
    let handle = app.clone();
    if let Err(error) = app.run_on_main_thread(move || work(&handle)) {
        tracing::warn!("could not reach the main thread for the menus: {error}");
    }
}

/// The built items by id.
type Built = BTreeMap<String, Shown>;

/// The menu bar as Tauri menus, and its items by id.
fn build(app: &AppHandle, bar: &[TopMenu]) -> tauri::Result<(Menu<Wry>, Built)> {
    let menu = Menu::new(app)?;
    let mut items = Built::new();
    for top in bar {
        let submenu = Submenu::new(app, &top.title, true)?;
        append(app, &submenu, &top.items, &mut items)?;
        menu.append(&submenu)?;
    }
    Ok((menu, items))
}

fn append(app: &AppHandle, parent: &Submenu<Wry>, nodes: &[Node], items: &mut Built) -> tauri::Result<()> {
    for node in nodes {
        match node {
            Node::Separator => parent.append(&PredefinedMenuItem::separator(app)?)?,
            Node::Predefined(predefined) => parent.append(&predefined_item(app, *predefined)?)?,
            Node::Submenu { title, items: nodes } => {
                let submenu = Submenu::new(app, title, true)?;
                append(app, &submenu, nodes, items)?;
                parent.append(&submenu)?;
            }
            Node::Item(item) => {
                let keys = item.accelerator.as_deref();
                let built = match item.checked {
                    None => MenuItemKind::MenuItem(MenuItem::with_id(app, &item.id, &item.title, item.enabled, keys)?),
                    Some(on) => MenuItemKind::Check(CheckMenuItem::with_id(
                        app,
                        &item.id,
                        &item.title,
                        item.enabled,
                        on,
                        keys,
                    )?),
                };
                match &built {
                    MenuItemKind::MenuItem(menu_item) => parent.append(menu_item)?,
                    MenuItemKind::Check(check) => parent.append(check)?,
                    _ => {}
                }
                items.insert(
                    item.id.clone(),
                    Shown {
                        built,
                        enabled: item.enabled,
                        checked: item.checked,
                    },
                );
            }
        }
    }
    Ok(())
}

fn predefined_item(app: &AppHandle, predefined: Predefined) -> tauri::Result<PredefinedMenuItem<Wry>> {
    match predefined {
        Predefined::About => {
            let info = app.package_info();
            PredefinedMenuItem::about(
                app,
                Some("About Lamina PDF"),
                Some(AboutMetadata {
                    name: Some(info.name.clone()),
                    version: Some(info.version.to_string()),
                    copyright: Some("© The Lamina PDF authors. MIT OR Apache-2.0.".to_owned()),
                    website: Some("https://laminapdf.com".to_owned()),
                    ..AboutMetadata::default()
                }),
            )
        }
        Predefined::Services => PredefinedMenuItem::services(app, None),
        Predefined::Hide => PredefinedMenuItem::hide(app, Some("Hide Lamina PDF")),
        Predefined::HideOthers => PredefinedMenuItem::hide_others(app, Some("Hide others")),
        Predefined::ShowAll => PredefinedMenuItem::show_all(app, Some("Show all")),
        Predefined::Quit => PredefinedMenuItem::quit(app, Some("Quit Lamina PDF")),
        Predefined::Copy => PredefinedMenuItem::copy(app, Some("Copy")),
        Predefined::SelectAll => PredefinedMenuItem::select_all(app, Some("Select all")),
        Predefined::Minimize => PredefinedMenuItem::minimize(app, Some("Minimise")),
        Predefined::Zoom => PredefinedMenuItem::maximize(app, Some("Zoom")),
        Predefined::FullScreen => PredefinedMenuItem::fullscreen(app, Some("Full screen")),
        Predefined::BringAllToFront => PredefinedMenuItem::bring_all_to_front(app, Some("Bring all to front")),
    }
}

/// What [`COMMAND_EVENT`] carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct MenuCommand {
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) arg: Option<String>,
}

/// What choosing item `id` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    /// Run a command in the focused window.
    Command(MenuCommand),
    /// Open a recent file, as the OS's open requests do.
    OpenRecent(String),
    /// Bring a window to the front.
    Focus(String),
    /// Open a page in the browser.
    Link(String),
    /// Nothing (a disabled or unknown item).
    None,
}

/// What choosing the item `id` does.
pub(crate) fn action(id: &str) -> Action {
    let command = |id: &str, arg: Option<&str>| {
        Action::Command(MenuCommand {
            id: id.to_owned(),
            arg: arg.map(str::to_owned),
        })
    };
    if let Some(rest) = id.strip_prefix("cmd:") {
        return match rest.split_once('=') {
            Some((id, value)) => command(id, Some(value)),
            None => command(rest, None),
        };
    }
    if id == "recent-clear" {
        return command("file.clear-recent", None);
    }
    match id.split_once(':') {
        Some(("recent" | "move", "none")) => Action::None,
        Some(("recent", recent)) => Action::OpenRecent(recent.to_owned()),
        Some(("move", label)) => command("tab.move-to-window", Some(label)),
        Some(("window", label)) => Action::Focus(label.to_owned()),
        Some(("link", url)) => Action::Link(url.to_owned()),
        _ => Action::None,
    }
}

/// A menu item was chosen.
pub(crate) fn on_menu_event(app: &AppHandle, event: &MenuEvent) {
    run(app, action(event.id().as_ref()));
}

/// Does `action`.
pub(crate) fn run(app: &AppHandle, action: Action) {
    match action {
        Action::Command(command) => send(app, &command),
        Action::OpenRecent(id) => open_recent(app, &id),
        Action::Focus(label) => focus(app, &label),
        Action::Link(url) => {
            if let Err(error) = app.opener().open_url(url, None::<&str>) {
                tracing::warn!("could not open a Help link: {error}");
            }
        }
        Action::None => {}
    }
}

/// Opens the recent file `id` in the focused window, as the OS's open requests do.
fn open_recent(app: &AppHandle, id: &str) {
    let path: Option<PathBuf> = app.state::<RecentFiles>().path_of(id);
    match (path, windows::target_window(app)) {
        (Some(path), Ok(window)) => crate::open::route_open(app, &window, vec![path]),
        (None, _) => tracing::warn!("Open recent: the entry is gone"),
        (_, Err(error)) => tracing::warn!("Open recent: no window: {error}"),
    }
}

/// Brings window `label` to the front (the Window menu's list).
fn focus(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.unminimize();
        if let Err(error) = window.set_focus() {
            tracing::warn!("could not bring a window forward: {error}");
        }
    }
}

/// Sends `command` to the focused window. With none, Open and New window open one.
fn send(app: &AppHandle, command: &MenuCommand) {
    match focused(app) {
        Some(label) => {
            if let Err(error) = app.emit_to(EventTarget::webview_window(&label), COMMAND_EVENT, command) {
                tracing::warn!("could not send a menu command: {error}");
            }
        }
        None if WITHOUT_WINDOW.contains(&command.id.as_str()) => {
            if let Err(error) = windows::new_window(app) {
                tracing::warn!("could not open a window from the menu: {error}");
            }
        }
        None => {}
    }
}

/// The menus as they show now, as [`render`] writes them.
#[cfg(feature = "e2e")]
pub(crate) fn snapshot(app: &AppHandle) -> String {
    let menus = app.state::<MenusJson>();
    let native = app.state::<NativeMenus>();
    let states = native.lock().states.clone();
    render(&current(app, &menus, &states))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(label: &str, title: &str) -> WindowEntry {
        WindowEntry {
            label: label.to_owned(),
            title: title.to_owned(),
        }
    }

    fn state(id: &str, enabled: bool, checked: Option<Checked>) -> CommandState {
        CommandState {
            id: id.to_owned(),
            enabled,
            checked,
        }
    }

    /// Two windows, two recent files, a document open in `main` with zoom, tabs and settings
    /// enabled, two pages chosen and focus mode on.
    fn snapshot_for(os: Os) -> String {
        let recent = [
            ("r1".to_owned(), "report.pdf".to_owned()),
            ("r2".to_owned(), "invoice.pdf".to_owned()),
        ];
        let windows = [entry("main", "report.pdf — Lamina PDF"), entry("doc-1", "Lamina PDF")];
        let states = [
            state("file.open", true, None),
            state("window.new", true, None),
            state("window.close", true, None),
            state("tab.close", true, None),
            state("tab.move-to-new-window", true, None),
            state("view.zoom-in", true, None),
            state("view.zoom-out", true, None),
            state("app.settings", true, None),
            state("app.quit", true, None),
            state("view.focus-mode", true, Some(Checked::On(true))),
            state("view.page-layout", true, Some(Checked::Value("two-up".to_owned()))),
        ];
        let ctx = Context {
            os,
            recent: &recent,
            windows: &windows,
            focused: Some("main"),
            state: Some(&states),
        };
        render(&menu_bar(&MenusJson::embedded(), &ctx))
    }

    /// Compares `actual` with the snapshot in `tests/menus/<name>.txt`; `LAMINA_UPDATE_MENUS=1`
    /// writes it.
    fn check_snapshot(name: &str, actual: &str) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/menus/{name}.txt"));
        if std::env::var_os("LAMINA_UPDATE_MENUS").is_some() {
            std::fs::write(&path, actual).unwrap();
        }
        let expected = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            actual,
            expected,
            "{} is out of date: LAMINA_UPDATE_MENUS=1 cargo test",
            path.display()
        );
    }

    #[test]
    fn macos_menus_match_their_snapshot() {
        check_snapshot("macos", &snapshot_for(Os::Mac));
    }

    #[test]
    fn windows_and_linux_menus_match_their_snapshot() {
        check_snapshot("windows-linux", &snapshot_for(Os::Other));
    }

    #[test]
    fn every_accelerator_parses_as_muda_reads_it() {
        for os in [Os::Mac, Os::Other] {
            for menu in &MenusJson::embedded().menus {
                for item in &menu.items {
                    if let KindJson::Command { id, mac, other, .. } = &item.kind {
                        let keys = match os {
                            Os::Mac => mac,
                            Os::Other => other,
                        };
                        if let Some(keys) = keys {
                            let muda = accelerator(keys, os);
                            assert!(
                                muda.parse::<muda::accelerator::Accelerator>().is_ok(),
                                "{id}: {keys} became {muda}, which muda can't read"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn every_command_with_keys_has_them_on_its_menu_item() {
        // The map's keys for this platform are the menu item's accelerator, for every command.
        let menus = MenusJson::embedded();
        for os in [Os::Mac, Os::Other] {
            let bar = menu_bar(
                &menus,
                &Context {
                    os,
                    recent: &[],
                    windows: &[],
                    focused: None,
                    state: None,
                },
            );
            let mut built: BTreeMap<String, Option<String>> = BTreeMap::new();
            let mut stack: Vec<&Node> = bar.iter().flat_map(|m| m.items.iter()).collect();
            while let Some(node) = stack.pop() {
                match node {
                    Node::Item(item) => {
                        built.insert(item.id.clone(), item.accelerator.clone());
                    }
                    Node::Submenu { items, .. } => stack.extend(items.iter()),
                    _ => {}
                }
            }
            for menu in menus.menus.iter().filter(|m| for_os(m.on, os)) {
                for item in menu.items.iter().filter(|i| for_os(i.on, os)) {
                    if let KindJson::Command { id, mac, other, .. } = &item.kind {
                        let keys = match os {
                            Os::Mac => mac,
                            Os::Other => other,
                        };
                        assert_eq!(
                            built.get(&format!("cmd:{id}")),
                            Some(&keys.as_deref().map(|k| accelerator(k, os))),
                            "{id} on {os:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_strings_shared_with_the_webview_match_its_own() {
        // The event's name, and the commands Rust names, are the webview's.
        let hook = include_str!("../../src/menus/use-native-menus.tsx");
        assert!(
            hook.contains(&format!("\"{COMMAND_EVENT}\"")),
            "use-native-menus.tsx names another event"
        );
        let tabs = include_str!("../../src/tabs/use-tabs.tsx");
        assert!(
            tabs.contains("\"tab.move-to-window\""),
            "use-tabs.tsx handles no tab.move-to-window"
        );
        let ids: Vec<String> = MenusJson::embedded()
            .menus
            .iter()
            .flat_map(|menu| &menu.items)
            .filter_map(|item| match &item.kind {
                KindJson::Command { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();
        for id in WITHOUT_WINDOW.iter().chain(&["tab.move-to-new-window"]) {
            assert!(ids.iter().any(|known| known == id), "{id} isn't a menu command");
        }
        assert_eq!(
            action("recent-clear"),
            Action::Command(MenuCommand {
                id: "file.clear-recent".to_owned(),
                arg: None
            })
        );
    }

    #[test]
    fn keys_turn_into_muda_notation() {
        assert_eq!(accelerator("Mod+Shift+N", Os::Mac), "Cmd+Shift+N");
        assert_eq!(accelerator("Mod+Shift+N", Os::Other), "Ctrl+Shift+N");
        assert_eq!(accelerator("Mod++", Os::Mac), "Cmd+Equal");
        assert_eq!(accelerator("Mod+-", Os::Other), "Ctrl+Minus");
        assert_eq!(accelerator("Mod+\\", Os::Mac), "Cmd+Backslash");
        assert_eq!(accelerator("Alt+ArrowLeft", Os::Other), "Alt+ArrowLeft");
        assert_eq!(accelerator("F1", Os::Other), "F1");
        assert_eq!(accelerator("Mod+,", Os::Mac), "Cmd+Comma");
    }

    #[test]
    fn with_no_window_only_open_and_new_window_are_enabled() {
        let bar = menu_bar(
            &MenusJson::embedded(),
            &Context {
                os: Os::Mac,
                recent: &[],
                windows: &[],
                focused: None,
                state: None,
            },
        );
        let text = render(&bar);
        assert!(text.contains("Open a file… [Cmd+O]  <cmd:file.open>"), "{text}");
        assert!(text.contains("New window [Cmd+Shift+N]  <cmd:window.new>"), "{text}");
        assert!(text.contains("Print… [Cmd+P] (disabled)"), "{text}");
        assert!(text.contains("Close tab [Cmd+W] (disabled)"), "{text}");
        assert!(text.contains("No recent files (disabled)"), "{text}");
    }

    #[test]
    fn items_say_what_choosing_them_does() {
        let command = |id: &str, arg: Option<&str>| {
            Action::Command(MenuCommand {
                id: id.to_owned(),
                arg: arg.map(str::to_owned),
            })
        };
        assert_eq!(action("cmd:view.zoom-in"), command("view.zoom-in", None));
        assert_eq!(action("cmd:view.theme=dark"), command("view.theme", Some("dark")));
        assert_eq!(action("recent-clear"), command("file.clear-recent", None));
        assert_eq!(action("move:doc-2"), command("tab.move-to-window", Some("doc-2")));
        assert_eq!(action("recent:abc"), Action::OpenRecent("abc".to_owned()));
        assert_eq!(action("recent:none"), Action::None);
        assert_eq!(action("window:doc-1"), Action::Focus("doc-1".to_owned()));
        assert_eq!(
            action("link:https://docs.laminapdf.com/"),
            Action::Link("https://docs.laminapdf.com/".to_owned())
        );
        assert_eq!(action("later:View licences"), Action::None);
    }
}
