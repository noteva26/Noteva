"use client"

import { motion } from "motion/react"
import { ReactNode } from "react"
import { cn } from "@/lib/utils"

interface AnimatedCardProps {
  children: ReactNode
  className?: string
  /** 是否启用悬停动画 */
  hover?: boolean
  /** 是否启用点击动画 */
  tap?: boolean
}

/**
 * 带动画效果的卡片包装器
 * 悬停时微微上浮，点击时有按压感
 */
export function AnimatedCard({
  children,
  className,
  hover = true,
  tap = true,
}: AnimatedCardProps) {
  return (
    <motion.div
      whileHover={hover ? {
        y: -2,
        boxShadow: "0 8px 30px rgba(0,0,0,0.08)",
      } : undefined}
      whileTap={tap ? { scale: 0.995 } : undefined}
      transition={{
        type: "spring",
        stiffness: 400,
        damping: 25,
      }}
      className={cn("transition-colors", className)}
    >
      {children}
    </motion.div>
  )
}

/**
 * 带动画效果的按钮包装器
 * 悬停时微微放大，点击时有按压感
 */
export function AnimatedButton({
  children,
  className,
  disabled = false,
}: AnimatedCardProps & { disabled?: boolean }) {
  if (disabled) {
    return <div className={className}>{children}</div>
  }

  return (
    <motion.div
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      transition={{
        type: "spring",
        stiffness: 500,
        damping: 30,
      }}
      className={className}
    >
      {children}
    </motion.div>
  )
}

/**
 * 带动画效果的图标包装器
 * 悬停时旋转
 */
export function AnimatedIcon({
  children,
  className,
  rotate = 15,
}: {
  children: ReactNode
  className?: string
  rotate?: number
}) {
  return (
    <motion.div
      whileHover={{ rotate }}
      transition={{ type: "spring", stiffness: 400, damping: 20 }}
      className={className}
    >
      {children}
    </motion.div>
  )
}

/**
 * 带动画效果的链接包装器
 * 悬停时向右滑动
 */
export function AnimatedLink({
  children,
  className,
  offset = 4,
}: {
  children: ReactNode
  className?: string
  offset?: number
}) {
  return (
    <motion.div
      whileHover={{ x: offset }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
      className={className}
    >
      {children}
    </motion.div>
  )
}
