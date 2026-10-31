// `@lamina/pdf/worker` (L2a): the async twin of the sync API, in a Web Worker.
export type { ConnectOptions, RemoteHttpOptions } from "./client.js";
export { connect, WorkerClient } from "./client.js";
export { PROTOCOL_VERSION } from "./protocol.js";
export type { ExcludedDocumentMembers, ExcludedPageMembers } from "./support.js";
export type { CallOptions, RemoteDocument, RemoteMember, RemotePage, RemoteResult } from "./types.js";
