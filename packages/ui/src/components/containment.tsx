import { Html } from '../html.ts';

// Containment (Glossary: Card, Dialog, Divider, Elevation, Bottom Sheet, Side Sheet)

export const Card = ({ variant = 'elevated', children, ...props }: any) => {
  const Tag = `md-${variant}-card`;
  return <Tag {...props}>{children}</Tag>;
};

export const Dialog = ({ headline, content, actions, ...props }: any) => {
  return (
    <md-dialog {...props}>
      {headline && <div slot="headline">{headline}</div>}
      <div slot="content">{content}</div>
      <div slot="actions">{actions}</div>
    </md-dialog>
  );
};

export const Divider = (props: any) => {
  return <md-divider {...props}></md-divider>;
};

export const Elevation = ({ level = 1, ...props }: any) => {
  return <md-elevation level={level} {...props}></md-elevation>;
};

// Emulated Sheets (using containers and elevation)
export const SideSheet = ({ children, open, ...props }: any) => {
  return (
    <aside class={`md3-side-sheet ${open ? 'open' : ''}`} {...props}>
      <Elevation level={1} />
      {children}
    </aside>
  );
};
