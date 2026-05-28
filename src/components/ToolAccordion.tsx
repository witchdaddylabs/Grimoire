// src/components/ToolAccordion.tsx
import { ChevronDown, ChevronRight } from "lucide-react";
import type { ReactNode } from "react";
import type { ToolSectionId } from "./types";

interface ToolAccordionProps {
  id: ToolSectionId;
  icon: ReactNode;
  open: boolean;
  title: string;
  onToggle: (id: ToolSectionId) => void;
  children: ReactNode;
}

function SectionTitle({ icon, id, title }: { icon: ReactNode; id: string; title: string }) {
  return (
    <span className="tool-section-title" id={id}>
      {icon}
      <strong>{title}</strong>
    </span>
  );
}

export function ToolAccordion({ id, icon, open, title, onToggle, children }: ToolAccordionProps) {
  const titleId = `${id}-title`;
  return (
    <section className={open ? "tool-section open" : "tool-section"} aria-labelledby={titleId}>
      <button
        className="tool-section-toggle"
        type="button"
        aria-expanded={open}
        aria-controls={`${id}-body`}
        onClick={() => onToggle(id)}
      >
        <SectionTitle icon={icon} id={titleId} title={title} />
        {open ? <ChevronDown size={15} aria-hidden="true" /> : <ChevronRight size={15} aria-hidden="true" />}
      </button>
      {open ? (
        <div className="tool-section-body" id={`${id}-body`}>
          {children}
        </div>
      ) : null}
    </section>
  );
}
