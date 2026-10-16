const base =
  "inline-flex items-center justify-center gap-1.5 rounded-md px-3 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50";

export const btn = {
  primary: `${base} bg-t4s-blue text-white hover:bg-t4s-blue-dark`,
  secondary: `${base} border border-t4s-line bg-white text-t4s-charcoal hover:bg-t4s-cream`,
  danger: `${base} bg-red-700 text-white hover:bg-red-800`,
  ghost: `${base} text-t4s-gray hover:bg-t4s-cream hover:text-t4s-charcoal`,
};

export const iconButton = "rounded-md p-1.5 text-t4s-gray hover:bg-t4s-cream hover:text-t4s-charcoal disabled:opacity-40";

export const inputCls =
  "w-full rounded-md border border-t4s-line bg-white px-3 py-2 text-sm focus:border-t4s-blue focus:outline-none focus:ring-2 focus:ring-t4s-blue/20";
