

import { useEffect, useRef, useState } from "react"
import { motion, AnimatePresence } from "motion/react"
import { useLocation } from "react-router-dom"

/**
 * 顶部加载进度条
 * 路由切换时显示，模拟 NProgress 效果
 *
 * 支持两种触发模式：
 * 1. 路由切换（pathname 变化）: 自动触发
 * 2. Suspense fallback: 作为 <Suspense fallback={<TopLoaderFallback />}> 使用
 */
export function TopLoader() {
  const { pathname } = useLocation()
  const [loading, setLoading] = useState(false)
  const [progress, setProgress] = useState(0)
  const prevPath = useRef(pathname)

  useEffect(() => {
    // 首次挂载不触发（避免初始渲染闪一下）
    if (prevPath.current === pathname) return
    prevPath.current = pathname

    setLoading(true)
    setProgress(0)

    // 渐进式进度模拟 — 快启动、慢到 80%、然后等待完成
    const timer1 = setTimeout(() => setProgress(30), 50)
    const timer2 = setTimeout(() => setProgress(55), 150)
    const timer3 = setTimeout(() => setProgress(75), 300)
    const timer4 = setTimeout(() => setProgress(90), 500)
    const timer5 = setTimeout(() => {
      setProgress(100)
      setTimeout(() => setLoading(false), 200)
    }, 600)

    return () => {
      clearTimeout(timer1)
      clearTimeout(timer2)
      clearTimeout(timer3)
      clearTimeout(timer4)
      clearTimeout(timer5)
    }
  }, [pathname])

  return <ProgressBar loading={loading} progress={progress} />
}

/**
 * Suspense fallback 组件
 * 用于 lazy() 组件首次加载时显示顶部进度条
 */
export function TopLoaderFallback() {
  const [progress, setProgress] = useState(10)

  useEffect(() => {
    // 持续缓慢增长，直到组件加载完成
    const interval = setInterval(() => {
      setProgress(prev => {
        if (prev >= 90) return prev
        // 越接近 90% 增长越慢
        const increment = Math.max(1, (90 - prev) * 0.1)
        return Math.min(90, prev + increment)
      })
    }, 200)

    return () => clearInterval(interval)
  }, [])

  return <ProgressBar loading={true} progress={progress} />
}

function ProgressBar({ loading, progress }: { loading: boolean; progress: number }) {
  return (
    <AnimatePresence>
      {loading && (
        <motion.div
          className="fixed top-0 left-0 right-0 z-[9999]"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0, transition: { duration: 0.3, delay: 0.1 } }}
        >
          {/* 进度条 */}
          <motion.div
            className="h-[2.5px] bg-primary shadow-[0_0_8px_hsl(var(--primary)/0.4)]"
            initial={{ width: "0%" }}
            animate={{ width: `${progress}%` }}
            transition={{ duration: 0.25, ease: "easeOut" }}
          />
        </motion.div>
      )}
    </AnimatePresence>
  )
}
