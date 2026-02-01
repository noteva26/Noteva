"use client"

import { motion } from "motion/react"
import { Loader2, LucideIcon } from "lucide-react"
import { cn } from "@/lib/utils"

interface LoadingStateProps {
  title?: string
  description?: string
  icon?: LucideIcon
  className?: string
  size?: "sm" | "md" | "lg"
}

/**
 * 加载状态组件
 * 带有优雅的入场动画
 */
export function LoadingState({
  title,
  description,
  icon: Icon = Loader2,
  className,
  size = "md",
}: LoadingStateProps) {
  const sizes = {
    sm: { container: "py-8", icon: "size-8", iconInner: "size-4" },
    md: { container: "py-12", icon: "size-12", iconInner: "size-5" },
    lg: { container: "py-16", icon: "size-16", iconInner: "size-6" },
  }

  const s = sizes[size]

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.3 }}
      className={cn(
        "flex flex-col items-center justify-center text-center",
        s.container,
        className
      )}
    >
      <div
        className={cn(
          "rounded-full bg-muted flex items-center justify-center mb-4",
          s.icon
        )}
      >
        <Icon className={cn("text-muted-foreground animate-spin", s.iconInner)} />
      </div>
      {title && (
        <h3 className="text-sm font-medium mb-1 animate-pulse">{title}</h3>
      )}
      {description && (
        <p className="text-xs text-muted-foreground animate-pulse">
          {description}
        </p>
      )}
    </motion.div>
  )
}

/**
 * 带边框的加载状态
 */
export function LoadingStateWithBorder(props: LoadingStateProps) {
  return (
    <div className="border border-dashed rounded-lg">
      <LoadingState {...props} />
    </div>
  )
}

/**
 * 内联加载指示器
 */
export function LoadingInline({ className }: { className?: string }) {
  return (
    <div className={cn("flex items-center gap-2 text-muted-foreground", className)}>
      <Loader2 className="size-4 animate-spin" />
      <span className="text-sm">加载中...</span>
    </div>
  )
}
