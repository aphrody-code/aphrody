import { Html } from '../html.ts';

// Inputs & Selection (Glossary: Checkbox, Radio, Switch, TextField, Slider)

export const Checkbox = (props: any) => {
  return <md-checkbox {...props}></md-checkbox>;
};

export const Radio = (props: any) => {
  return <md-radio {...props}></md-radio>;
};

export const Switch = (props: any) => {
  return <md-switch {...props}></md-switch>;
};

export const TextField = ({ variant = 'filled', label, ...props }: any) => {
  const Tag = `md-${variant}-text-field`;
  return <Tag label={label} {...props}></Tag>;
};

export const Slider = (props: any) => {
  return <md-slider {...props}></md-slider>;
};
