import { Html } from '../html.ts';

// Menus (Glossary: Menu)

export const Menu = ({ open, anchor, children, ...props }: any) => {
  return (
    <md-menu open={open} anchor={anchor} {...props}>
      {children}
    </md-menu>
  );
};

export const MenuItem = ({ headline, ...props }: any) => {
  return (
    <md-menu-item {...props}>
      <div slot="headline">{headline}</div>
    </md-menu-item>
  );
};
