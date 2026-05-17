import { Html } from '../html.ts';

// Actions (Glossary: Button, FAB, Extended FAB, Icon Button)

export const Button = ({ variant = 'filled', children, ...props }: any) => {
  const Tag = `md-${variant}-button`;
  return <Tag {...props}>{children}</Tag>;
};

export const Fab = ({ variant = 'primary', size = 'medium', lowered = false, icon, label, ...props }: any) => {
  return (
    <md-fab variant={variant} size={size} lowered={lowered} label={label} {...props}>
      {icon && <md-icon slot="icon">{icon}</md-icon>}
    </md-fab>
  );
};

export const IconButton = ({ variant = 'standard', icon, ...props }: any) => {
  if (variant === 'standard') {
    return <md-icon-button {...props}><md-icon>{icon}</md-icon></md-icon-button>;
  }
  const Tag = `md-${variant}-icon-button`;
  return <Tag {...props}><md-icon>{icon}</md-icon></Tag>;
};

export const Chip = ({ type = 'assist', label, icon, ...props }: any) => {
  const Tag = `md-${type}-chip`;
  return (
    <Tag label={label} {...props}>
      {icon && <md-icon slot="icon">{icon}</md-icon>}
    </Tag>
  );
};
