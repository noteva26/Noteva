export const themePageHeaderMotion = {
  initial: { opacity: 0, y: -10 },
  animate: { opacity: 1, y: 0 },
  transition: { duration: 0.45, ease: "easeOut" },
} as const;

export const themeSpring = {
  type: "spring" as const,
  stiffness: 400,
  damping: 30,
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
