import { motion } from "motion/react";
import type { ReactNode } from "react";
import {
  themeDuration,
  themeEasing,
  themePageContentMotion,
} from "@/lib/motion";

interface PageTransitionProps {
  children: ReactNode;
  className?: string;
}

export function PageTransition({ children, className }: PageTransitionProps) {
  return (
    <motion.div {...themePageContentMotion} className={className}>
      {children}
    </motion.div>
  );
}

export function FadeIn({
  children,
  className,
  delay = 0,
}: PageTransitionProps & { delay?: number }) {
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{
        duration: themeDuration.enter,
        delay,
        ease: themeEasing.standard,
      }}
      className={className}
    >
      {children}
    </motion.div>
  );
}
