export function PixelIcon({
  rows,
  className,
  palette,
}: {
  rows: string[];
  className?: string;
  palette?: Record<string, string>;
}) {
  const defaultPalette: Record<string, string> = {
    "#": "currentColor",
    "+": "currentColor",
  };

  const colors = palette ? { ...defaultPalette, ...palette } : defaultPalette;

  return (
    <svg
      className={className}
      viewBox="0 0 16 16"
      fill="none"
      shapeRendering="crispEdges"
      aria-hidden="true"
      focusable="false"
    >
      {rows.flatMap((row, y) =>
        [...row].map((c, x) =>
          c === "." ? null : (
            <rect
              key={`${x}-${y}`}
              x={x}
              y={y}
              width="1"
              height="1"
              fill={colors[c] ?? "currentColor"}
            />
          )
        )
      )}
    </svg>
  );
}
