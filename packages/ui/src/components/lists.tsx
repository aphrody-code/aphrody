import { Html } from '../html.ts';

// Lists (Glossary: List, Image List)

export const List = ({ children, ...props }: any) => {
  return <md-list {...props}>{children}</md-list>;
};

export const ListItem = ({ headline, supportingText, start, end, ...props }: any) => {
  return (
    <md-list-item {...props}>
      {headline && <div slot="headline">{headline}</div>}
      {supportingText && <div slot="supporting-text">{supportingText}</div>}
      {start && <div slot="start">{start}</div>}
      {end && <div slot="end">{end}</div>}
    </md-list-item>
  );
};

export const ImageList = ({ children, ...props }: any) => {
  return <div class="md3-image-list" {...props}>{children}</div>;
};
