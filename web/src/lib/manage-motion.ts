export const manageMotion = {
  page: {
    initial: { opacity: 0, y: 8 },
    animate: { opacity: 1, y: 0 },
    transition: { type: "spring", stiffness: 380, damping: 32, mass: 0.7 },
  },
  sidebarItem: {
    hover: { x: 4 },
    activeHover: { x: 0 },
    transition: { type: "spring", stiffness: 420, damping: 30, mass: 0.65 },
  },
} as const;

export const manageMotionClassNames = {
  overlay:
    "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:duration-150 data-[state=open]:duration-150",
  dialog:
    "duration-150 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-98 data-[state=open]:zoom-in-98 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[49%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[49%]",
  dropdown:
    "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-98 data-[state=open]:zoom-in-98 data-[state=closed]:duration-100 data-[state=open]:duration-100 data-[side=bottom]:slide-in-from-top-1 data-[side=left]:slide-in-from-right-1 data-[side=right]:slide-in-from-left-1 data-[side=top]:slide-in-from-bottom-1",
  tabsTrigger:
    "transition-[background-color,color,box-shadow,transform] duration-150 data-[state=active]:translate-y-0",
  tabsContent:
    "data-[state=active]:animate-in data-[state=active]:fade-in-0 data-[state=active]:slide-in-from-bottom-1 data-[state=active]:duration-150",
  sidebar:
    "transition-transform duration-200 ease-out",
  sidebarChildren:
    "animate-in fade-in-0 slide-in-from-top-1 duration-150",
};
