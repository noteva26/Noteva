"use client"

import { motion, Variants } from "motion/react"
import { Children, ReactNode } from "react"

interface AnimatedListProps {
  children: ReactNode
  className?: string
  /** 每个子元素之间的延迟时间（秒） */
  staggerDelay?: number
  /** 是否启用动画 */
  animate?: boolean
}

/**
 * 列表错开入场动画组件
 * 子元素会像水波一样依次展开
 */
export function AnimatedList({
  children,
  className,
  staggerDelay = 0.03,
  animate = true,
}: AnimatedListProps) {
  if (!animate) {
    return <div className={className}>{children}</div>
  }

  const container: Variants = {
    hidden: { opacity: 0 },
    show: {
      opacity: 1,
      transition: {
        staggerChildren: staggerDelay,
      },
    },
  }

  const item: Variants = {
    hidden: { opacity: 0, y: 12 },
    show: {
      opacity: 1,
      y: 0,
      transition: {
        type: "spring" as const,
        stiffness: 400,
        damping: 30,
      },
    },
  }

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
  )
}
