import { Html } from '../html.ts';

// Communication (Glossary: Progress Indicator, Snackbar, Tooltip)

export const CircularProgress = ({ value, indeterminate = false, ...props }: any) => {
  return <md-circular-progress value={value} indeterminate={indeterminate} {...props}></md-circular-progress>;
};

export const LinearProgress = ({ value, indeterminate = false, ...props }: any) => {
  return <md-linear-progress value={value} indeterminate={indeterminate} {...props}></md-linear-progress>;
};

// Emulated Snackbar (custom element wrapper)
export const Snackbar = ({ message, actionLabel, ...props }: any) => {
  return (
    <div class="md3-snackbar" role="status" {...props}>
      <div class="md3-snackbar-supporting-text">{message}</div>
      {actionLabel && <md-text-button>{actionLabel}</md-text-button>}
    </div>
  );
};

export const Tooltip = ({ tooltip, ...props }: any) => {
  // @material/web doesn't have a stable tooltip yet, emulating structure
  return (
    <div class="md3-tooltip-wrapper" {...props}>
      <span class="md3-tooltip">{tooltip}</span>
    </div>
  );
};
