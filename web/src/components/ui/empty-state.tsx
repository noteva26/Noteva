"use client"

import { motion } from "motion/react"
import { FileText, LucideIcon } from "lucide-react"
import { cn } from "@/lib/utils"
import { Button } from "./button"

interface EmptyStateProps {
  title?: string
  description?: string
  icon?: LucideIcon
  actionText?: string
  onAction?: () => void
  className?: string
  size?: "sm" | "md" | "lg"
}

/**
 * 空状态组件
 * 用于显示无数据、无内容等空状态
 */
export function EmptyState({
  title,
  description = "暂无数据",
  icon: Icon = FileText,
  actionText,
  onAction,
  className,
  size = "md",
}: EmptyStateProps) {
  const sizes = {
    sm: { container: "py-8", icon: "size-10", iconInner: "size-4" },
    md: { container: "py-12", icon: "size-12", iconInner: "size-5" },
    lg: { container: "py-16", icon: "size-16", iconInner: "size-6" },
  }

  const s = sizes[size]

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{
        type: "spring",
        stiffness: 400,
        damping: 30,
      }}
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
        <Icon className={cn("text-muted-foreground", s.iconInner)} />
      </div>
      {title && <h3 className="text-sm font-medium mb-1">{title}</h3>}
      <p className="text-xs text-muted-foreground max-w-md">{description}</p>
      {onAction && actionText && (
        <Button onClick={onAction} variant="outline" size="sm" className="mt-4">
          {actionText}
        </Button>
      )}
    </motion.div>
  )
}

/**
 * 带边框的空状态
 */
export function EmptyStateWithBorder(props: EmptyStateProps) {
  return (
    <div className="border border-dashed rounded-lg">
      <EmptyState {...props} />
    </div>
  )
}

/**
 * 内联空状态
 */
export function EmptyInline({
  message = "暂无数据",
  icon: Icon = FileText,
  className,
}: {
  message?: string
  icon?: LucideIcon
  className?: string
}) {
  return (
    <div
      className={cn(
        "flex items-center justify-center gap-2 py-8 text-sm text-muted-foreground",
        className
      )}
    >
      <Icon className="size-4" />
      <span>{message}</span>
    </div>
  )
}
