import { useEffect, useRef, useState } from "react";
import { motion, AnimatePresence } from "motion/react";
import { useLocation } from "react-router-dom";

export function TopLoader() {
  const { pathname } = useLocation();
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState(0);
  const prevPath = useRef(pathname);

  useEffect(() => {
    if (prevPath.current === pathname) return;
    prevPath.current = pathname;

    setLoading(true);
    setProgress(0);

    const timers: number[] = [];
    const schedule = (callback: () => void, delay: number) => {
      const timer = window.setTimeout(callback, delay);
      timers.push(timer);
    };

    schedule(() => setProgress(30), 50);
    schedule(() => setProgress(55), 150);
    schedule(() => setProgress(75), 300);
    schedule(() => setProgress(90), 500);
    schedule(() => {
      setProgress(100);
      schedule(() => setLoading(false), 200);
    }, 600);

    return () => {
      timers.forEach((timer) => window.clearTimeout(timer));
    };
  }, [pathname]);

  return <ProgressBar loading={loading} progress={progress} />;
}

export function TopLoaderFallback() {
  const [progress, setProgress] = useState(10);

  useEffect(() => {
    const interval = window.setInterval(() => {
      setProgress((current) => {
        if (current >= 90) return current;
        const increment = Math.max(1, (90 - current) * 0.1);
        return Math.min(90, current + increment);
      });
    }, 200);

    return () => window.clearInterval(interval);
  }, []);

  return <ProgressBar loading={true} progress={progress} />;
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
          <motion.div
            className="h-[2.5px] bg-primary shadow-[0_0_8px_hsl(var(--primary)/0.4)]"
            initial={{ width: "0%" }}
            animate={{ width: `${progress}%` }}
            transition={{ duration: 0.25, ease: "easeOut" }}
          />
        </motion.div>
      )}
    </AnimatePresence>
  );
}
