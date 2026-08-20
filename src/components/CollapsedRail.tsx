// src/components/CollapsedRail.tsx
import { ChevronLeft, ChevronRight } from "lucide-react";
import type { ReactNode } from "react";

interface CollapsedRailProps {
  icon: ReactNode;
  label: string;
  onExpand: () => void;
  side: "left" | "right";
}

export function CollapsedRail({ icon, label, onExpand, side }: CollapsedRailProps) {
  return (
    <button className={`collapsed-rail ${side}`} type="button" onClick={onExpand} aria-label={label} title={label}>
      {icon}
      {side === "left" ? <ChevronRight size={16} aria-hidden="true" /> : <ChevronLeft size={16} aria-hidden="true" />}
    </button>
  );
}
