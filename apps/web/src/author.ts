// The name comments carry (Paper C04: "asked once, the first time you comment; stored on this
// device"), kept in localStorage; where storage is blocked, a name given lasts the visit. Settings
// shows and changes it, so it lives apart from Comment mode, which loads later.

const AUTHOR_KEY = "lamina.comment.author";

/** The name given this visit: what marks carry when the browser refuses storage. */
let sessionAuthor: string | undefined;

/** The name comments carry: undefined until asked, empty when the person left it off. */
export function storedAuthor(): string | undefined {
  try {
    return localStorage.getItem(AUTHOR_KEY) ?? sessionAuthor;
  } catch {
    return sessionAuthor;
  }
}

/** Keeps the name comments carry: on this device, and for this visit where storage is blocked. */
export function storeAuthor(name: string): void {
  sessionAuthor = name.trim();
  try {
    localStorage.setItem(AUTHOR_KEY, sessionAuthor);
  } catch {
    // Storage is blocked: the name lasts this visit.
  }
}
