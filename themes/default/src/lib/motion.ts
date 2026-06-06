export const themeEasing = {
  standard: [0.22, 1, 0.36, 1],
  emphasized: [0.16, 1, 0.3, 1],
  exit: [0.4, 0, 1, 1],
} as const;

export const themeDuration = {
  micro: 0.16,
  enter: 0.34,
  page: 0.42,
  overlay: 0.22,
  collapse: 0.22,
} as const;

export const themeSpring = {
  type: "spring" as const,
  stiffness: 420,
  damping: 34,
  mass: 0.9,
} as const;

export const themeLayoutSpring = {
  type: "spring" as const,
  stiffness: 500,
  damping: 38,
  mass: 0.85,
} as const;

export const themePageHeaderMotion = {
  initial: { opacity: 0, y: -10 },
  animate: { opacity: 1, y: 0 },
  transition: { duration: themeDuration.page, ease: themeEasing.standard },
} as const;

export const themePageContentMotion = {
  initial: { opacity: 0, y: 14 },
  animate: { opacity: 1, y: 0 },
  transition: { duration: themeDuration.page, ease: themeEasing.standard },
} as const;

export const themeOverlayMotion = {
  initial: { opacity: 0 },
  animate: { opacity: 1 },
  exit: { opacity: 0 },
  transition: { duration: themeDuration.overlay, ease: themeEasing.standard },
} as const;

export const themePreviewImageMotion = {
  initial: { opacity: 0, scale: 0.985, y: 8 },
  animate: { opacity: 1, scale: 1, y: 0 },
  exit: { opacity: 0, scale: 0.985, y: 6 },
  transition: { duration: themeDuration.overlay, ease: themeEasing.standard },
} as const;

export const themeCollapseMotion = {
  initial: { opacity: 0, height: 0, y: -4 },
  animate: { opacity: 1, height: "auto", y: 0 },
  exit: { opacity: 0, height: 0, y: -4 },
  transition: { duration: themeDuration.collapse, ease: themeEasing.standard },
} as const;

export const themeTocMotion = {
  initial: { opacity: 0, x: 8 },
  animate: { opacity: 1, x: 0 },
  transition: { duration: themeDuration.enter, ease: themeEasing.standard, delay: 0.08 },
} as const;

export const themeHoverLift = { y: -2 } as const;

export function getThemeListItemMotion(index = 0, delayStep = 0.04) {
  return {
    initial: { opacity: 0, y: 18 },
    animate: { opacity: 1, y: 0 },
    transition: {
      ...themeSpring,
      delay: index * delayStep,
    },
  } as const;
}
