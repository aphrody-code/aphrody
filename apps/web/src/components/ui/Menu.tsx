// Reusable anchored M3 menu. Wires md-menu's anchorElement + light-dismiss
// (the native `closed` event) back into React state via a ref, so it stays in
// sync no matter how the menu is dismissed.

import { useEffect, useRef, useState, type ComponentRef } from "react";
import { MdMenu } from "@aphrody/m3-react";

export function Menu({
  trigger,
  children,
}: {
  trigger: (props: { open: boolean; toggle: () => void }) => React.ReactNode;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const anchorRef = useRef<HTMLSpanElement>(null);
  const menuRef = useRef<ComponentRef<typeof MdMenu>>(null);

  useEffect(() => {
    const menu = menuRef.current;
    if (menu && anchorRef.current) menu.anchorElement = anchorRef.current;
    if (!menu) return;
    const onClosed = () => setOpen(false);
    menu.addEventListener("closed", onClosed);
    return () => menu.removeEventListener("closed", onClosed);
  }, []);

  return (
    <span ref={anchorRef} style={{ position: "relative", display: "inline-flex" }}>
      {trigger({ open, toggle: () => setOpen((o) => !o) })}
      <MdMenu ref={menuRef} open={open} onClosed={() => setOpen(false)} positioning="fixed">
        {children}
      </MdMenu>
    </span>
  );
}
