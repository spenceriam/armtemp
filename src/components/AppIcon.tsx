// The ARMtemp thermometer mark, ported from the Clod design's inline SVGs.

interface Props {
  size?: number;
  white?: boolean;
}

export function AppIcon({ size = 24, white = true }: Props) {
  const fill = white ? "#fff" : "currentColor";
  return (
    <svg viewBox="0 0 24 24" width={size} height={size} fill="none">
      <rect x="9" y="3" width="6" height="13" rx="3" fill={fill} opacity={0.35} />
      <rect x="10.5" y="7" width="3" height="9" rx="1.5" fill={fill} />
      <circle cx="12" cy="17.5" r="4.2" fill={fill} />
    </svg>
  );
}
