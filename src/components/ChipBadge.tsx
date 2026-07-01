// Snapdragon tier badge — renders ONLY the official Qualcomm badge artwork
// bundled in src/assets/chips/ (x.png, x-plus.png, x-elite.png, x2.png,
// x2-plus.png, x2-elite.png). No recreated/stylized substitutes: if a tier's
// artwork isn't present, nothing is rendered.
const OFFICIAL: Record<string, string> = import.meta.glob(
  "../assets/chips/*.{png,webp,svg}",
  { eager: true, query: "?url", import: "default" }
) as Record<string, string>;

export type ChipTier = "x" | "x-plus" | "x-elite" | "x2-plus" | "x2-elite" | "generic";

export function tierFromName(name: string | null | undefined): ChipTier {
  const n = (name ?? "").toLowerCase();
  if (!n.includes("snapdragon")) return "generic";
  if (n.includes("x2 elite")) return "x2-elite";
  if (n.includes("x2 plus")) return "x2-plus";
  // There is no base "X2" SKU — an unrecognized X2 string gets no badge.
  if (n.includes("x2")) return "generic";
  if (n.includes("elite")) return "x-elite";
  if (n.includes("plus")) return "x-plus";
  return "x";
}

const ALT: Record<ChipTier, string> = {
  x: "Snapdragon X",
  "x-plus": "Snapdragon X Plus",
  "x-elite": "Snapdragon X Elite",
  "x2-plus": "Snapdragon X2 Plus",
  "x2-elite": "Snapdragon X2 Elite",
  generic: "",
};

export function ChipBadge({ tier, size }: { tier: ChipTier; size: number }) {
  const official = Object.entries(OFFICIAL).find(([path]) =>
    path.includes(`/${tier}.`)
  )?.[1];
  if (!official) return null;
  return (
    <img
      src={official}
      width={size}
      height={size}
      alt={ALT[tier]}
      title={ALT[tier]}
      style={{ borderRadius: size * 0.16, flex: "none", objectFit: "cover" }}
    />
  );
}
