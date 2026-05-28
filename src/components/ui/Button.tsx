// src/components/ui/Button.tsx
import type { ReactNode } from "react";

interface ButtonProps {
  children: ReactNode;
  className?: string;
  type?: "button" | "submit";
  disabled?: boolean;
  onClick?: () => void;
}

export function Button({ children, className = "", type = "button", disabled, onClick }: ButtonProps) {
  const base = "button";
  return (
    <button className={`${base} ${className}`.trim()} type={type} disabled={disabled} onClick={onClick}>
      {children}
    </button>
  );
}

export function PrimaryButton({ children, className = "", ...props }: ButtonProps) {
  return <Button {...props} className={`button-primary ${className}`.trim()}>{children}</Button>;
}

export function SecondaryButton({ children, className = "", ...props }: ButtonProps) {
  return <Button {...props} className={`button-secondary ${className}`.trim()}>{children}</Button>;
}

export function DangerButton({ children, className = "", ...props }: ButtonProps) {
  return <Button {...props} className={`button-danger ${className}`.trim()}>{children}</Button>;
}
