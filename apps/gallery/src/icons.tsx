// The icons route: every icon at the sizes the chrome uses, in `text-3`, and once in `accent`.

import { ICONS, type IconName } from "@laminapdf/icons";
import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";

const SIZES = [14, 16, 18, 20, 22] as const;

export function Icons(): ReactNode {
  const names = Object.keys(ICONS) as IconName[];
  return (
    <table className="g-icons">
      <caption>Every icon in text-3 at each size, then in accent at 18 px</caption>
      <thead>
        <tr>
          <th scope="col">Name</th>
          {SIZES.map((size) => (
            <th scope="col" key={size}>
              {size} px
            </th>
          ))}
          <th scope="col">Accent</th>
        </tr>
      </thead>
      <tbody>
        {names.map((name) => (
          <tr key={name}>
            <th scope="row">
              <code>{name}</code>
            </th>
            {SIZES.map((size) => (
              <td key={size}>
                <Icon name={name} size={size} />
              </td>
            ))}
            <td className="g-accent">
              <Icon name={name} size={18} />
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
