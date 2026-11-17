// The object inspector's states (O15), for the axe and size tests and the gallery: a page and its
// content stream, each of the other tabs, a stream past the decode limit, and the view reading.

import type { ReactElement } from "react";
import {
  type InspectObjectData,
  type InspectOverview,
  type InspectTab,
  ObjectInspector,
  type StreamState,
} from "../src/dialogs.ts";

/** One state of a component (states.tsx's `Example`). */
interface InspectorExample {
  component: string;
  state: string;
  element: () => ReactElement;
}

const noop = () => {};
const ref = (num: number) => ({ type: "reference", num, generation: 0 }) as const;
const name = (value: string) => ({ type: "name", value }) as const;
const dict = (entries: [string, InspectObjectData["value"]][]) => ({ type: "dictionary", entries }) as const;
const at = (offset: number) => ({ kind: "offset", offset }) as const;

const CONTENT = new TextEncoder().encode(
  [
    "q 1 0 0 1 72 744 cm",
    "BT",
    "/F2 7.5 Tf 0.43 g",
    "(NORTHWIND HOUSING TRUST) Tj",
    "ET",
    "BT",
    "/F1 17 Tf 0.11 g",
    "0 -56 Td",
    "(2. Repairs backlog) Tj",
    "ET",
    "Q",
  ].join("\n"),
);

const OVERVIEW: InspectOverview = {
  trailer: dict([
    ["Size", { type: "integer", value: 9 }],
    ["Root", ref(1)],
    ["Info", ref(8)],
  ]),
  version: "1.7",
  conformance: { headerVersion: "1.7", pdfa: { part: 2, conformance: "B" }, pdfua: 1 },
  objects: [
    { num: 1, generation: 0, kind: "dictionary", typeName: "Catalog", location: at(15) },
    { num: 2, generation: 0, kind: "dictionary", typeName: "Pages", location: at(78) },
    { num: 3, generation: 0, kind: "dictionary", typeName: "Page", location: at(140) },
    { num: 4, generation: 0, kind: "dictionary", typeName: "Page", location: at(301) },
    { num: 5, generation: 0, kind: "stream", location: { kind: "objectStream", stream: 7, index: 0 } },
    { num: 6, generation: 0, kind: "dictionary", typeName: "Font", subtype: "Type1", location: at(520) },
    { num: 8, generation: 0, kind: "dictionary", location: at(640) },
  ],
  revisions: [
    { index: 0, section: 702, bytes: 1180, added: [1, 2, 3, 4, 5, 6, 8], changed: [], freed: [], signed: false },
    { index: 1, section: 1402, bytes: 1830, added: [], changed: [3], freed: [], signed: true },
  ],
  fonts: [
    {
      name: "NotoSans-Regular",
      kind: "Type0",
      embedded: true,
      subset: true,
      encoding: { kind: "named", name: "Identity-H" },
      object: 6,
      toUnicode: true,
      toUnicodeCoverage: { mapped: 95, codes: 95 },
    },
    { name: "Helvetica", kind: "Type1", embedded: false, subset: false, toUnicode: false },
  ],
  colour: {
    outputIntents: [{ subtype: "GTS_PDFA1", conditionIdentifier: "sRGB IEC61966-2.1", profile: 9 }],
    profiles: [
      {
        object: 9,
        components: 3,
        outputIntent: true,
        version: "2.1.0",
        deviceClass: "mntr",
        colourSpace: "RGB",
        description: "sRGB IEC61966-2.1",
      },
    ],
  },
};

const OBJECTS: ReadonlyMap<number, InspectObjectData> = new Map<number, InspectObjectData>([
  [
    3,
    {
      entry: OVERVIEW.objects[2] as InspectObjectData["entry"],
      value: dict([
        ["Type", name("Page")],
        ["Parent", ref(2)],
        [
          "MediaBox",
          {
            type: "array",
            items: [0, 0, 612, 792].map((value) => ({ type: "integer", value }) as const),
          },
        ],
        ["Resources", dict([["Font", dict([["F1", ref(6)]])]])],
        ["Contents", ref(5)],
        ["Annots", { type: "array", items: [ref(51)] }],
      ]),
      revision: 1,
      referrers: [{ from: 2, path: "/Kids[0]" }],
    },
  ],
  [
    5,
    {
      entry: OVERVIEW.objects[4] as InspectObjectData["entry"],
      value: dict([["Filter", name("FlateDecode")]]),
      stream: { filters: ["FlateDecode"], length: 3412 },
      revision: 0,
      referrers: [{ from: 3, path: "/Contents" }],
    },
  ],
]);

const READ: StreamState = { status: "read", bytes: CONTENT, total: CONTENT.length };

/** The inspector with `tab`, gone to `goTo`, and stream 5 in `streams`. */
function inspector(
  tab: InspectTab,
  options: { goTo?: number; streams?: ReadonlyMap<string, StreamState>; overview?: InspectOverview | undefined } = {},
): ReactElement {
  return (
    <div style={{ blockSize: 640, display: "flex", flexDirection: "column" }}>
      <ObjectInspector
        overview={"overview" in options ? options.overview : OVERVIEW}
        objects={OBJECTS}
        onRead={noop}
        streams={options.streams ?? new Map([["5:decoded", READ]])}
        onReadStream={noop}
        pageOf={(num) => (num === 3 ? 2 : num === 4 ? 3 : undefined)}
        initialTab={tab}
        goTo={options.goTo}
        onBack={noop}
        onShowPage={noop}
        onCopy={noop}
        onSaveRaw={noop}
      />
    </div>
  );
}

export const inspectorExamples: InspectorExample[] = [
  {
    component: "ObjectInspector",
    state: "a page with its content stream",
    element: () => inspector("objects", { goTo: 3 }),
  },
  {
    component: "ObjectInspector",
    state: "a stream that decodes past the limit",
    element: () =>
      inspector("objects", {
        goTo: 5,
        streams: new Map<string, StreamState>([
          ["5:decoded", { status: "over-limit", limit: "64.0 MB" }],
          ["5:raw", READ],
        ]),
      }),
  },
  { component: "ObjectInspector", state: "revisions, the second signed", element: () => inspector("revisions") },
  { component: "ObjectInspector", state: "fonts", element: () => inspector("fonts") },
  { component: "ObjectInspector", state: "colour profiles", element: () => inspector("colour") },
  { component: "ObjectInspector", state: "conformance claims", element: () => inspector("conformance") },
  {
    component: "ObjectInspector",
    state: "reading the document",
    element: () => inspector("objects", { overview: undefined }),
  },
];
