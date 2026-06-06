import { Children, type ReactNode } from "react";
import { motion, type Variants } from "motion/react";
import { themeSpring } from "@/lib/motion";

interface AnimatedListProps {
  children: ReactNode;
  className?: string;
  staggerDelay?: number;
  animate?: boolean;
}

export function AnimatedList({
  children,
  className,
  staggerDelay = 0.03,
  animate = true,
}: AnimatedListProps) {
  if (!animate) {
    return <div className={className}>{children}</div>;
  }

  const container: Variants = {
    hidden: { opacity: 0 },
    show: {
      opacity: 1,
      transition: {
        staggerChildren: staggerDelay,
      },
    },
  };

  const item: Variants = {
    hidden: { opacity: 0, y: 12 },
    show: {
      opacity: 1,
      y: 0,
      transition: themeSpring,
    },
  };

  return (
    <motion.div
      variants={container}
      initial="hidden"
      animate="show"
      className={className}
    >
      {Children.map(children, (child) => (
        <motion.div variants={item}>{child}</motion.div>
      ))}
    </motion.div>
  );
}
