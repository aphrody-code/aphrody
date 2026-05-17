import { Html } from '../html.ts';

// Navigation (Glossary: Navigation Bar, Navigation Drawer, Navigation Rail, Tabs)

export const NavigationBar = ({ children, ...props }: any) => {
  return <nav class="md3-navigation-bar" {...props}>{children}</nav>;
};

export const NavigationDrawer = ({ children, open, ...props }: any) => {
  return (
    <aside class={`md3-navigation-drawer ${open ? 'open' : ''}`} {...props}>
      {children}
    </aside>
  );
};

export const NavigationRail = ({ children, ...props }: any) => {
  return <nav class="md3-navigation-rail" {...props}>{children}</nav>;
};

export const Tabs = ({ variant = 'primary', children, ...props }: any) => {
  const Tag = `md-${variant}-tabs`;
  return <Tag {...props}>{children}</Tag>;
};

export const Tab = ({ variant = 'primary', label, icon, ...props }: any) => {
  const Tag = `md-${variant}-tab`;
  return (
    <Tag {...props}>
      {label && <span>{label}</span>}
      {icon && <md-icon slot="icon">{icon}</md-icon>}
    </Tag>
  );
};
