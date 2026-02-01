"use client"

import { motion } from "motion/react"
import { ReactNode } from "react"

interface PageTransitionProps {
  children: ReactNode
  className?: string
}

/**
 * 页面过渡动画组件
 * 使用 Spring 动画实现丝滑的页面切换效果
 */
export function PageTransition({ children, className }: PageTransitionProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{
        type: "spring",
        stiffness: 400,
        damping: 30,
      }}
      className={className}
    >
      {children}
    </motion.div>
  )
}

/**
 * 淡入动画组件
 */
export function FadeIn({ 
  children, 
  className,
  delay = 0 
}: PageTransitionProps & { delay?: number }) {
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{
        duration: 0.4,
        delay,
        ease: "easeOut",
      }}
      className={className}
    >
      {children}
    </motion.div>
  )
}
