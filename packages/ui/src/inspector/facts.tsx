// The object inspector's other tabs (O15): Revisions (one row per revision: its cross-reference
// section, objects added, changed and freed, signed or not), Fonts (name, type, embedded or subset,
// encoding, ToUnicode coverage), Colour (output intents and ICC profiles) and Conformance (the
// claims as badges, each "Claimed, not verified"). Each is a table with a caption and column
// headers; one table component draws them all.

import type { ReactNode } from "react";
import { Badge } from "../components/badge.js";
import { laterMessage } from "../later-messages.js";

/** One revision, as `@laminapdf/sdk`'s `Revision`. */
export interface InspectRevision {
  readonly index: number;
  readonly section: number;
  readonly bytes: number;
  readonly added: readonly number[];
  readonly changed: readonly number[];
  readonly freed: readonly number[];
  readonly signed: boolean;
}

/** One font, as `@laminapdf/sdk`'s `DocumentFont`. */
export interface InspectFont {
  readonly name?: string | undefined;
  readonly kind?: string | undefined;
  readonly embedded: boolean;
  readonly subset: boolean;
  readonly encoding?: { readonly kind: "named"; readonly name: string } | { readonly kind: "custom" } | undefined;
  readonly object?: number | undefined;
  readonly toUnicode: boolean;
  readonly toUnicodeCoverage?: { readonly mapped: number; readonly codes: number } | undefined;
}

/** The colour profiles, as `@laminapdf/sdk`'s `ColorProfiles`. */
export interface InspectColour {
  readonly outputIntents: readonly {
    readonly subtype?: string | undefined;
    readonly conditionIdentifier?: string | undefined;
    readonly condition?: string | undefined;
    readonly profile?: number | undefined;
    readonly page?: number | undefined;
  }[];
  readonly profiles: readonly {
    readonly object: number;
    readonly components?: number | undefined;
    readonly outputIntent: boolean;
    readonly version?: string | undefined;
    readonly deviceClass?: string | undefined;
    readonly colourSpace?: string | undefined;
    readonly description?: string | undefined;
  }[];
}

/** The claims, as `@laminapdf/sdk`'s `ConformanceClaims`. */
export interface InspectConformance {
  readonly headerVersion?: string | undefined;
  readonly catalogVersion?: string | undefined;
  readonly pdfa?: { readonly part: number; readonly conformance?: string | undefined } | undefined;
  readonly pdfua?: number | undefined;
  readonly pdfx?: string | undefined;
}

/** A table: its caption, its column headings, and its rows of cells. */
function FactsTable({
  caption,
  columns,
  rows,
  empty,
}: {
  caption: string;
  columns: readonly string[];
  rows: readonly (readonly ReactNode[])[];
  empty: string;
}): ReactNode {
  if (rows.length === 0) return <p className="paper-inspect-note">{empty}</p>;
  return (
    <table className="paper-inspect-table">
      <caption className="paper-inspect-caption">{caption}</caption>
      <thead>
        <tr>
          {columns.map((c) => (
            <th key={c} scope="col">
              {c}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {rows.map((cells, r) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: rows keep their order; nothing reorders them.
          <tr key={r}>
            {cells.map((cell, c) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: a cell's column is its identity.
              <td key={c}>{cell}</td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

const yes = (flag: boolean) => laterMessage(flag ? "inspect.yes" : "inspect.no");
const count = (list: readonly number[]) => list.length.toLocaleString("en");

/** The Revisions tab. */
export function RevisionsTable({ revisions }: { revisions: readonly InspectRevision[] }): ReactNode {
  return (
    <FactsTable
      caption={laterMessage("inspect.tab.revisions")}
      columns={[
        laterMessage("inspect.revision"),
        laterMessage("inspect.revision.section"),
        laterMessage("inspect.revision.bytes"),
        laterMessage("inspect.revision.added"),
        laterMessage("inspect.revision.changed"),
        laterMessage("inspect.revision.freed"),
        laterMessage("inspect.revision.signed"),
      ]}
      rows={revisions.map((r) => [
        String(r.index + 1),
        r.section.toLocaleString("en"),
        r.bytes.toLocaleString("en"),
        count(r.added),
        count(r.changed),
        count(r.freed),
        r.signed ? (
          <Badge key="signed" tone="accent">
            {laterMessage("inspect.revision.signedBadge")}
          </Badge>
        ) : (
          yes(false)
        ),
      ])}
      empty={laterMessage("inspect.revisions.none")}
    />
  );
}

/** A font's ToUnicode column: "Yes · 95 of 95 codes", "Yes", "No". */
function toUnicode(font: InspectFont): string {
  if (!font.toUnicode) return yes(false);
  const coverage = font.toUnicodeCoverage;
  if (coverage === undefined) return yes(true);
  return laterMessage("inspect.font.coverage", { mapped: coverage.mapped, codes: coverage.codes });
}

/** The Fonts tab. */
export function FontsTable({ fonts }: { fonts: readonly InspectFont[] }): ReactNode {
  return (
    <FactsTable
      caption={laterMessage("inspect.tab.fonts")}
      columns={[
        laterMessage("inspect.font.name"),
        laterMessage("inspect.type"),
        laterMessage("inspect.font.embedded"),
        laterMessage("inspect.font.encoding"),
        laterMessage("inspect.font.toUnicode"),
        laterMessage("inspect.font.object"),
      ]}
      rows={fonts.map((f) => [
        f.name ?? laterMessage("inspect.unnamed"),
        f.kind ?? "",
        f.embedded ? laterMessage(f.subset ? "inspect.font.subset" : "inspect.yes") : yes(false),
        f.encoding === undefined
          ? ""
          : f.encoding.kind === "named"
            ? f.encoding.name
            : laterMessage("inspect.font.own"),
        toUnicode(f),
        f.object === undefined ? "" : `${f.object} 0 R`,
      ])}
      empty={laterMessage("inspect.fonts.none")}
    />
  );
}

/** The Colour tab: the output intents, then the ICC profiles. */
export function ColourTables({ colour }: { colour: InspectColour }): ReactNode {
  return (
    <>
      <h3 className="paper-inspect-heading">{laterMessage("inspect.colour.intents")}</h3>
      <FactsTable
        caption={laterMessage("inspect.colour.intents")}
        columns={[
          laterMessage("inspect.colour.for"),
          laterMessage("inspect.colour.condition"),
          laterMessage("inspect.colour.profile"),
          laterMessage("inspect.colour.where"),
        ]}
        rows={colour.outputIntents.map((i) => [
          i.subtype ?? "",
          [i.conditionIdentifier, i.condition].filter((t) => t !== undefined && t !== "").join(" · "),
          i.profile === undefined ? "" : `${i.profile} 0 R`,
          i.page === undefined
            ? laterMessage("inspect.colour.document")
            : laterMessage("inspect.page", { page: i.page + 1 }),
        ])}
        empty={laterMessage("inspect.colour.noIntents")}
      />
      <h3 className="paper-inspect-heading">{laterMessage("inspect.colour.profiles")}</h3>
      <FactsTable
        caption={laterMessage("inspect.colour.profiles")}
        columns={[
          laterMessage("inspect.font.object"),
          laterMessage("inspect.colour.description"),
          laterMessage("inspect.colour.space"),
          laterMessage("inspect.colour.class"),
          laterMessage("inspect.colour.version"),
        ]}
        rows={colour.profiles.map((p) => [
          `${p.object} 0 R`,
          p.description ?? "",
          p.colourSpace ?? (p.components === undefined ? "" : String(p.components)),
          p.deviceClass ?? "",
          p.version ?? "",
        ])}
        empty={laterMessage("inspect.colour.noProfiles")}
      />
    </>
  );
}

/** The claims a file makes, each as a badge with "Claimed, not verified". */
function claims(c: InspectConformance): string[] {
  const out: string[] = [];
  if (c.pdfa !== undefined) out.push(`PDF/A-${c.pdfa.part}${(c.pdfa.conformance ?? "").toLowerCase()}`);
  if (c.pdfua !== undefined) out.push(`PDF/UA-${c.pdfua}`);
  if (c.pdfx !== undefined) out.push(c.pdfx);
  return out;
}

/** The Conformance tab. */
export function ConformanceFacts({ conformance }: { conformance: InspectConformance }): ReactNode {
  const list = claims(conformance);
  return (
    <>
      <dl className="paper-inspect-facts">
        <div>
          <dt>{laterMessage("inspect.conformance.header")}</dt>
          <dd>{conformance.headerVersion ?? laterMessage("inspect.none")}</dd>
        </div>
        <div>
          <dt>{laterMessage("inspect.conformance.catalog")}</dt>
          <dd>{conformance.catalogVersion ?? laterMessage("inspect.none")}</dd>
        </div>
      </dl>
      <h3 className="paper-inspect-heading">{laterMessage("inspect.conformance.claims")}</h3>
      {list.length === 0 ? (
        <p className="paper-inspect-note">{laterMessage("inspect.conformance.none")}</p>
      ) : (
        <ul className="paper-inspect-claims">
          {list.map((claim) => (
            <li key={claim}>
              <Badge tone="neutral">{claim}</Badge>
              <span className="paper-inspect-note">{laterMessage("inspect.conformance.claimed")}</span>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}

/** "PDF/A-2b · PDF/UA-1", or `undefined` without claims: for the properties dialog's row. */
export function claimsText(conformance: InspectConformance): string | undefined {
  const list = claims(conformance);
  return list.length === 0 ? undefined : list.join(" · ");
}
