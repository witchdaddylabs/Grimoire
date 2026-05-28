// src/components/PanelHeader.tsx
import type { ReactNode } from "react";

interface PanelHeaderProps {
  icon: ReactNode;
  title: string;
  subtitle: string;
  action?: ReactNode;
}

export function PanelHeader({ icon, title, subtitle, action }: PanelHeaderProps) {
  return (
    <div className="panel-header">
      <div className="panel-heading">
        <div className="panel-icon">{icon}</div>
        <div>
          <h3>{title}</h3>
          <p>{subtitle}</p>
        </div>
      </div>
      {action}
    </div>
  );
}
