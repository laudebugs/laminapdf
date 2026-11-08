/**
 * What Diagram.astro hands every drawing: the prefix its arrowheads' ids start with, and the ids
 * of the caption and the text description its two layouts point their aria attributes at.
 */
export interface DrawingProps {
  prefix: string;
  labelledby: string;
  describedby: string;
}
