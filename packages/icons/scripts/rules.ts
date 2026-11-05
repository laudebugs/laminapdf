// The design language's icon rules (§5) as checks: the root `<svg>` exactly as the wrapper gives
// it, only geometry on the shapes, nothing outside the 24-unit grid's margin. The lint script, the
// generator and the tests all call `lintIcon`, so an icon that breaks a rule never reaches the
// generated module.

import { boundingBox, parseSvg } from "./svg.ts";

/** The root `<svg>` of every icon file, attribute for attribute. */
const ROOT = {
  xmlns: "http://www.w3.org/2000/svg",
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  "stroke-width": "1.5",
};

/** The attributes each shape may carry: its geometry and nothing else. */
const GEOMETRY: Record<string, string[]> = {
  path: ["d"],
  circle: ["cx", "cy", "r"],
  ellipse: ["cx", "cy", "rx", "ry"],
  rect: ["x", "y", "width", "height", "rx", "ry"],
  line: ["x1", "y1", "x2", "y2"],
  polyline: ["points"],
  polygon: ["points"],
};

/** Icons drawn with dots: stroked circles under 1.5 units, which read as filled. */
const DOTS = new Set(["more", "drag-grip", "tag"]);
/** Icons with a dashed outline (Mark area's selection rectangle). */
const DASHED = new Set(["mark-area"]);

/** The least margin to the grid's edge that passes, and the design language's target of 3.5–4.5. */
const MARGIN = { min: 2, target: [3.5, 4.5] as const };

/** What is wrong with an icon (errors fail) and what is unusual about it (warnings are reported). */
export interface Finding {
  errors: string[];
  warnings: string[];
}

/** Checks one icon file against the rules. */
export function lintIcon(name: string, text: string): Finding {
  const errors: string[] = [];
  const warnings: string[] = [];
  let svg: ReturnType<typeof parseSvg>;
  try {
    svg = parseSvg(text);
  } catch (e) {
    return { errors: [`${name}: ${(e as Error).message}`], warnings };
  }
  for (const [key, want] of Object.entries(ROOT)) {
    if (svg.root[key] !== want)
      errors.push(`${name}: <svg> needs ${key}="${want}", has ${JSON.stringify(svg.root[key])}`);
  }
  for (const key of Object.keys(svg.root)) {
    if (!(key in ROOT)) errors.push(`${name}: <svg> may not carry ${key}`);
  }
  if (svg.shapes.length === 0) errors.push(`${name}: no shapes`);
  for (const shape of svg.shapes) errors.push(...shapeErrors(name, shape));
  if (errors.length > 0) return { errors, warnings };
  const box = boundingBox(svg.shapes);
  const margin = Math.min(box.minX, box.minY, 24 - box.maxX, 24 - box.maxY);
  const round = Math.round(margin * 100) / 100;
  // A margin over the target is a small glyph (a chevron, a minus) and drawn so on purpose; only a
  // tight one is worth a look.
  if (margin < MARGIN.min) errors.push(`${name}: margin ${round} is under ${MARGIN.min} units`);
  else if (margin < MARGIN.target[0]) warnings.push(`${name}: margin ${round}, under the ${MARGIN.target[0]} target`);
  return { errors, warnings };
}

function shapeErrors(name: string, shape: { name: string; attrs: Record<string, string> }): string[] {
  const allowed = GEOMETRY[shape.name];
  if (allowed === undefined) return [`${name}: <${shape.name}> is not a shape the icons use`];
  const out: string[] = [];
  for (const key of Object.keys(shape.attrs)) {
    const dashed = key === "stroke-dasharray" && DASHED.has(name);
    if (!allowed.includes(key) && !dashed) out.push(`${name}: <${shape.name}> may not carry ${key}`);
  }
  const r = Number.parseFloat(shape.attrs.r ?? "Infinity");
  if (shape.name === "circle" && r < 1.5 && !DOTS.has(name)) {
    out.push(`${name}: a circle of radius ${r} draws a dot; only the icons drawn with dots may`);
  }
  return out;
}
