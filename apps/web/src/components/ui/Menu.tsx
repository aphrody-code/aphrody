// Reusable anchored M3 menu. Renders md-menu in the top layer (positioning
// "popover") so it never clips, mis-positions, or hides behind the app bar
// regardless of ancestor stacking/transform/overflow contexts, and keeps React
// state in sync with native dismiss. The trigger lives inside the anchor element,
// which makes md-menu's outside-click dismiss ignore trigger clicks so the toggle
// opens AND closes cleanly (no light-dismiss vs toggle race).

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
  }, []);

  return (
    <span ref={anchorRef} style={{ display: "inline-flex" }}>
      {trigger({ open, toggle: () => setOpen((o) => !o) })}
      <MdMenu
        ref={menuRef}
        open={open}
        positioning="popover"
        onOpened={() => setOpen(true)}
        onClosed={() => setOpen(false)}
      >
        {children}
      </MdMenu>
    </span>
  );
}
