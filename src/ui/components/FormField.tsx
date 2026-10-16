import type { ReactNode } from "react";

export function FormField({ label, htmlFor, required = false, error, className = "", children }: {
  label: string;
  htmlFor: string;
  required?: boolean;
  error?: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={className}>
      <label htmlFor={htmlFor} className="mb-1 block text-sm font-medium">
        {label}
        {required && <span className="text-red-700"> *</span>}
      </label>
      {children}
      {error && (
        <p role="alert" className="mt-1 text-xs text-red-700">
          {error}
        </p>
      )}
    </div>
  );
}
