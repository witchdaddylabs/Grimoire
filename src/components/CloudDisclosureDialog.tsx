// src/components/CloudDisclosureDialog.tsx
import { ShieldCheck, X } from "lucide-react";
import type { AiProviderKind } from "../app/ai";
import { providerLabels } from "../app/ai";

interface CloudDisclosureDialogProps {
  provider: AiProviderKind;
  copy: string;
  onAccept: () => void;
  onCancel: () => void;
}

export function CloudDisclosureDialog({ provider, copy, onAccept, onCancel }: CloudDisclosureDialogProps) {
  return (
    <div className="onboarding-backdrop" role="dialog" aria-modal="true" aria-labelledby="cloud-disclosure-title">
      <section className="onboarding-panel cloud-disclosure-panel">
        <button className="icon-button dismiss" type="button" aria-label="Cancel cloud provider" onClick={onCancel}>
          <X size={16} />
        </button>
        <p className="eyebrow">Cloud model disclosure</p>
        <h2 id="cloud-disclosure-title">{providerLabels[provider]}</h2>
        <p className="cloud-disclosure-copy">{copy}</p>
        <div className="inline-actions">
          <button className="button button-secondary" type="button" onClick={onCancel}>Keep Local</button>
          <button className="button button-primary" type="button" onClick={onAccept}>
            <ShieldCheck size={16} aria-hidden="true" />Accept
          </button>
        </div>
      </section>
    </div>
  );
}
